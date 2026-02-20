use anyhow::Result;
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

/// The status of a single git repository.
#[derive(Debug, Clone, Default)]
pub struct RepoStatus {
    pub branch: String,
    pub uncommitted_count: usize,
    /// Commits ahead of the upstream (unpushed).
    pub unpushed_count: usize,
    /// Commits behind the upstream (need pull).
    pub behind_count: usize,
    pub stash_count: usize,
    pub has_remote: bool,
    pub is_detached: bool,
}

/// A discovered git repository with its current status.
#[derive(Debug, Clone)]
pub struct Repo {
    pub path: PathBuf,
    pub name: String,
    pub status: RepoStatus,
    pub last_checked: Option<DateTime<Local>>,
}

impl Repo {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self {
            path,
            name,
            status: RepoStatus::default(),
            last_checked: None,
        }
    }

    pub fn needs_attention(&self) -> bool {
        self.status.uncommitted_count > 0
            || self.status.unpushed_count > 0
            || self.status.behind_count > 0
    }

    pub fn urgency(&self) -> u8 {
        match (
            self.status.uncommitted_count > 0,
            self.status.unpushed_count > 0,
        ) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }

    pub fn status_color(&self) -> StatusColor {
        if !self.status.has_remote {
            StatusColor::NoRemote
        } else {
            match (
                self.status.uncommitted_count > 0,
                self.status.unpushed_count > 0,
            ) {
                (true, true) => StatusColor::Dirty,
                (true, false) => StatusColor::Uncommitted,
                (false, true) => StatusColor::Unpushed,
                (false, false) => StatusColor::Clean,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusColor {
    Clean,
    Uncommitted,
    Unpushed,
    Dirty,
    NoRemote,
}

const TIMEOUT: Duration = Duration::from_secs(5);

async fn run_git(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = tokio::time::timeout(
        TIMEOUT,
        Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output(),
    )
    .await??;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn get_branch(repo_path: &Path) -> Result<(String, bool)> {
    let raw = run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    let branch = raw.trim().to_string();
    let is_detached = branch == "HEAD";
    Ok((branch, is_detached))
}

pub async fn get_uncommitted_count(repo_path: &Path) -> Result<usize> {
    let raw = run_git(repo_path, &["status", "--porcelain"]).await?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Returns `(ahead, behind, has_remote)`.
pub async fn get_remote_counts(repo_path: &Path) -> Result<(usize, usize, bool)> {
    let remote_raw = run_git(repo_path, &["remote"]).await?;
    let has_remote = !remote_raw.trim().is_empty();
    if !has_remote {
        return Ok((0, 0, false));
    }

    let parse_count = |args: &'static [&'static str], path: PathBuf| async move {
        let result = tokio::time::timeout(
            TIMEOUT,
            Command::new("git").args(args).current_dir(&path).output(),
        )
        .await;
        match result {
            Ok(Ok(o)) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<usize>()
                .unwrap_or(0),
            _ => 0,
        }
    };

    let path = repo_path.to_path_buf();
    let (ahead, behind) = tokio::join!(
        parse_count(&["rev-list", "--count", "@{upstream}..HEAD"], path.clone()),
        parse_count(&["rev-list", "--count", "HEAD..@{upstream}"], path),
    );

    Ok((ahead, behind, true))
}

/// Count stashed changes.
pub async fn get_stash_count(repo_path: &Path) -> Result<usize> {
    let raw = run_git(repo_path, &["stash", "list"]).await?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Check all status for a single repo concurrently.
pub async fn check_repo_status(repo_path: &Path) -> Result<RepoStatus> {
    let (branch_res, uncommitted_res, remote_res, stash_res) = tokio::join!(
        get_branch(repo_path),
        get_uncommitted_count(repo_path),
        get_remote_counts(repo_path),
        get_stash_count(repo_path),
    );

    let (branch, is_detached) = branch_res.unwrap_or_else(|_| ("unknown".to_string(), false));
    let uncommitted_count = uncommitted_res.unwrap_or(0);
    let (unpushed_count, behind_count, has_remote) = remote_res.unwrap_or((0, 0, false));
    let stash_count = stash_res.unwrap_or(0);

    Ok(RepoStatus {
        branch,
        uncommitted_count,
        unpushed_count,
        behind_count,
        stash_count,
        has_remote,
        is_detached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    fn init_test_repo(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join("gitpulse_git_test").join(name);
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let run = |args: &[&str]| {
            StdCommand::new("git")
                .args(args)
                .current_dir(&base)
                .output()
                .unwrap()
        };
        run(&["init"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        base
    }

    #[tokio::test]
    async fn test_clean_repo_has_zero_counts() {
        let base = init_test_repo("clean");
        std::fs::write(base.join("README.md"), "hello").unwrap();
        StdCommand::new("git")
            .args(["add", "."])
            .current_dir(&base)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&base)
            .output()
            .unwrap();
        let status = check_repo_status(&base).await.unwrap();
        assert_eq!(status.uncommitted_count, 0);
        assert_eq!(status.unpushed_count, 0);
        assert_eq!(status.behind_count, 0);
        assert_eq!(status.stash_count, 0);
        assert!(!status.has_remote);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[tokio::test]
    async fn test_uncommitted_changes_counted() {
        let base = init_test_repo("dirty");
        std::fs::write(base.join("file.txt"), "change").unwrap();
        let count = get_uncommitted_count(&base).await.unwrap();
        assert_eq!(count, 1);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[tokio::test]
    async fn test_stash_count() {
        let base = init_test_repo("stash");
        std::fs::write(base.join("README.md"), "hello").unwrap();
        StdCommand::new("git")
            .args(["add", "."])
            .current_dir(&base)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&base)
            .output()
            .unwrap();
        // Create a stash
        std::fs::write(base.join("change.txt"), "unstaged").unwrap();
        StdCommand::new("git")
            .args(["add", "."])
            .current_dir(&base)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["stash"])
            .current_dir(&base)
            .output()
            .unwrap();
        let count = get_stash_count(&base).await.unwrap();
        assert_eq!(count, 1);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
