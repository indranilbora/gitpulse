use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardSnapshot {
    pub generated_at_epoch_secs: i64,
    pub overview: OverviewMetrics,
    pub alerts: Vec<DashboardAlert>,
    pub repos: Vec<RepoRow>,
    pub worktrees: Vec<WorktreeRow>,
    pub processes: Vec<RepoProcess>,
    pub dependencies: Vec<DependencyHealth>,
    pub env_audit: Vec<EnvAuditResult>,
    pub mcp_servers: Vec<McpServerHealth>,
    pub providers: Vec<ProviderUsage>,
}

impl DashboardSnapshot {
    pub fn total_estimated_cost_usd(&self) -> f64 {
        self.providers
            .iter()
            .map(|p| p.estimated_cost_usd)
            .sum::<f64>()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DashboardSection {
    Home,
    Repos,
    Worktrees,
    Processes,
    Dependencies,
    EnvAudit,
    McpHealth,
    AiCosts,
}

impl DashboardSection {
    pub fn all() -> [DashboardSection; 8] {
        [
            DashboardSection::Home,
            DashboardSection::Repos,
            DashboardSection::Worktrees,
            DashboardSection::Processes,
            DashboardSection::Dependencies,
            DashboardSection::EnvAudit,
            DashboardSection::McpHealth,
            DashboardSection::AiCosts,
        ]
    }

    pub fn category(self) -> &'static str {
        match self {
            DashboardSection::Home => "OVERVIEW",
            DashboardSection::Repos | DashboardSection::Worktrees => "WORKSPACE",
            DashboardSection::Processes
            | DashboardSection::Dependencies
            | DashboardSection::EnvAudit => "MONITOR",
            DashboardSection::McpHealth | DashboardSection::AiCosts => "INTEGRATIONS",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            DashboardSection::Home => "Home",
            DashboardSection::Repos => "Repos",
            DashboardSection::Worktrees => "Worktrees",
            DashboardSection::Processes => "Processes",
            DashboardSection::Dependencies => "Deps",
            DashboardSection::EnvAudit => "Env Audit",
            DashboardSection::McpHealth => "MCP Health",
            DashboardSection::AiCosts => "AI Costs",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverviewMetrics {
    pub total_repos: usize,
    pub actionable_repos: usize,
    pub dirty_repos: usize,
    pub repos_ahead: usize,
    pub repos_behind: usize,
    pub total_worktrees: usize,
    pub repo_processes: usize,
    pub env_issues: usize,
    pub dep_issues: usize,
    pub mcp_unhealthy: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCommand {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub repo: Option<String>,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRow {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub dirty: usize,
    pub ahead: usize,
    pub behind: usize,
    pub stash: usize,
    pub recommendation: String,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeRow {
    pub repo: String,
    pub path: String,
    pub branch: String,
    pub detached: bool,
    pub bare: bool,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoProcess {
    pub repo: String,
    pub pid: i32,
    pub elapsed: String,
    pub command: String,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    pub repo: String,
    pub path: String,
    pub ecosystems: Vec<String>,
    pub issue_count: usize,
    pub issues: Vec<String>,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvAuditResult {
    pub repo: String,
    pub path: String,
    pub env_files: Vec<String>,
    pub missing_keys: Vec<String>,
    pub extra_keys: Vec<String>,
    pub tracked_secret_files: Vec<String>,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerHealth {
    pub source: String,
    pub server_name: String,
    pub command: String,
    pub healthy: bool,
    pub detail: String,
    pub action: Option<ActionCommand>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Gemini,
    OpenAi,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Gemini => "gemini",
            ProviderKind::OpenAi => "openai",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub provider: ProviderKind,
    pub configured: bool,
    pub config_sources: Vec<String>,
    pub sessions: usize,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub estimated_cost_usd: f64,
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_categories() {
        assert_eq!(DashboardSection::Home.category(), "OVERVIEW");
        assert_eq!(DashboardSection::Repos.category(), "WORKSPACE");
        assert_eq!(DashboardSection::Worktrees.category(), "WORKSPACE");
        assert_eq!(DashboardSection::Processes.category(), "MONITOR");
        assert_eq!(DashboardSection::Dependencies.category(), "MONITOR");
        assert_eq!(DashboardSection::EnvAudit.category(), "MONITOR");
        assert_eq!(DashboardSection::McpHealth.category(), "INTEGRATIONS");
        assert_eq!(DashboardSection::AiCosts.category(), "INTEGRATIONS");
    }

    #[test]
    fn total_cost_rolls_up() {
        let mut s = DashboardSnapshot::default();
        s.providers.push(ProviderUsage {
            provider: ProviderKind::Claude,
            configured: true,
            config_sources: vec![],
            sessions: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            estimated_cost_usd: 12.5,
            notes: vec![],
        });
        s.providers.push(ProviderUsage {
            provider: ProviderKind::OpenAi,
            configured: true,
            config_sources: vec![],
            sessions: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            estimated_cost_usd: 7.5,
            notes: vec![],
        });
        assert_eq!(s.total_estimated_cost_usd(), 20.0);
    }
}
