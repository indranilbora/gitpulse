use crate::config::{default_config_path, default_directories, Config};
use anyhow::Result;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Run the interactive setup wizard.
///
/// If `existing` is `Some`, the wizard starts with those directories pre-populated
/// (used when re-configuring via `s` key or `--setup` flag).
/// Returns the updated `Config` (already saved to disk).
pub fn run_setup(existing: Option<&Config>, config_path: Option<&PathBuf>) -> Result<Config> {
    let home = dirs::home_dir().unwrap_or_default();

    println!();
    println!("  ╔══════════════════════════════════════╗");
    println!("  ║       GitPulse — Setup Wizard        ║");
    println!("  ╚══════════════════════════════════════╝");
    println!();
    println!("  GitPulse will scan directories you choose for git repos.");
    println!();

    // Gather suggestions: standard dirs that actually exist on disk
    let suggestions: Vec<PathBuf> = ["Developer", "Projects", "repos", "code", "src", "work"]
        .iter()
        .map(|d| home.join(d))
        .filter(|p| p.exists())
        .collect();

    // Seed current dirs from existing config (reconfigure path)
    let chosen: Vec<PathBuf> = existing
        .map(|c| c.watch_directories.clone())
        .unwrap_or_default();

    // ── show current dirs if reconfiguring ──────────────────────────────────
    if !chosen.is_empty() {
        println!("  Current watch directories:");
        for (i, dir) in chosen.iter().enumerate() {
            let status = if dir.exists() { "✓" } else { "✗ not found" };
            println!("    [{}] {}  ({})", i + 1, dir.display(), status);
        }
        println!();
        println!("  Press Enter to keep these, or type new paths to replace them.");
        println!();
    }

    // ── show suggestions ────────────────────────────────────────────────────
    if !suggestions.is_empty() {
        println!("  Detected directories on this machine:");
        for (i, dir) in suggestions.iter().enumerate() {
            println!("    [{}] {}", i + 1, dir.display());
        }
        println!();
        println!("  Enter a number to add a suggestion, a full path, or leave");
        println!("  blank to finish. Supports ~ and $HOME.");
    } else {
        println!("  No standard directories detected. Enter full paths below.");
    }

    println!();

    // ── collect input ───────────────────────────────────────────────────────
    let mut new_dirs: Vec<PathBuf> = Vec::new();
    let stdin = io::stdin();

    loop {
        print!("  > ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.read_line(&mut line)?;
        let trimmed = line.trim();

        // Blank line = done
        if trimmed.is_empty() {
            break;
        }

        // Number → pick from suggestions list
        if let Ok(n) = trimmed.parse::<usize>() {
            if n >= 1 && n <= suggestions.len() {
                let path = suggestions[n - 1].clone();
                println!("    Added: {}", path.display());
                new_dirs.push(path);
                continue;
            } else {
                println!("    No suggestion [{}] — try again or enter a path", n);
                continue;
            }
        }

        // Full path (with ~ / $HOME expansion)
        let path = expand_home(trimmed, &home);
        if !path.exists() {
            println!(
                "    Warning: {} does not exist yet — added anyway",
                path.display()
            );
        } else {
            println!("    Added: {}", path.display());
        }
        new_dirs.push(path);
    }

    // ── decide final list ────────────────────────────────────────────────────
    let final_dirs = if new_dirs.is_empty() {
        if !chosen.is_empty() {
            // User pressed Enter immediately → keep existing
            println!("  Keeping current directories.");
            chosen
        } else {
            // Truly nothing entered → fall back to compile-time defaults
            println!("  Nothing entered — using default directories.");
            default_directories()
        }
    } else {
        new_dirs
    };

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let watch_directories: Vec<PathBuf> = final_dirs
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect();

    // ── build and save ───────────────────────────────────────────────────────
    let mut config = existing.cloned().unwrap_or_default();
    config.watch_directories = watch_directories;
    config.missing_directories = config
        .watch_directories
        .iter()
        .filter(|p| !p.exists())
        .cloned()
        .collect();

    let path = config_path.cloned().unwrap_or_else(default_config_path);
    save_config(&config, &path)?;

    println!();
    println!("  Saved to {}", path.display());
    println!("  Tip: run `gitpulse --setup` anytime to change these.");
    println!();

    Ok(config)
}

/// Expand `~` and `$HOME` in a user-entered path string.
fn expand_home(s: &str, home: &Path) -> PathBuf {
    if let Some(stripped) = s.strip_prefix("~/") {
        home.join(stripped)
    } else if s == "~" {
        home.to_path_buf()
    } else if let Some(stripped) = s.strip_prefix("$HOME/") {
        home.join(stripped)
    } else if s == "$HOME" {
        home.to_path_buf()
    } else {
        PathBuf::from(s)
    }
}

/// Serialise `Config` to the provided path with a comment header.
pub fn save_config(config: &Config, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Serialise to TOML, then prepend a comment block
    let body = toml::to_string_pretty(config)?;
    let content = format!(
        "# GitPulse configuration\n\
         # Run `gitpulse --setup` to reconfigure watch directories.\n\
         # See README.md for all options.\n\n\
         {}",
        body
    );

    std::fs::write(path, content)?;
    Ok(())
}
