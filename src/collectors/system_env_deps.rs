use crate::dashboard::{DependencyHealth, EnvAuditResult, RepoProcess};
use crate::git::Repo;

pub fn collect_repo_processes(_repos: &[Repo]) -> Vec<RepoProcess> {
    Vec::new()
}

pub fn collect_dependency_health(_repos: &[Repo]) -> Vec<DependencyHealth> {
    Vec::new()
}

pub fn collect_env_audit(_repos: &[Repo]) -> Vec<EnvAuditResult> {
    Vec::new()
}
