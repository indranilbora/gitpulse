use crate::dashboard::WorktreeRow;
use crate::git::Repo;

pub fn collect_worktrees(_repos: &[Repo]) -> Vec<WorktreeRow> {
    Vec::new()
}
