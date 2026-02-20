use anyhow::Result;
use std::path::Path;
use tokio::sync::mpsc::Sender;

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

/// Run `git fetch` in the background.
pub fn git_fetch(repo_path: &Path) -> Result<()> {
    std::process::Command::new("git")
        .args(["fetch", "--quiet"])
        .current_dir(repo_path)
        .spawn()?;
    Ok(())
}

/// Run `git pull` asynchronously; send a status notification when done.
pub fn git_pull(repo_path: &Path, notif_tx: Sender<String>) {
    let path = repo_path.to_path_buf();
    tokio::spawn(async move {
        let result = tokio::process::Command::new("git")
            .args(["pull", "--quiet"])
            .current_dir(&path)
            .output()
            .await;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let msg = match result {
            Ok(o) if o.status.success() => format!("✓  pull {} — done", name),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                let first = err.lines().next().unwrap_or("unknown error");
                format!("✗  pull {} — {}", name, first)
            }
            Err(e) => format!("✗  pull {} — {}", name, e),
        };
        let _ = notif_tx.send(msg).await;
    });
}

/// Run `git push` asynchronously; send a status notification when done.
pub fn git_push(repo_path: &Path, notif_tx: Sender<String>) {
    let path = repo_path.to_path_buf();
    tokio::spawn(async move {
        let result = tokio::process::Command::new("git")
            .args(["push", "--quiet"])
            .current_dir(&path)
            .output()
            .await;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let msg = match result {
            Ok(o) if o.status.success() => format!("✓  push {} — done", name),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                let first = err.lines().next().unwrap_or("unknown error");
                format!("✗  push {} — {}", name, first)
            }
            Err(e) => format!("✗  push {} — {}", name, e),
        };
        let _ = notif_tx.send(msg).await;
    });
}

/// Run `git commit -a -m <message>` asynchronously; send a status notification when done.
pub fn git_commit(repo_path: &Path, message: &str, notif_tx: Sender<String>) {
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
    });
}
