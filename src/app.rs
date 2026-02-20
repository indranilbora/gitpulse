use crate::config::Config;
use crate::git::Repo;
use chrono::{DateTime, Local};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Help,
    /// Typing a commit message; Enter commits, Esc cancels.
    Commit,
}

pub struct App {
    pub repos: Vec<Repo>,
    pub selected: usize,
    pub filter_text: String,
    pub commit_message: String,
    pub mode: AppMode,
    pub last_scan: Option<DateTime<Local>>,
    pub is_scanning: bool,
    pub config: Config,
    pub should_quit: bool,
    pub should_reconfigure: bool,
    /// Show repos grouped by parent directory (toggled with `g`).
    pub group_by_dir: bool,
    /// Transient status message (e.g. pull/push result). Clears after 4 s.
    pub notification: Option<(String, Instant)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            repos: Vec::new(),
            selected: 0,
            filter_text: String::new(),
            commit_message: String::new(),
            mode: AppMode::Normal,
            last_scan: None,
            is_scanning: true,
            config,
            should_quit: false,
            should_reconfigure: false,
            group_by_dir: false,
            notification: None,
        }
    }

    /// Returns repos matching the current filter and `show_clean` setting,
    /// sorted by (parent dir, urgency, name) when grouping is active.
    pub fn filtered_repos(&self) -> Vec<&Repo> {
        let mut repos: Vec<&Repo> = self
            .repos
            .iter()
            .filter(|r| self.config.show_clean || r.needs_attention())
            .filter(|r| {
                if self.filter_text.is_empty() {
                    return true;
                }
                let f = self.filter_text.to_lowercase();
                r.name.to_lowercase().contains(&f) || r.status.branch.to_lowercase().contains(&f)
            })
            .collect();

        if self.group_by_dir {
            repos.sort_by(|a, b| {
                let pa = a
                    .path
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let pb = b
                    .path
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                pa.cmp(&pb)
                    .then_with(|| b.urgency().cmp(&a.urgency()))
                    .then_with(|| a.name.cmp(&b.name))
            });
        }

        repos
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.filtered_repos().len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected as i32 + delta).rem_euclid(len as i32) as usize;
    }

    pub fn clamp_selection(&mut self) {
        let len = self.filtered_repos().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    pub fn selected_repo(&self) -> Option<&Repo> {
        self.filtered_repos().into_iter().nth(self.selected)
    }

    /// Set a transient notification message (shown in the status bar for 4 s).
    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notification = Some((msg.into(), Instant::now()));
    }

    /// Tick: clear expired notifications.
    pub fn tick(&mut self) {
        if let Some((_, t)) = &self.notification {
            if t.elapsed() > std::time::Duration::from_secs(4) {
                self.notification = None;
            }
        }
    }
}
