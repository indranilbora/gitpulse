use crate::config::Config;
use crate::git::{check_repo_status, Repo, RepoStatus};
use crate::scanner::find_repos;
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tokio::task::JoinSet;

const MAX_CONCURRENT: usize = 20;

/// Cached entry: the mtime of `.git/index` at last check plus the result.
#[derive(Clone)]
pub struct CacheEntry {
    signals: CacheSignals,
    checked_at: Instant,
    status: RepoStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CacheSignals {
    index_mtime: Option<SystemTime>,
    head_mtime: Option<SystemTime>,
    fetch_head_mtime: Option<SystemTime>,
    remote_refs_mtime: Option<SystemTime>,
}

/// Persistent status cache keyed by repo path.
/// Pass this into successive `scan_all` calls to avoid re-running git commands
/// when the `.git/index` file hasn't changed.
pub type StatusCache = HashMap<PathBuf, CacheEntry>;

/// Scan all configured directories, check each repo's git status concurrently,
/// and return a sorted list with dirty repos first.
///
/// `cache` is updated in-place: entries whose `.git/index` mtime is unchanged
/// are reused without spawning new git processes.
pub async fn scan_all(config: &Config, cache: &mut StatusCache) -> Vec<Repo> {
    let paths = find_repos(&config.watch_directories, config.max_scan_depth);

    // Filter ignored repos by directory name
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !config.ignored_repos.iter().any(|ig| ig == name)
        })
        .collect();

    // Split into cache-hit repos (no git needed) and repos that need checking
    let mut repos: Vec<Repo> = Vec::with_capacity(paths.len());
    let mut to_check: Vec<PathBuf> = Vec::new();

    for path in &paths {
        if let Some(cached) = cache_hit(path, cache, stale_after(config.refresh_interval_secs)) {
            let mut repo = Repo::new(path.clone());
            repo.status = cached;
            repo.last_checked = Some(Local::now());
            repos.push(repo);
        } else {
            to_check.push(path.clone());
        }
    }

    // Check remaining repos in bounded concurrent batches
    for chunk in to_check.chunks(MAX_CONCURRENT) {
        let mut set: JoinSet<(PathBuf, Repo)> = JoinSet::new();
        for path in chunk {
            let path = path.clone();
            set.spawn(async move {
                let mut repo = Repo::new(path.clone());
                if let Ok(status) = check_repo_status(&path).await {
                    repo.status = status;
                    repo.last_checked = Some(Local::now());
                }
                (path, repo)
            });
        }
        while let Some(res) = set.join_next().await {
            if let Ok((path, repo)) = res {
                // Update cache with new repo state signals.
                if let Some(signals) = read_cache_signals(&path) {
                    cache.insert(
                        path,
                        CacheEntry {
                            signals,
                            checked_at: Instant::now(),
                            status: repo.status.clone(),
                        },
                    );
                }
                repos.push(repo);
            }
        }
    }

    // Sort: highest urgency first, then alphabetical by name
    repos.sort_by(|a, b| {
        b.urgency()
            .cmp(&a.urgency())
            .then_with(|| a.name.cmp(&b.name))
    });

    repos
}

/// Return the cached `RepoStatus` if `.git/index` hasn't changed, otherwise `None`.
fn cache_hit(path: &Path, cache: &StatusCache, max_age: Duration) -> Option<RepoStatus> {
    let signals = read_cache_signals(path)?;
    let entry = cache.get(path)?;
    if entry.checked_at.elapsed() <= max_age && entry.signals == signals {
        Some(entry.status.clone())
    } else {
        None
    }
}

fn stale_after(refresh_interval_secs: u64) -> Duration {
    // Keep remote-derived values fresh even if local mtimes don't change.
    let secs = refresh_interval_secs.saturating_mul(2).clamp(6, 30);
    Duration::from_secs(secs)
}

fn read_cache_signals(repo: &Path) -> Option<CacheSignals> {
    let git_dir = resolve_git_dir(repo)?;
    Some(CacheSignals {
        index_mtime: file_mtime(git_dir.join("index")),
        head_mtime: file_mtime(git_dir.join("HEAD")),
        fetch_head_mtime: file_mtime(git_dir.join("FETCH_HEAD")),
        remote_refs_mtime: latest_mtime_in_dir(git_dir.join("refs").join("remotes"))
            .or_else(|| file_mtime(git_dir.join("packed-refs"))),
    })
}

fn file_mtime(path: PathBuf) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn resolve_git_dir(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }

    // Worktree/submodule style: .git is a text file with "gitdir: <path>".
    let raw = fs::read_to_string(&dot_git).ok()?;
    let line = raw.lines().next()?.trim();
    let rel = line.strip_prefix("gitdir:")?.trim();
    let git_dir = PathBuf::from(rel);
    if git_dir.is_absolute() {
        Some(git_dir)
    } else {
        Some(repo.join(git_dir))
    }
}

fn latest_mtime_in_dir(path: PathBuf) -> Option<SystemTime> {
    if !path.is_dir() {
        return None;
    }

    let mut latest: Option<SystemTime> = None;
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Some(ts) = latest_mtime_in_dir(entry_path) {
                latest = Some(match latest {
                    Some(curr) => curr.max(ts),
                    None => ts,
                });
            }
        } else if let Some(ts) = file_mtime(entry_path) {
            latest = Some(match latest {
                Some(curr) => curr.max(ts),
                None => ts,
            });
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    fn init_repo(name: &str) -> PathBuf {
        let base = std::env::temp_dir()
            .join("agentpulse_monitor_test")
            .join(name);
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let status = StdCommand::new("git")
            .args(["init"])
            .current_dir(&base)
            .status()
            .unwrap();
        assert!(status.success());
        base
    }

    #[test]
    fn resolves_standard_git_dir() {
        let repo = init_repo("signals");
        let git_dir = resolve_git_dir(&repo).unwrap();
        assert!(git_dir.ends_with(".git"));
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn stale_after_is_bounded() {
        assert_eq!(stale_after(1), Duration::from_secs(6));
        assert_eq!(stale_after(100), Duration::from_secs(30));
    }

    #[test]
    fn latest_mtime_handles_missing_dir() {
        let missing = std::env::temp_dir().join("agentpulse-nope-dir");
        assert!(latest_mtime_in_dir(missing).is_none());
    }

    #[test]
    fn cache_hit_invalidates_on_age() {
        let repo = init_repo("age");
        let signals = read_cache_signals(&repo).unwrap();
        let mut cache = StatusCache::new();
        cache.insert(
            repo.clone(),
            CacheEntry {
                signals,
                checked_at: Instant::now() - Duration::from_secs(60),
                status: RepoStatus::default(),
            },
        );
        assert!(cache_hit(&repo, &cache, Duration::from_secs(5)).is_none());
        let _ = fs::remove_dir_all(&repo);
    }
}
