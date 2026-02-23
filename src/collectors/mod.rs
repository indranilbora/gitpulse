use crate::dashboard::{
    DependencyHealth, EnvAuditResult, McpServerHealth, ProviderUsage, RepoProcess, WorktreeRow,
};
use crate::git::Repo;

pub mod ai_mcp;
pub mod git_worktrees;
pub mod system_env_deps;

pub use ai_mcp::{collect_mcp_servers, collect_provider_usage};
pub use git_worktrees::collect_worktrees;
pub use system_env_deps::{collect_dependency_health, collect_env_audit, collect_repo_processes};

#[derive(Debug, Clone, Default)]
pub struct CollectorOutput {
    pub worktrees: Vec<WorktreeRow>,
    pub processes: Vec<RepoProcess>,
    pub dependencies: Vec<DependencyHealth>,
    pub env_audit: Vec<EnvAuditResult>,
    pub mcp_servers: Vec<McpServerHealth>,
    pub providers: Vec<ProviderUsage>,
}

pub fn collect_all(repos: &[Repo]) -> CollectorOutput {
    CollectorOutput {
        worktrees: collect_worktrees(repos),
        processes: collect_repo_processes(repos),
        dependencies: collect_dependency_health(repos),
        env_audit: collect_env_audit(repos),
        mcp_servers: collect_mcp_servers(repos),
        providers: collect_provider_usage(),
    }
}
