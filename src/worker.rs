use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::discovery::{discover_repos_with_progress, RepoRef};
use crate::git::{git_pull, git_push};
use crate::logger::log_debug;
use crate::status::{error_repo_state, git_status, RepoState};

#[derive(Clone, Copy)]
pub enum Action {
    Pull,
    Push,
}

pub enum WorkerCmd {
    Scan { root: PathBuf },
    Refresh { repos: Vec<RepoRef> },
    Action { path: PathBuf, action: Action },
    Quit,
}

pub enum WorkerEvent {
    ScanComplete(Vec<RepoState>),
    RefreshComplete(Vec<RepoState>),
    ScanProgress {
        ratio: f64,
    },
    ActionResult {
        path: PathBuf,
        action: Action,
        result: Result<String, String>,
    },
}

// Progress is split into discovery (40%) and status (60%) phases.
const DISCOVERY_PROGRESS_WEIGHT: f64 = 0.4;
const STATUS_PROGRESS_WEIGHT: f64 = 0.6;

pub fn spawn_worker(
    cmd_rx: Receiver<WorkerCmd>,
    evt_tx: Sender<WorkerEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        'worker_loop: while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                WorkerCmd::Scan { root } => {
                    log_debug(&format!("Scan start root={}", root.display()));
                    let scan_start = Instant::now();
                    let mut total_estimate = 0usize;
                    let stop = Arc::new(AtomicBool::new(false));
                    let stop_flag = Arc::clone(&stop);
                    let repos = discover_repos_with_progress(&root, |visited, remaining| {
                        if stop_flag.load(Ordering::Relaxed) {
                            return false;
                        }
                        total_estimate = total_estimate.max(visited + remaining);
                        if total_estimate == 0 {
                            return true;
                        }
                        let ratio = visited as f64 / total_estimate as f64;
                        let scaled =
                            (ratio * DISCOVERY_PROGRESS_WEIGHT).min(DISCOVERY_PROGRESS_WEIGHT);
                        if evt_tx
                            .send(WorkerEvent::ScanProgress { ratio: scaled })
                            .is_err()
                        {
                            stop_flag.store(true, Ordering::Relaxed);
                            return false;
                        }
                        true
                    });
                    if stop.load(Ordering::Relaxed) {
                        break 'worker_loop;
                    }
                    log_debug(&format!(
                        "Discovery complete repos={} elapsed_ms={}",
                        repos.len(),
                        scan_start.elapsed().as_millis()
                    ));

                    // Parallelize status fetching
                    let (states, channel_closed) = fetch_status_parallel(repos, &evt_tx);
                    if channel_closed {
                        break 'worker_loop;
                    }

                    if evt_tx.send(WorkerEvent::ScanComplete(states)).is_err() {
                        break 'worker_loop;
                    }
                    log_debug(&format!(
                        "Scan complete elapsed_ms={}",
                        scan_start.elapsed().as_millis()
                    ));
                }
                WorkerCmd::Refresh { repos } => {
                    // Parallelize refresh as well
                    let (refreshed, channel_closed) = fetch_status_parallel(repos, &evt_tx);
                    if channel_closed {
                        break 'worker_loop;
                    }
                    if evt_tx
                        .send(WorkerEvent::RefreshComplete(refreshed))
                        .is_err()
                    {
                        break 'worker_loop;
                    }
                }
                WorkerCmd::Action { path, action } => {
                    let result = match action {
                        Action::Pull => git_pull(&path),
                        Action::Push => git_push(&path),
                    };
                    if evt_tx
                        .send(WorkerEvent::ActionResult {
                            path,
                            action,
                            result,
                        })
                        .is_err()
                    {
                        break 'worker_loop;
                    }
                }
                WorkerCmd::Quit => break 'worker_loop,
            }
        }
    })
}

fn fetch_status_parallel(
    repos: Vec<RepoRef>,
    evt_tx: &Sender<WorkerEvent>,
) -> (Vec<RepoState>, bool) {
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};

    let total_repos = repos.len().max(1);
    let states = Arc::new(Mutex::new(Vec::with_capacity(repos.len())));
    let completed = Arc::new(Mutex::new(0usize));
    let stop = Arc::new(AtomicBool::new(false));

    // Determine worker count: use available parallelism, cap at 16 to avoid overwhelming the system
    let worker_count = thread::available_parallelism()
        .map(|n| n.get().min(16))
        .unwrap_or(4);

    log_debug(&format!(
        "Fetching status for {} repos using {} workers",
        repos.len(),
        worker_count
    ));

    // Use scoped threads to avoid 'static lifetime requirements
    thread::scope(|scope| {
        // Create work queue channel
        let (work_tx, work_rx) = channel();
        let work_rx = Arc::new(Mutex::new(work_rx));

        // Send all work items to the queue
        for (idx, repo) in repos.into_iter().enumerate() {
            let _ = work_tx.send((idx, repo));
        }
        drop(work_tx); // Close the channel after sending all work

        // Spawn worker threads
        let mut handles = Vec::new();
        for _ in 0..worker_count {
            let work_rx = Arc::clone(&work_rx);
            let states = Arc::clone(&states);
            let completed = Arc::clone(&completed);
            let stop = Arc::clone(&stop);
            let evt_tx = evt_tx.clone();

            let handle = scope.spawn(move || {
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    // Get next work item
                    let work_item = {
                        let rx = work_rx.lock().unwrap();
                        rx.recv()
                    };

                    let (idx, repo) = match work_item {
                        Ok(item) => item,
                        Err(_) => break, // Channel closed, no more work
                    };
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }

                    let status_start = Instant::now();
                    let state = match git_status(&repo.path, &repo.git_dir) {
                        Ok(status) => {
                            log_debug(&format!(
                                "Status OK repo={} elapsed_ms={}",
                                repo.path.display(),
                                status_start.elapsed().as_millis()
                            ));
                            status
                        }
                        Err(err) => {
                            log_debug(&format!(
                                "Status ERR repo={} elapsed_ms={} error={}",
                                repo.path.display(),
                                status_start.elapsed().as_millis(),
                                err
                            ));
                            error_repo_state(&repo, &err)
                        }
                    };

                    states.lock().unwrap().push((idx, state));

                    let count = {
                        let mut c = completed.lock().unwrap();
                        *c += 1;
                        *c
                    };

                    let ratio = DISCOVERY_PROGRESS_WEIGHT
                        + count as f64 / total_repos as f64 * STATUS_PROGRESS_WEIGHT;
                    if evt_tx.send(WorkerEvent::ScanProgress { ratio }).is_err() {
                        stop.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }
    });

    // Sort by original index to maintain order
    let mut results = Arc::try_unwrap(states).unwrap().into_inner().unwrap();
    results.sort_by_key(|(idx, _)| *idx);
    let channel_closed = stop.load(Ordering::Relaxed);
    (
        results.into_iter().map(|(_, state)| state).collect(),
        channel_closed,
    )
}
