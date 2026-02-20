use crate::config::Config;
use crate::git::{check_repo_status, Repo, RepoStatus};
use crate::scanner::find_repos;
use chrono::Local;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::task::JoinSet;

const MAX_CONCURRENT: usize = 20;

/// Cached entry: the mtime of `.git/index` at last check plus the result.
#[derive(Clone)]
pub struct CacheEntry {
    index_mtime: SystemTime,
    status: RepoStatus,
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
        if let Some(cached) = cache_hit(path, cache) {
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
                // Update cache with new mtime
                let mtime = index_mtime(&path);
                if let Some(mtime) = mtime {
                    cache.insert(
                        path,
                        CacheEntry {
                            index_mtime: mtime,
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
fn cache_hit(path: &Path, cache: &StatusCache) -> Option<RepoStatus> {
    let mtime = index_mtime(path)?;
    let entry = cache.get(path)?;
    if entry.index_mtime == mtime {
        Some(entry.status.clone())
    } else {
        None
    }
}

/// Return the modification time of `<repo>/.git/index`, or `None` if unavailable.
fn index_mtime(repo: &Path) -> Option<SystemTime> {
    std::fs::metadata(repo.join(".git").join("index"))
        .ok()
        .and_then(|m| m.modified().ok())
}
