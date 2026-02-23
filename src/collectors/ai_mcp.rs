use crate::dashboard::{McpServerHealth, ProviderUsage};
use crate::git::Repo;

pub fn collect_mcp_servers(_repos: &[Repo]) -> Vec<McpServerHealth> {
    Vec::new()
}

pub fn collect_provider_usage() -> Vec<ProviderUsage> {
    Vec::new()
}
