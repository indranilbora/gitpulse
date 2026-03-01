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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    GitStatus {
        repo_path: String,
    },
    GitFetch {
        repo_path: String,
    },
    GitPullRebase {
        repo_path: String,
    },
    GitPush {
        repo_path: String,
    },
    GitWorktreeList {
        repo_path: String,
    },
    GitAddCommitPullRebase {
        repo_path: String,
        message: String,
    },
    GitPullRebasePush {
        repo_path: String,
    },
    GitAddCommitPush {
        repo_path: String,
        message: String,
    },
    GitAddCommit {
        repo_path: String,
        message: String,
    },
    GitStashList {
        repo_path: String,
    },
    GitRemoteList {
        repo_path: String,
    },
    GitSwitchCreate {
        repo_path: String,
        branch: String,
    },
    KillProcess {
        pid: i32,
    },
    NpmInstallLockfile {
        repo_path: String,
    },
    CargoGenerateLockfile {
        repo_path: String,
    },
    UvLock {
        repo_path: String,
    },
    PipCompileRequirements {
        repo_path: String,
    },
    GoModTidy {
        repo_path: String,
    },
    BundleLock {
        repo_path: String,
    },
    IgnoreEnvFiles {
        repo_path: String,
        files: Vec<String>,
    },
    SeedEnvFromExample {
        repo_path: String,
    },
    ProbeBinaryHelp {
        binary: String,
    },
    CheckBinaryInPath {
        binary: String,
    },
    ShowMessage {
        message: String,
    },
}

impl ActionKind {
    pub fn preview(&self) -> String {
        match self {
            ActionKind::GitStatus { repo_path } => format!("git -C {:?} status -sb", repo_path),
            ActionKind::GitFetch { repo_path } => format!("git -C {:?} fetch --quiet", repo_path),
            ActionKind::GitPullRebase { repo_path } => {
                format!("git -C {:?} pull --rebase", repo_path)
            }
            ActionKind::GitPush { repo_path } => format!("git -C {:?} push", repo_path),
            ActionKind::GitWorktreeList { repo_path } => {
                format!("git -C {:?} worktree list", repo_path)
            }
            ActionKind::GitAddCommitPullRebase { repo_path, message } => format!(
                "git -C {:?} add -A && git -C {:?} commit -m {:?} && git -C {:?} pull --rebase",
                repo_path, repo_path, message, repo_path
            ),
            ActionKind::GitPullRebasePush { repo_path } => {
                format!(
                    "git -C {:?} pull --rebase && git -C {:?} push",
                    repo_path, repo_path
                )
            }
            ActionKind::GitAddCommitPush { repo_path, message } => format!(
                "git -C {:?} add -A && git -C {:?} commit -m {:?} && git -C {:?} push",
                repo_path, repo_path, message, repo_path
            ),
            ActionKind::GitAddCommit { repo_path, message } => format!(
                "git -C {:?} add -A && git -C {:?} commit -m {:?}",
                repo_path, repo_path, message
            ),
            ActionKind::GitStashList { repo_path } => {
                format!("git -C {:?} stash list", repo_path)
            }
            ActionKind::GitRemoteList { repo_path } => {
                format!("git -C {:?} remote -v", repo_path)
            }
            ActionKind::GitSwitchCreate { repo_path, branch } => {
                format!("git -C {:?} switch -c {:?}", repo_path, branch)
            }
            ActionKind::KillProcess { pid } => format!("kill {}", pid),
            ActionKind::NpmInstallLockfile { repo_path } => {
                format!("npm --prefix {:?} install --package-lock-only", repo_path)
            }
            ActionKind::CargoGenerateLockfile { repo_path } => {
                format!("cargo -C {:?} generate-lockfile", repo_path)
            }
            ActionKind::UvLock { repo_path } => format!("uv --directory {:?} lock", repo_path),
            ActionKind::PipCompileRequirements { repo_path } => {
                format!("pip-compile {:?}/requirements.txt", repo_path)
            }
            ActionKind::GoModTidy { repo_path } => format!("go -C {:?} mod tidy", repo_path),
            ActionKind::BundleLock { repo_path } => format!("bundle -C {:?} lock", repo_path),
            ActionKind::IgnoreEnvFiles { repo_path, files } => format!(
                "append .env* to {:?}/.gitignore and git rm --cached {}",
                repo_path,
                files.join(" ")
            ),
            ActionKind::SeedEnvFromExample { repo_path } => {
                format!("copy {:?}/.env.example -> {:?}/.env", repo_path, repo_path)
            }
            ActionKind::ProbeBinaryHelp { binary } => format!("{:?} --help", binary),
            ActionKind::CheckBinaryInPath { binary } => format!("which {:?}", binary),
            ActionKind::ShowMessage { message } => format!("echo {:?}", message),
        }
    }

    pub fn affected_repo_path(&self) -> Option<&str> {
        match self {
            ActionKind::GitStatus { repo_path }
            | ActionKind::GitFetch { repo_path }
            | ActionKind::GitPullRebase { repo_path }
            | ActionKind::GitPush { repo_path }
            | ActionKind::GitWorktreeList { repo_path }
            | ActionKind::GitAddCommitPullRebase { repo_path, .. }
            | ActionKind::GitPullRebasePush { repo_path }
            | ActionKind::GitAddCommitPush { repo_path, .. }
            | ActionKind::GitAddCommit { repo_path, .. }
            | ActionKind::GitStashList { repo_path }
            | ActionKind::GitRemoteList { repo_path }
            | ActionKind::GitSwitchCreate { repo_path, .. }
            | ActionKind::NpmInstallLockfile { repo_path }
            | ActionKind::CargoGenerateLockfile { repo_path }
            | ActionKind::UvLock { repo_path }
            | ActionKind::PipCompileRequirements { repo_path }
            | ActionKind::GoModTidy { repo_path }
            | ActionKind::BundleLock { repo_path }
            | ActionKind::IgnoreEnvFiles { repo_path, .. }
            | ActionKind::SeedEnvFromExample { repo_path } => Some(repo_path),
            ActionKind::KillProcess { .. }
            | ActionKind::ProbeBinaryHelp { .. }
            | ActionKind::CheckBinaryInPath { .. }
            | ActionKind::ShowMessage { .. } => None,
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            ActionKind::KillProcess { .. } | ActionKind::IgnoreEnvFiles { .. }
        )
    }

    pub fn risk_level(&self) -> &'static str {
        if self.is_destructive() {
            "high"
        } else {
            match self {
                ActionKind::GitAddCommitPullRebase { .. }
                | ActionKind::GitPullRebasePush { .. }
                | ActionKind::GitAddCommitPush { .. }
                | ActionKind::GitAddCommit { .. }
                | ActionKind::GitSwitchCreate { .. }
                | ActionKind::GitPullRebase { .. }
                | ActionKind::GitFetch { .. }
                | ActionKind::GitPush { .. } => "medium",
                _ => "low",
            }
        }
    }

    pub fn cancel_reassurance(&self) -> &'static str {
        if self.is_destructive() {
            "Cancel keeps your files and processes unchanged."
        } else {
            "Cancel does not run anything."
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCommand {
    pub label: String,
    /// Human-readable preview of what will run.
    pub command: String,
    pub action: ActionKind,
}

impl ActionCommand {
    pub fn new(label: impl Into<String>, action: ActionKind) -> Self {
        let command = action.preview();
        Self {
            label: label.into(),
            command,
            action,
        }
    }
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
    /// `live`, `local_logs`, `heuristic`, or `unconfigured`.
    pub data_source: String,
    /// Unix epoch seconds representing when the source data was last updated.
    pub source_updated_at_epoch_secs: i64,
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
            data_source: "live".to_string(),
            source_updated_at_epoch_secs: 0,
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
            data_source: "live".to_string(),
            source_updated_at_epoch_secs: 0,
            sessions: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            estimated_cost_usd: 7.5,
            notes: vec![],
        });
        assert_eq!(s.total_estimated_cost_usd(), 20.0);
    }

    #[test]
    fn action_command_preview_is_derived() {
        let action = ActionCommand::new(
            "pull",
            ActionKind::GitPullRebase {
                repo_path: "/tmp/repo".to_string(),
            },
        );
        assert_eq!(action.label, "pull");
        assert!(action.command.contains("pull --rebase"));
    }

    #[test]
    fn action_kind_serializes_with_type_tag() {
        let encoded = serde_json::to_string(&ActionKind::KillProcess { pid: 42 }).unwrap();
        assert!(encoded.contains("\"type\":\"kill_process\""));
        assert!(encoded.contains("\"pid\":42"));
    }

    #[test]
    fn destructive_actions_have_high_risk() {
        let kill = ActionKind::KillProcess { pid: 7 };
        assert!(kill.is_destructive());
        assert_eq!(kill.risk_level(), "high");
    }

    #[test]
    fn repo_path_extraction_works() {
        let action = ActionKind::GitPush {
            repo_path: "/tmp/repo".to_string(),
        };
        assert_eq!(action.affected_repo_path(), Some("/tmp/repo"));
    }
}
