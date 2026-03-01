use crate::dashboard::ActionKind;
use anyhow::anyhow;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct ActionCompletion {
    pub affected_repo_path: Option<String>,
}

/// Open a repo in the configured editor (detached process).
pub fn open_in_editor(repo_path: &Path, editor: &str) -> Result<()> {
    match editor {
        "code" | "vscode" => {
            std::process::Command::new("code").arg(repo_path).spawn()?;
        }
        "cursor" => {
            std::process::Command::new("cursor")
                .arg(repo_path)
                .spawn()?;
        }
        other => {
            std::process::Command::new(other).arg(repo_path).spawn()?;
        }
    }
    Ok(())
}

/// Open a repo in the OS file manager.
pub fn open_in_file_manager(repo_path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(repo_path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(repo_path)
            .spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(repo_path)
            .spawn()?;
    }
    Ok(())
}

/// Run `git commit -a -m <message>` asynchronously; send a status notification when done.
pub fn git_commit(
    repo_path: &Path,
    message: &str,
    notif_tx: Sender<String>,
    completion_tx: Sender<ActionCompletion>,
) {
    let path = repo_path.to_path_buf();
    let message = message.to_string();
    tokio::spawn(async move {
        let result = tokio::process::Command::new("git")
            .args(["commit", "-a", "-m", &message])
            .current_dir(&path)
            .output()
            .await;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let msg = match result {
            Ok(o) if o.status.success() => format!("✓  committed {} — \"{}\"", name, message),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                let first = err.lines().next().unwrap_or("nothing to commit");
                format!("✗  commit {} — {}", name, first)
            }
            Err(e) => format!("✗  commit {} — {}", name, e),
        };
        let _ = notif_tx.send(msg).await;
        let _ = completion_tx
            .send(ActionCompletion {
                affected_repo_path: Some(path.to_string_lossy().to_string()),
            })
            .await;
    });
}

/// Run a typed, allowlisted action asynchronously and report the first-line result.
pub fn run_action(
    action: ActionKind,
    notif_tx: Sender<String>,
    completion_tx: Sender<ActionCompletion>,
) {
    tokio::spawn(async move {
        let affected_repo_path = action.affected_repo_path().map(ToString::to_string);
        let msg = match execute_action(&action).await {
            Ok(first) => {
                let hint = success_hint(&action);
                if first.is_empty() {
                    format!("✓  action — done ({})", hint)
                } else {
                    format!("✓  action — {} ({})", first, hint)
                }
            }
            Err(e) => format!("✗  action — {} (review and retry)", e),
        };
        let _ = notif_tx.send(msg).await;
        let _ = completion_tx
            .send(ActionCompletion { affected_repo_path })
            .await;
    });
}

async fn execute_action(action: &ActionKind) -> Result<String> {
    match action {
        ActionKind::GitStatus { repo_path } => run_git(repo_path, &["status", "-sb"]).await,
        ActionKind::GitFetch { repo_path } => run_git(repo_path, &["fetch", "--quiet"]).await,
        ActionKind::GitPullRebase { repo_path } => run_git(repo_path, &["pull", "--rebase"]).await,
        ActionKind::GitPush { repo_path } => run_git(repo_path, &["push"]).await,
        ActionKind::GitWorktreeList { repo_path } => {
            run_git(repo_path, &["worktree", "list"]).await
        }
        ActionKind::GitAddCommitPullRebase { repo_path, message } => {
            run_git(repo_path, &["add", "-A"]).await?;
            run_git(repo_path, &["commit", "-m", message]).await?;
            run_git(repo_path, &["pull", "--rebase"]).await
        }
        ActionKind::GitPullRebasePush { repo_path } => {
            run_git(repo_path, &["pull", "--rebase"]).await?;
            run_git(repo_path, &["push"]).await
        }
        ActionKind::GitAddCommitPush { repo_path, message } => {
            run_git(repo_path, &["add", "-A"]).await?;
            run_git(repo_path, &["commit", "-m", message]).await?;
            run_git(repo_path, &["push"]).await
        }
        ActionKind::GitAddCommit { repo_path, message } => {
            run_git(repo_path, &["add", "-A"]).await?;
            run_git(repo_path, &["commit", "-m", message]).await
        }
        ActionKind::GitStashList { repo_path } => run_git(repo_path, &["stash", "list"]).await,
        ActionKind::GitRemoteList { repo_path } => run_git(repo_path, &["remote", "-v"]).await,
        ActionKind::GitSwitchCreate { repo_path, branch } => {
            run_git(repo_path, &["switch", "-c", branch]).await
        }
        ActionKind::KillProcess { pid } => run_cmd_owned(None, "kill", vec![pid.to_string()]).await,
        ActionKind::NpmInstallLockfile { repo_path } => {
            run_cmd(Some(repo_path), "npm", &["install", "--package-lock-only"]).await
        }
        ActionKind::CargoGenerateLockfile { repo_path } => {
            run_cmd(Some(repo_path), "cargo", &["generate-lockfile"]).await
        }
        ActionKind::UvLock { repo_path } => run_cmd(Some(repo_path), "uv", &["lock"]).await,
        ActionKind::PipCompileRequirements { repo_path } => {
            run_cmd(Some(repo_path), "pip-compile", &["requirements.txt"]).await
        }
        ActionKind::GoModTidy { repo_path } => {
            run_cmd(Some(repo_path), "go", &["mod", "tidy"]).await
        }
        ActionKind::BundleLock { repo_path } => run_cmd(Some(repo_path), "bundle", &["lock"]).await,
        ActionKind::IgnoreEnvFiles { repo_path, files } => {
            append_env_pattern_to_gitignore(repo_path)?;
            if files.is_empty() {
                return Ok("updated .gitignore".to_string());
            }
            let mut args = vec!["rm".to_string(), "--cached".to_string(), "--".to_string()];
            args.extend(files.clone());
            run_cmd_owned(Some(repo_path), "git", args).await
        }
        ActionKind::SeedEnvFromExample { repo_path } => {
            let from = Path::new(repo_path).join(".env.example");
            let to = Path::new(repo_path).join(".env");
            if !from.exists() {
                return Err(anyhow!(".env.example not found"));
            }
            fs::copy(&from, &to)?;
            Ok("seeded .env from .env.example".to_string())
        }
        ActionKind::ProbeBinaryHelp { binary } => run_cmd(None, binary, &["--help"]).await,
        ActionKind::CheckBinaryInPath { binary } => {
            if resolve_binary_in_path(binary).is_some() {
                Ok(format!("found {}", binary))
            } else {
                Err(anyhow!("{} not found in PATH", binary))
            }
        }
        ActionKind::ShowMessage { message } => Ok(message.clone()),
    }
}

async fn run_git(repo_path: &str, args: &[&str]) -> Result<String> {
    run_cmd(Some(repo_path), "git", args).await
}

async fn run_cmd(current_dir: Option<&str>, program: &str, args: &[&str]) -> Result<String> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().await?;
    if output.status.success() {
        Ok(first_line(&output.stdout))
    } else {
        let detail = first_line(&output.stderr);
        if detail.is_empty() {
            Err(anyhow!("{} failed", program))
        } else {
            Err(anyhow!(detail))
        }
    }
}

async fn run_cmd_owned(
    current_dir: Option<&str>,
    program: &str,
    args: Vec<String>,
) -> Result<String> {
    let mut cmd = tokio::process::Command::new(program);
    let owned_args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
    cmd.args(owned_args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().await?;
    if output.status.success() {
        Ok(first_line(&output.stdout))
    } else {
        let detail = first_line(&output.stderr);
        if detail.is_empty() {
            Err(anyhow!("{} failed", program))
        } else {
            Err(anyhow!(detail))
        }
    }
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn append_env_pattern_to_gitignore(repo_path: &str) -> Result<()> {
    let path = Path::new(repo_path).join(".gitignore");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == ".env*") {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(".env*\n");
    fs::write(path, updated)?;
    Ok(())
}

fn resolve_binary_in_path(binary: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists() && candidate.is_file())
}

fn success_hint(action: &ActionKind) -> &'static str {
    match action {
        ActionKind::KillProcess { .. } => "process stopped",
        ActionKind::IgnoreEnvFiles { .. } => "secrets protected; review git status",
        ActionKind::GitPullRebase { .. }
        | ActionKind::GitPush { .. }
        | ActionKind::GitAddCommit { .. }
        | ActionKind::GitAddCommitPush { .. }
        | ActionKind::GitAddCommitPullRebase { .. }
        | ActionKind::GitPullRebasePush { .. } => "changes applied; status will refresh",
        _ => "done",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn append_env_pattern_idempotent() {
        let base = std::env::temp_dir().join("agentpulse_gitignore_action_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let gitignore = base.join(".gitignore");
        fs::write(&gitignore, "target/\n").unwrap();

        append_env_pattern_to_gitignore(base.to_str().unwrap()).unwrap();
        append_env_pattern_to_gitignore(base.to_str().unwrap()).unwrap();

        let raw = fs::read_to_string(&gitignore).unwrap();
        let count = raw.lines().filter(|line| line.trim() == ".env*").count();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn resolves_git_in_path() {
        assert!(resolve_binary_in_path("git").is_some());
    }

    #[tokio::test]
    async fn run_action_emits_completion_for_non_repo_action() {
        let (notif_tx, mut notif_rx) = mpsc::channel(1);
        let (done_tx, mut done_rx) = mpsc::channel(1);

        run_action(
            ActionKind::ShowMessage {
                message: "hello".to_string(),
            },
            notif_tx,
            done_tx,
        );

        let notif = notif_rx.recv().await.expect("notification expected");
        assert!(notif.contains("hello"));

        let done = done_rx.recv().await.expect("completion expected");
        assert!(done.affected_repo_path.is_none());
    }

    #[tokio::test]
    async fn run_action_completion_includes_repo_path() {
        let repo_path = "/tmp/agentpulse-no-such-repo";
        let (notif_tx, mut notif_rx) = mpsc::channel(1);
        let (done_tx, mut done_rx) = mpsc::channel(1);

        run_action(
            ActionKind::GitStatus {
                repo_path: repo_path.to_string(),
            },
            notif_tx,
            done_tx,
        );

        let _ = notif_rx.recv().await.expect("notification expected");
        let done = done_rx.recv().await.expect("completion expected");
        assert_eq!(done.affected_repo_path.as_deref(), Some(repo_path));
    }
}
