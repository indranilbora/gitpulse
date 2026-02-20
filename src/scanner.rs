use std::collections::HashSet;
use std::path::{Path, PathBuf};

static SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".build",
    "Pods",
    "DerivedData",
    "vendor",
    "venv",
    "dist",
    "build",
    ".next",
    "target",
    "__pycache__",
    ".gradle",
    ".cache",
];

/// Recursively find all git repositories under the given directories up to `max_depth`.
pub fn find_repos(directories: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let skip_set: HashSet<&str> = SKIP_DIRS.iter().copied().collect();
    let mut repos = Vec::new();

    for dir in directories {
        if !dir.is_dir() {
            continue;
        }
        scan_dir(dir, 0, max_depth, &skip_set, &mut repos);
    }

    repos.sort();
    repos.dedup();
    repos
}

fn scan_dir(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    skip_set: &HashSet<&str>,
    repos: &mut Vec<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    // If this directory contains .git, it's a repo — record and stop descending.
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        repos.push(dir.to_path_buf());
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // permission denied or similar — skip silently
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        // Skip hidden directories (names starting with `.`)
        if name.starts_with('.') {
            continue;
        }

        // Skip known noise directories
        if skip_set.contains(name) {
            continue;
        }

        scan_dir(&path, depth + 1, max_depth, skip_set, repos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_git_repo(base: &Path, name: &str) -> PathBuf {
        let repo = base.join(name);
        fs::create_dir_all(repo.join(".git")).unwrap();
        repo
    }

    #[test]
    fn test_finds_repos() {
        let base = std::env::temp_dir().join("gitpulse_scanner_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        make_git_repo(&base, "repo_a");
        make_git_repo(&base, "repo_b");

        // Should NOT be found (nested inside repo_a, scanner stops at .git)
        let nested = base.join("repo_a").join("subdir");
        fs::create_dir_all(nested.join(".git")).unwrap();

        let repos = find_repos(std::slice::from_ref(&base), 3);
        assert!(repos.contains(&base.join("repo_a")));
        assert!(repos.contains(&base.join("repo_b")));
        // repo_a/subdir should not appear because we stopped at repo_a
        assert!(!repos.contains(&base.join("repo_a").join("subdir")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_skips_node_modules() {
        let base = std::env::temp_dir().join("gitpulse_skip_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let nm = base.join("node_modules").join("some_pkg");
        fs::create_dir_all(nm.join(".git")).unwrap();

        let repos = find_repos(std::slice::from_ref(&base), 3);
        assert!(repos.is_empty());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_missing_directory_is_skipped() {
        let repos = find_repos(&[PathBuf::from("/nonexistent/path")], 3);
        assert!(repos.is_empty());
    }
}
