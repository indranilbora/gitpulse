pub mod builder;
pub mod models;

pub use builder::collect_and_build;
pub use models::{
    ActionCommand, ActionKind, DashboardAlert, DashboardSection, DashboardSnapshot,
    DependencyHealth, EnvAuditResult, McpServerHealth, ProviderKind, ProviderUsage, RepoProcess,
    RepoRow, WorktreeRow,
};
