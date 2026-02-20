/// Integration tests for GitPulse scanner + git status checker.
///
/// Each test creates real git repositories in a temp directory and exercises
/// the scanner and/or status-checking code against them.
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── helpers ────────────────────────────────────────────────────────────────

fn tmp_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("gitpulse_integration").join(name);
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git command failed")
}

/// Initialise a bare git repo with a configured identity and an initial commit.
fn init_repo(base: &Path, name: &str) -> PathBuf {
    let repo = base.join(name);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init"]);
    git(&repo, &["config", "user.email", "test@test.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    // Create an initial commit so HEAD is valid
    std::fs::write(repo.join("README.md"), format!("# {}", name)).unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "init"]);
    repo
}

/// Add an uncommitted (untracked) file to `repo`.
fn add_untracked(repo: &Path, filename: &str) {
    std::fs::write(repo.join(filename), "dirty").unwrap();
}

/// Stage a file without committing.
fn add_staged(repo: &Path, filename: &str) {
    std::fs::write(repo.join(filename), "staged").unwrap();
    git(repo, &["add", filename]);
}

// ─── scanner tests ──────────────────────────────────────────────────────────

#[test]
fn test_scanner_finds_five_repos() {
    let base = tmp_dir("scanner_five");
    for name in &["alpha", "beta", "gamma", "delta", "epsilon"] {
        init_repo(&base, name);
    }

    let found = gitpulse::scanner::find_repos(std::slice::from_ref(&base), 3);
    assert_eq!(
        found.len(),
        5,
        "expected 5 repos, got {}: {:?}",
        found.len(),
        found
    );

    let names: Vec<String> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    for expected in &["alpha", "beta", "gamma", "delta", "epsilon"] {
        assert!(
            names.contains(&expected.to_string()),
            "missing repo: {}",
            expected
        );
    }
}

#[test]
fn test_scanner_does_not_recurse_into_repo() {
    let base = tmp_dir("scanner_no_recurse");
    let outer = init_repo(&base, "outer");
    // A nested repo inside the working tree of `outer`
    let inner = outer.join("nested");
    std::fs::create_dir_all(&inner).unwrap();
    git(&inner, &["init"]);

    let found = gitpulse::scanner::find_repos(std::slice::from_ref(&base), 5);
    // Only `outer` should appear; scanner stops at first .git
    assert_eq!(found.len(), 1);
    assert_eq!(found[0], outer);
}

#[test]
fn test_scanner_respects_depth_limit() {
    let base = tmp_dir("scanner_depth");
    // Create repo at depth 4 (base / a / b / c / repo)
    let deep = base.join("a").join("b").join("c").join("repo");
    std::fs::create_dir_all(&deep).unwrap();
    git(&deep, &["init"]);

    let found_shallow = gitpulse::scanner::find_repos(std::slice::from_ref(&base), 2);
    assert!(found_shallow.is_empty(), "depth=2 should miss depth-4 repo");

    let found_deep = gitpulse::scanner::find_repos(std::slice::from_ref(&base), 4);
    assert_eq!(found_deep.len(), 1);
}

// ─── git status tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_status_clean_repo() {
    let base = tmp_dir("status_clean");
    let repo = init_repo(&base, "clean");

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert_eq!(
        status.uncommitted_count, 0,
        "clean repo should have 0 uncommitted"
    );
    assert_eq!(status.unpushed_count, 0);
    assert!(!status.has_remote);
}

#[tokio::test]
async fn test_status_untracked_file() {
    let base = tmp_dir("status_untracked");
    let repo = init_repo(&base, "dirty");
    add_untracked(&repo, "new_file.txt");

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert_eq!(status.uncommitted_count, 1);
}

#[tokio::test]
async fn test_status_staged_file() {
    let base = tmp_dir("status_staged");
    let repo = init_repo(&base, "staged");
    add_staged(&repo, "staged.txt");

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert_eq!(status.uncommitted_count, 1);
}

#[tokio::test]
async fn test_status_multiple_dirty_files() {
    let base = tmp_dir("status_multi");
    let repo = init_repo(&base, "multi");
    add_untracked(&repo, "a.txt");
    add_untracked(&repo, "b.txt");
    add_staged(&repo, "c.txt");

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert_eq!(status.uncommitted_count, 3);
}

#[tokio::test]
async fn test_status_no_remote() {
    let base = tmp_dir("status_no_remote");
    let repo = init_repo(&base, "norepo");

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert!(!status.has_remote);
    assert_eq!(status.unpushed_count, 0);
}

#[tokio::test]
async fn test_status_branch_name() {
    let base = tmp_dir("status_branch");
    let repo = init_repo(&base, "branched");
    git(&repo, &["checkout", "-b", "feature/test"]);

    let status = gitpulse::git::check_repo_status(&repo).await.unwrap();
    assert_eq!(status.branch, "feature/test");
    assert!(!status.is_detached);
}

// ─── urgency / status_color tests ───────────────────────────────────────────

#[tokio::test]
async fn test_urgency_ordering() {
    use gitpulse::git::{Repo, RepoStatus, StatusColor};

    let make = |uncommitted: usize, unpushed: usize, has_remote: bool| {
        let mut r = Repo::new(PathBuf::from("/tmp/test"));
        r.status = RepoStatus {
            branch: "main".into(),
            uncommitted_count: uncommitted,
            unpushed_count: unpushed,
            behind_count: 0,
            stash_count: 0,
            has_remote,
            is_detached: false,
        };
        r
    };

    let dirty = make(2, 1, true);
    let uncommitted = make(1, 0, true);
    let unpushed = make(0, 3, true);
    let clean = make(0, 0, true);
    let no_remote = make(0, 0, false);

    assert_eq!(dirty.urgency(), 3);
    assert_eq!(uncommitted.urgency(), 2);
    assert_eq!(unpushed.urgency(), 1);
    assert_eq!(clean.urgency(), 0);

    assert_eq!(dirty.status_color(), StatusColor::Dirty);
    assert_eq!(uncommitted.status_color(), StatusColor::Uncommitted);
    assert_eq!(unpushed.status_color(), StatusColor::Unpushed);
    assert_eq!(clean.status_color(), StatusColor::Clean);
    assert_eq!(no_remote.status_color(), StatusColor::NoRemote);
}

// ─── monitor sort order ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_monitor_dirty_repos_sorted_first() {
    let base = tmp_dir("monitor_sort");
    let clean1 = init_repo(&base, "alpha_clean");
    let dirty = init_repo(&base, "beta_dirty");
    let clean2 = init_repo(&base, "gamma_clean");
    add_untracked(&dirty, "change.txt");

    let cfg = gitpulse::config::Config {
        watch_directories: vec![base.clone()],
        refresh_interval_secs: 60,
        max_scan_depth: 2,
        editor: None,
        show_clean: true,
        ignored_repos: vec![],
        watch_mode: false,
        missing_directories: vec![],
    };

    let mut cache = gitpulse::monitor::StatusCache::new();
    let repos = gitpulse::monitor::scan_all(&cfg, &mut cache).await;

    assert_eq!(repos.len(), 3);
    // dirty repo should be first
    assert_eq!(repos[0].path, dirty);
    // remaining two should be alphabetical
    let remaining: Vec<&PathBuf> = repos[1..].iter().map(|r| &r.path).collect();
    assert!(remaining.contains(&&clean1));
    assert!(remaining.contains(&&clean2));
}
