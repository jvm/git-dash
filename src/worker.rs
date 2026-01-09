use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
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

pub fn spawn_worker(
    cmd_rx: Receiver<WorkerCmd>,
    evt_tx: Sender<WorkerEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                WorkerCmd::Scan { root } => {
                    log_debug(&format!("Scan start root={}", root.display()));
                    let scan_start = Instant::now();
                    let mut total_estimate = 0usize;
                    let repos = discover_repos_with_progress(&root, |visited, remaining| {
                        total_estimate = total_estimate.max(visited + remaining);
                        if total_estimate == 0 {
                            return;
                        }
                        let ratio = visited as f64 / total_estimate as f64;
                        let scaled = (ratio * 0.4).min(0.4);
                        let _ = evt_tx.send(WorkerEvent::ScanProgress { ratio: scaled });
                    });
                    log_debug(&format!(
                        "Discovery complete repos={} elapsed_ms={}",
                        repos.len(),
                        scan_start.elapsed().as_millis()
                    ));

                    // Parallelize status fetching
                    let states = fetch_status_parallel(repos, &evt_tx);

                    let _ = evt_tx.send(WorkerEvent::ScanComplete(states));
                    log_debug(&format!(
                        "Scan complete elapsed_ms={}",
                        scan_start.elapsed().as_millis()
                    ));
                }
                WorkerCmd::Refresh { repos } => {
                    // Parallelize refresh as well
                    let refreshed = fetch_status_parallel(repos, &evt_tx);
                    let _ = evt_tx.send(WorkerEvent::RefreshComplete(refreshed));
                }
                WorkerCmd::Action { path, action } => {
                    let result = match action {
                        Action::Pull => git_pull(&path),
                        Action::Push => git_push(&path),
                    };
                    let _ = evt_tx.send(WorkerEvent::ActionResult {
                        path,
                        action,
                        result,
                    });
                }
                WorkerCmd::Quit => break,
            }
        }
    })
}

fn fetch_status_parallel(repos: Vec<RepoRef>, evt_tx: &Sender<WorkerEvent>) -> Vec<RepoState> {
    use std::sync::{Arc, Mutex};

    let total_repos = repos.len().max(1);
    let states = Arc::new(Mutex::new(Vec::with_capacity(repos.len())));
    let completed = Arc::new(Mutex::new(0usize));

    // Use scoped threads to avoid 'static lifetime requirements
    thread::scope(|scope| {
        let mut handles = Vec::new();

        for (idx, repo) in repos.into_iter().enumerate() {
            let states = Arc::clone(&states);
            let completed = Arc::clone(&completed);
            let evt_tx = evt_tx.clone();

            let handle = scope.spawn(move || {
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

                let ratio = 0.4 + count as f64 / total_repos as f64 * 0.6;
                let _ = evt_tx.send(WorkerEvent::ScanProgress { ratio });
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
    results.into_iter().map(|(_, state)| state).collect()
}
