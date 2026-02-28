use crate::agent;
use crate::config::Config;
use crate::dashboard::{ActionCommand, DashboardSection, DashboardSnapshot};
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
    /// Show only repos with non-idle recommendations (toggled with `A`).
    pub agent_focus_mode: bool,
    /// Currently focused dashboard section.
    pub section: DashboardSection,
    /// Latest collected dashboard snapshot (repos + processes + deps + env + MCP + AI).
    pub dashboard: DashboardSnapshot,
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
            agent_focus_mode: false,
            section: DashboardSection::Home,
            dashboard: DashboardSnapshot::default(),
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
            .filter(|r| !self.agent_focus_mode || agent::needs_attention(r))
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

    pub fn active_row_count(&self) -> usize {
        match self.section {
            DashboardSection::Home => self.dashboard.alerts.len(),
            DashboardSection::Repos => self.filtered_repos().len(),
            DashboardSection::Worktrees => self.dashboard.worktrees.len(),
            DashboardSection::Processes => self.dashboard.processes.len(),
            DashboardSection::Dependencies => self.dashboard.dependencies.len(),
            DashboardSection::EnvAudit => self.dashboard.env_audit.len(),
            DashboardSection::McpHealth => self.dashboard.mcp_servers.len(),
            DashboardSection::AiCosts => self.dashboard.providers.len(),
        }
    }

    /// Item count for a specific section (used by sidebar badges).
    pub fn section_row_count(&self, section: DashboardSection) -> usize {
        match section {
            DashboardSection::Home => self.dashboard.alerts.len(),
            DashboardSection::Repos => self.filtered_repos().len(),
            DashboardSection::Worktrees => self.dashboard.worktrees.len(),
            DashboardSection::Processes => self.dashboard.processes.len(),
            DashboardSection::Dependencies => self.dashboard.dependencies.len(),
            DashboardSection::EnvAudit => self.dashboard.env_audit.len(),
            DashboardSection::McpHealth => self.dashboard.mcp_servers.len(),
            DashboardSection::AiCosts => self.dashboard.providers.len(),
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.active_row_count();
        if len == 0 {
            return;
        }
        self.selected = (self.selected as i32 + delta).rem_euclid(len as i32) as usize;
    }

    pub fn clamp_selection(&mut self) {
        let len = self.active_row_count();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    pub fn selected_repo(&self) -> Option<&Repo> {
        self.filtered_repos().into_iter().nth(self.selected)
    }

    pub fn selected_action(&self) -> Option<ActionCommand> {
        match self.section {
            DashboardSection::Home => self
                .dashboard
                .alerts
                .get(self.selected)
                .and_then(|a| a.action.clone()),
            DashboardSection::Repos => self.selected_repo().and_then(|repo| {
                let rec = agent::recommend(repo);
                if rec.short_action == "noop" {
                    None
                } else {
                    Some(ActionCommand {
                        label: rec.action.to_string(),
                        command: rec.command,
                    })
                }
            }),
            DashboardSection::Worktrees => self
                .dashboard
                .worktrees
                .get(self.selected)
                .and_then(|r| r.action.clone()),
            DashboardSection::Processes => self
                .dashboard
                .processes
                .get(self.selected)
                .and_then(|r| r.action.clone()),
            DashboardSection::Dependencies => self
                .dashboard
                .dependencies
                .get(self.selected)
                .and_then(|r| r.action.clone()),
            DashboardSection::EnvAudit => self
                .dashboard
                .env_audit
                .get(self.selected)
                .and_then(|r| r.action.clone()),
            DashboardSection::McpHealth => self
                .dashboard
                .mcp_servers
                .get(self.selected)
                .and_then(|r| r.action.clone()),
            DashboardSection::AiCosts => None,
        }
    }

    pub fn next_section(&mut self) {
        let all = DashboardSection::all();
        let idx = all
            .iter()
            .position(|s| *s == self.section)
            .unwrap_or(0)
            .saturating_add(1)
            % all.len();
        self.section = all[idx];
        self.selected = 0;
        self.clamp_selection();
    }

    pub fn previous_section(&mut self) {
        let all = DashboardSection::all();
        let idx = all.iter().position(|s| *s == self.section).unwrap_or(0);
        let next_idx = if idx == 0 { all.len() - 1 } else { idx - 1 };
        self.section = all[next_idx];
        self.selected = 0;
        self.clamp_selection();
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
