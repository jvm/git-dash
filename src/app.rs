use std::path::PathBuf;
use std::sync::mpsc::Sender;

use ratatui::widgets::TableState;

use crate::discovery::RepoRef;
use crate::status::RepoState;
use crate::worker::{Action, WorkerCmd};

#[derive(Clone, Copy, PartialEq)]
pub enum SortOrder {
    Name,
    Status,
    AheadBehind,
    LastFetch,
}

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
    pub help_visible: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub sort_order: SortOrder,
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
            help_visible: false,
            search_mode: false,
            search_query: String::new(),
            sort_order: SortOrder::Name,
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
        let len = self.filtered_repos().len();
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
        let len = self.filtered_repos().len();
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
        let len = self.filtered_repos().len();
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
        let len = self.filtered_repos().len();
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn jump_to_first(&mut self) {
        if !self.filtered_repos().is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn jump_to_last(&mut self) {
        let len = self.filtered_repos().len();
        if len > 0 {
            self.table_state.select(Some(len - 1));
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoState> {
        let filtered = self.filtered_repos();
        self.table_state.selected().and_then(|i| {
            filtered
                .get(i)
                .and_then(|r| self.repos.iter().find(|repo| repo.path == r.path))
        })
    }

    pub fn set_status(&mut self, status: String) {
        self.status_line = status;
    }

    pub fn sort_repos(&mut self) {
        match self.sort_order {
            SortOrder::Name => {
                self.repos.sort_by(|a, b| a.name.cmp(&b.name));
            }
            SortOrder::Status => {
                // Dirty repos first, then by name
                self.repos
                    .sort_by(|a, b| b.dirty.cmp(&a.dirty).then_with(|| a.name.cmp(&b.name)));
            }
            SortOrder::AheadBehind => {
                // Repos with changes first (ahead or behind), then by name
                self.repos.sort_by(|a, b| {
                    let a_has_changes = a.ahead_behind != "-";
                    let b_has_changes = b.ahead_behind != "-";
                    b_has_changes
                        .cmp(&a_has_changes)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
            SortOrder::LastFetch => {
                // Most recently fetched first, then by name
                self.repos.sort_by(|a, b| {
                    a.last_fetch
                        .cmp(&b.last_fetch)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
        }

        if self.repos.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    pub fn cycle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Name => SortOrder::Status,
            SortOrder::Status => SortOrder::AheadBehind,
            SortOrder::AheadBehind => SortOrder::LastFetch,
            SortOrder::LastFetch => SortOrder::Name,
        };
        self.sort_repos();
        let sort_name = match self.sort_order {
            SortOrder::Name => "Name",
            SortOrder::Status => "Status (dirty first)",
            SortOrder::AheadBehind => "Ahead/Behind",
            SortOrder::LastFetch => "Last Fetch",
        };
        self.set_status(format!("Sorted by: {}", sort_name));
    }

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    pub fn filtered_repos(&self) -> Vec<RepoState> {
        if self.search_query.is_empty() {
            self.repos.clone()
        } else {
            let query_lower = self.search_query.to_lowercase();
            self.repos
                .iter()
                .filter(|repo| repo.name.to_lowercase().contains(&query_lower))
                .cloned()
                .collect()
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        // Reset selection to first repo
        if !self.repos.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        // Reset selection when search changes
        self.table_state.select(Some(0));
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        // Reset selection when search changes
        if !self.repos.is_empty() {
            self.table_state.select(Some(0));
        }
    }
}
