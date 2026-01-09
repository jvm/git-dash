use std::path::PathBuf;
use std::sync::mpsc::Sender;

use ratatui::widgets::TableState;

use crate::discovery::RepoRef;
use crate::status::RepoState;
use crate::worker::{Action, WorkerCmd};

pub struct App {
    pub root: PathBuf,
    pub repos: Vec<RepoState>,
    pub table_state: TableState,
    pub cmd_tx: Sender<WorkerCmd>,
    pub status_line: String,
    pub loading: bool,
    pub scan_progress: f64,
    pub confirmation: Option<Action>,
    pub should_quit: bool,
}

impl App {
    pub fn new(root: PathBuf, cmd_tx: Sender<WorkerCmd>) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            root,
            repos: Vec::new(),
            table_state,
            cmd_tx,
            status_line: "Ready".to_string(),
            loading: false,
            scan_progress: 0.0,
            confirmation: None,
            should_quit: false,
        }
    }

    pub fn request_scan(&mut self) {
        self.loading = true;
        self.scan_progress = 0.0;
        let _ = self.cmd_tx.send(WorkerCmd::Scan {
            root: self.root.clone(),
        });
    }

    pub fn request_refresh(&mut self) {
        let repos = self
            .repos
            .iter()
            .map(|repo| RepoRef {
                path: repo.path.clone(),
                git_dir: repo.git_dir.clone(),
            })
            .collect();
        let _ = self.cmd_tx.send(WorkerCmd::Refresh { repos });
    }

    pub fn request_confirm(&mut self, action: Action) {
        if self.repos.is_empty() {
            self.set_status("No repositories selected".to_string());
            return;
        }

        // Validate that we have a remote before allowing push/pull
        if let Some(repo) = self.selected_repo() {
            if repo.remote_url == "-" {
                self.set_status("No remote configured for this repository".to_string());
                return;
            }
        }

        self.confirmation = Some(action);
    }

    pub fn perform_action(&mut self, action: Action) {
        if let Some(repo) = self.selected_repo() {
            let _ = self.cmd_tx.send(WorkerCmd::Action {
                path: repo.path.clone(),
                action,
            });
            self.set_status("Running action...".to_string());
        }
    }

    pub fn request_quit(&mut self) {
        let _ = self.cmd_tx.send(WorkerCmd::Quit);
    }

    pub fn next(&mut self) {
        let len = self.repos.len();
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let len = self.repos.len();
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn page_down(&mut self) {
        let len = self.repos.len();
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 10).min(len - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn page_up(&mut self) {
        let len = self.repos.len();
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn selected_repo(&self) -> Option<&RepoState> {
        self.table_state.selected().and_then(|i| self.repos.get(i))
    }

    pub fn set_status(&mut self, status: String) {
        self.status_line = status;
    }

    pub fn sort_repos(&mut self) {
        self.repos.sort_by(|a, b| a.name.cmp(&b.name));
        if self.repos.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }
}
