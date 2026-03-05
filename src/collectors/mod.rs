use crate::dashboard::{
    DashboardAlert, DependencyHealth, EnvAuditResult, McpServerHealth, ProviderUsage, RepoProcess,
    RepoRow, WorktreeRow,
};
use crate::git::Repo;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub mod ai_mcp;
pub mod git_worktrees;
pub mod system_env_deps;

pub use ai_mcp::{collect_mcp_servers, collect_provider_usage};
pub use git_worktrees::{collect_git_alerts, collect_repo_rows, collect_worktrees};
pub use system_env_deps::{collect_dependency_health, collect_env_audit, collect_repo_processes};

#[derive(Debug, Clone, Default)]
pub struct CollectorOutput {
    pub alerts: Vec<DashboardAlert>,
    pub repos: Vec<RepoRow>,
    pub worktrees: Vec<WorktreeRow>,
    pub processes: Vec<RepoProcess>,
    pub dependencies: Vec<DependencyHealth>,
    pub env_audit: Vec<EnvAuditResult>,
    pub mcp_servers: Vec<McpServerHealth>,
    pub providers: Vec<ProviderUsage>,
}

#[derive(Clone)]
struct ProviderSnapshotCacheEntry {
    generated_at: Instant,
    providers: Vec<ProviderUsage>,
}

static PROVIDER_SNAPSHOT_CACHE: OnceLock<Mutex<Option<ProviderSnapshotCacheEntry>>> =
    OnceLock::new();

pub fn collect_all(repos: &[Repo]) -> CollectorOutput {
    let repo_rows = collect_repo_rows(repos);
    let worktrees = collect_worktrees(repos);

    CollectorOutput {
        alerts: collect_git_alerts(repos, &repo_rows, &worktrees),
        repos: repo_rows,
        worktrees,
        processes: collect_repo_processes(repos),
        dependencies: collect_dependency_health(repos),
        env_audit: collect_env_audit(repos),
        mcp_servers: collect_mcp_servers(repos),
        providers: collect_provider_usage_cadenced(),
    }
}

fn collect_provider_usage_cadenced() -> Vec<ProviderUsage> {
    let refresh_secs = std::env::var("AGENTPULSE_PROVIDER_REFRESH_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(45);
    let refresh_after = Duration::from_secs(refresh_secs);

    let cache = PROVIDER_SNAPSHOT_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.generated_at.elapsed() < refresh_after {
                return entry.providers.clone();
            }
        }
    }

    let providers = collect_provider_usage();

    if let Ok(mut guard) = cache.lock() {
        *guard = Some(ProviderSnapshotCacheEntry {
            generated_at: Instant::now(),
            providers: providers.clone(),
        });
    }

    providers
}
