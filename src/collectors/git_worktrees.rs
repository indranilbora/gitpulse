use crate::agent;
use crate::dashboard::{ActionCommand, ActionKind, DashboardAlert, RepoRow, WorktreeRow};
use crate::git::Repo;
use std::path::Path;
use std::process::Command;

pub fn collect_repo_rows(repos: &[Repo]) -> Vec<RepoRow> {
    let mut rows: Vec<RepoRow> = repos
        .iter()
        .map(|repo| {
            let rec = agent::recommend(repo);
            let action = if rec.short_action == "noop" {
                None
            } else {
                agent::recommended_action_kind(repo)
                    .map(|kind| ActionCommand::new(rec.action, kind))
            };

            RepoRow {
                name: repo.name.clone(),
                path: repo.path.to_string_lossy().to_string(),
                branch: repo.status.branch.clone(),
                dirty: repo.status.uncommitted_count,
                ahead: repo.status.unpushed_count,
                behind: repo.status.behind_count,
                stash: repo.status.stash_count,
                recommendation: rec.short_action.to_string(),
                action,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.dirty
            .cmp(&a.dirty)
            .then_with(|| b.behind.cmp(&a.behind))
            .then_with(|| b.ahead.cmp(&a.ahead))
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

pub fn collect_worktrees(repos: &[Repo]) -> Vec<WorktreeRow> {
    let mut rows: Vec<WorktreeRow> = Vec::new();

    for repo in repos {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&repo.path)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let parsed = parse_worktree_output(repo, &String::from_utf8_lossy(&o.stdout));
                if parsed.is_empty() {
                    rows.push(default_worktree_row(repo));
                } else {
                    rows.extend(parsed);
                }
            }
            _ => rows.push(default_worktree_row(repo)),
        }
    }

    rows.sort_by(|a, b| a.repo.cmp(&b.repo).then_with(|| a.path.cmp(&b.path)));
    rows
}

pub fn collect_git_alerts(repo_rows: &[RepoRow], worktrees: &[WorktreeRow]) -> Vec<DashboardAlert> {
    let mut alerts = Vec::new();

    for row in repo_rows {
        if row.dirty > 0 {
            alerts.push(DashboardAlert {
                severity: "warn".to_string(),
                title: format!("{} has local changes", row.name),
                detail: format!("{} modified/untracked file(s)", row.dirty),
                repo: Some(row.name.clone()),
                action: Some(ActionCommand::new(
                    "open status",
                    ActionKind::GitStatus {
                        repo_path: row.path.clone(),
                    },
                )),
            });
        }

        if row.behind > 0 {
            alerts.push(DashboardAlert {
                severity: "high".to_string(),
                title: format!("{} is behind remote", row.name),
                detail: format!("{} commit(s) behind", row.behind),
                repo: Some(row.name.clone()),
                action: Some(ActionCommand::new(
                    "pull --rebase",
                    ActionKind::GitPullRebase {
                        repo_path: row.path.clone(),
                    },
                )),
            });
        }

        if row.ahead > 0 {
            alerts.push(DashboardAlert {
                severity: "info".to_string(),
                title: format!("{} has unpushed commits", row.name),
                detail: format!("{} commit(s) ahead", row.ahead),
                repo: Some(row.name.clone()),
                action: Some(ActionCommand::new(
                    "push",
                    ActionKind::GitPush {
                        repo_path: row.path.clone(),
                    },
                )),
            });
        }
    }

    for wt in worktrees.iter().filter(|w| w.detached) {
        alerts.push(DashboardAlert {
            severity: "high".to_string(),
            title: format!("Detached worktree in {}", wt.repo),
            detail: format!("{} is detached", wt.path),
            repo: Some(wt.repo.clone()),
            action: Some(ActionCommand::new(
                "inspect worktree",
                ActionKind::GitStatus {
                    repo_path: wt.path.clone(),
                },
            )),
        });
    }

    alerts.truncate(120);
    alerts
}

fn default_worktree_row(repo: &Repo) -> WorktreeRow {
    WorktreeRow {
        repo: repo.name.clone(),
        path: repo.path.to_string_lossy().to_string(),
        branch: repo.status.branch.clone(),
        detached: repo.status.is_detached,
        bare: false,
        action: Some(ActionCommand::new(
            "list worktrees",
            ActionKind::GitWorktreeList {
                repo_path: repo.path.to_string_lossy().to_string(),
            },
        )),
    }
}

fn parse_worktree_output(repo: &Repo, raw: &str) -> Vec<WorktreeRow> {
    #[derive(Default)]
    struct Current {
        path: String,
        branch: String,
        detached: bool,
        bare: bool,
    }

    let mut out = Vec::new();
    let mut current = Current::default();

    let flush = |acc: &mut Vec<WorktreeRow>, cur: &mut Current| {
        if cur.path.is_empty() {
            return;
        }
        acc.push(WorktreeRow {
            repo: repo.name.clone(),
            path: cur.path.clone(),
            branch: if cur.branch.is_empty() {
                repo.status.branch.clone()
            } else {
                cur.branch.clone()
            },
            detached: cur.detached,
            bare: cur.bare,
            action: Some(ActionCommand::new(
                "open worktree",
                ActionKind::GitStatus {
                    repo_path: cur.path.clone(),
                },
            )),
        });

        cur.path.clear();
        cur.branch.clear();
        cur.detached = false;
        cur.bare = false;
    };

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(&mut out, &mut current);
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            if !current.path.is_empty() {
                flush(&mut out, &mut current);
            }
            current.path = path.to_string();
            continue;
        }

        if let Some(branch) = line.strip_prefix("branch ") {
            current.branch = branch
                .strip_prefix("refs/heads/")
                .unwrap_or(branch)
                .to_string();
            continue;
        }

        if line == "detached" {
            current.detached = true;
            continue;
        }

        if line == "bare" {
            current.bare = true;
        }
    }

    flush(&mut out, &mut current);

    out
}

#[allow(dead_code)]
fn _is_path_inside_repo(path: &str, repo_root: &Path) -> bool {
    path.starts_with(&repo_root.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{Repo, RepoStatus};
    use std::path::PathBuf;

    #[test]
    fn parses_worktree_porcelain() {
        let mut repo = Repo::new(PathBuf::from("/tmp/example"));
        repo.status = RepoStatus {
            branch: "main".to_string(),
            uncommitted_count: 0,
            unpushed_count: 0,
            behind_count: 0,
            stash_count: 0,
            has_remote: true,
            is_detached: false,
        };

        let raw = "worktree /tmp/example\nHEAD deadbeef\nbranch refs/heads/main\n\nworktree /tmp/example-wt\nHEAD cafe\ndetached\n";
        let rows = parse_worktree_output(&repo, raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].branch, "main");
        assert!(rows[1].detached);
    }
}
