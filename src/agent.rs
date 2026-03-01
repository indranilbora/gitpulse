use crate::dashboard::ActionKind;
use crate::git::Repo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionPriority {
    Critical,
    High,
    Medium,
    Low,
    Idle,
}

impl ActionPriority {
    pub fn rank(self) -> u8 {
        match self {
            ActionPriority::Critical => 4,
            ActionPriority::High => 3,
            ActionPriority::Medium => 2,
            ActionPriority::Low => 1,
            ActionPriority::Idle => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ActionPriority::Critical => "critical",
            ActionPriority::High => "high",
            ActionPriority::Medium => "medium",
            ActionPriority::Low => "low",
            ActionPriority::Idle => "idle",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub priority: ActionPriority,
    pub short_action: &'static str,
    pub action: &'static str,
    pub command: String,
    pub reason: String,
}

pub fn needs_attention(repo: &Repo) -> bool {
    recommend(repo).priority != ActionPriority::Idle
}

pub fn recommend(repo: &Repo) -> Recommendation {
    let path = repo.path.to_string_lossy();
    let cmd = |s: &str| format!("cd {:?} && {}", path, s);

    if repo.status.is_detached {
        return Recommendation {
            priority: ActionPriority::Critical,
            short_action: "reattach",
            action: "reattach HEAD to a branch",
            command: cmd("git switch -c rescue-work"),
            reason: "Repository is in detached HEAD state.".to_string(),
        };
    }

    if repo.status.behind_count > 0 && repo.status.uncommitted_count > 0 {
        return Recommendation {
            priority: ActionPriority::Critical,
            short_action: "commit+rebase",
            action: "commit/stash local work, then pull --rebase",
            command: cmd("git add -A && git commit -m \"wip\" && git pull --rebase"),
            reason: format!(
                "{} local changes + {} commits behind remote.",
                repo.status.uncommitted_count, repo.status.behind_count
            ),
        };
    }

    if repo.status.behind_count > 0 && repo.status.unpushed_count > 0 {
        return Recommendation {
            priority: ActionPriority::High,
            short_action: "rebase+push",
            action: "pull --rebase and then push",
            command: cmd("git pull --rebase && git push"),
            reason: format!(
                "{} ahead and {} behind remote (diverged).",
                repo.status.unpushed_count, repo.status.behind_count
            ),
        };
    }

    if repo.status.behind_count > 0 {
        return Recommendation {
            priority: ActionPriority::High,
            short_action: "pull",
            action: "pull latest changes",
            command: cmd("git pull --rebase"),
            reason: format!("{} commits behind remote.", repo.status.behind_count),
        };
    }

    if repo.status.uncommitted_count > 0 && repo.status.unpushed_count > 0 {
        return Recommendation {
            priority: ActionPriority::High,
            short_action: "commit+push",
            action: "commit local work and push",
            command: cmd("git add -A && git commit -m \"wip\" && git push"),
            reason: format!(
                "{} local changes + {} commits ahead.",
                repo.status.uncommitted_count, repo.status.unpushed_count
            ),
        };
    }

    if repo.status.uncommitted_count > 0 {
        return Recommendation {
            priority: ActionPriority::Medium,
            short_action: "commit",
            action: "commit local work",
            command: cmd("git add -A && git commit -m \"wip\""),
            reason: format!("{} uncommitted file(s).", repo.status.uncommitted_count),
        };
    }

    if repo.status.unpushed_count > 0 {
        return Recommendation {
            priority: ActionPriority::Medium,
            short_action: "push",
            action: "push local commits",
            command: cmd("git push"),
            reason: format!("{} commit(s) ahead of remote.", repo.status.unpushed_count),
        };
    }

    if repo.status.stash_count > 0 {
        return Recommendation {
            priority: ActionPriority::Low,
            short_action: "review stash",
            action: "review stashed work",
            command: cmd("git stash list"),
            reason: format!("{} stash entry(ies) present.", repo.status.stash_count),
        };
    }

    if !repo.status.has_remote {
        return Recommendation {
            priority: ActionPriority::Low,
            short_action: "set remote",
            action: "configure remote tracking",
            command: cmd("git remote -v"),
            reason: "No remote configured.".to_string(),
        };
    }

    Recommendation {
        priority: ActionPriority::Idle,
        short_action: "noop",
        action: "no action needed",
        command: cmd("git status -sb"),
        reason: "Working tree and remote state are clean.".to_string(),
    }
}

pub fn recommended_action_kind(repo: &Repo) -> Option<ActionKind> {
    let repo_path = repo.path.to_string_lossy().to_string();

    if repo.status.is_detached {
        return Some(ActionKind::GitSwitchCreate {
            repo_path,
            branch: "rescue-work".to_string(),
        });
    }

    if repo.status.behind_count > 0 && repo.status.uncommitted_count > 0 {
        return Some(ActionKind::GitAddCommitPullRebase {
            repo_path,
            message: "wip".to_string(),
        });
    }

    if repo.status.behind_count > 0 && repo.status.unpushed_count > 0 {
        return Some(ActionKind::GitPullRebasePush { repo_path });
    }

    if repo.status.behind_count > 0 {
        return Some(ActionKind::GitPullRebase { repo_path });
    }

    if repo.status.uncommitted_count > 0 && repo.status.unpushed_count > 0 {
        return Some(ActionKind::GitAddCommitPush {
            repo_path,
            message: "wip".to_string(),
        });
    }

    if repo.status.uncommitted_count > 0 {
        return Some(ActionKind::GitAddCommit {
            repo_path,
            message: "wip".to_string(),
        });
    }

    if repo.status.unpushed_count > 0 {
        return Some(ActionKind::GitPush { repo_path });
    }

    if repo.status.stash_count > 0 {
        return Some(ActionKind::GitStashList { repo_path });
    }

    if !repo.status.has_remote {
        return Some(ActionKind::GitRemoteList { repo_path });
    }

    None
}

pub fn sorted_recommendations(repos: &[Repo]) -> Vec<(&Repo, Recommendation)> {
    let mut items: Vec<(&Repo, Recommendation)> = repos.iter().map(|r| (r, recommend(r))).collect();
    items.sort_by(|(repo_a, rec_a), (repo_b, rec_b)| {
        rec_b
            .priority
            .rank()
            .cmp(&rec_a.priority.rank())
            .then_with(|| repo_a.name.cmp(&repo_b.name))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{Repo, RepoStatus};
    use std::path::PathBuf;

    fn repo_with_status(name: &str, status: RepoStatus) -> Repo {
        Repo {
            path: PathBuf::from(format!("/tmp/{}", name)),
            name: name.to_string(),
            status,
            last_checked: None,
        }
    }

    #[test]
    fn test_detached_is_critical() {
        let repo = repo_with_status(
            "detached",
            RepoStatus {
                branch: "HEAD".to_string(),
                uncommitted_count: 0,
                unpushed_count: 0,
                behind_count: 0,
                stash_count: 0,
                has_remote: true,
                is_detached: true,
            },
        );
        let rec = recommend(&repo);
        assert_eq!(rec.priority, ActionPriority::Critical);
    }

    #[test]
    fn test_commit_then_push_is_high() {
        let repo = repo_with_status(
            "busy",
            RepoStatus {
                branch: "main".to_string(),
                uncommitted_count: 3,
                unpushed_count: 2,
                behind_count: 0,
                stash_count: 0,
                has_remote: true,
                is_detached: false,
            },
        );
        let rec = recommend(&repo);
        assert_eq!(rec.priority, ActionPriority::High);
        assert_eq!(rec.short_action, "commit+push");
    }

    #[test]
    fn test_clean_repo_is_idle() {
        let repo = repo_with_status(
            "clean",
            RepoStatus {
                branch: "main".to_string(),
                uncommitted_count: 0,
                unpushed_count: 0,
                behind_count: 0,
                stash_count: 0,
                has_remote: true,
                is_detached: false,
            },
        );
        let rec = recommend(&repo);
        assert_eq!(rec.priority, ActionPriority::Idle);
    }
}
