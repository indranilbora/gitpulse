use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_directories")]
    pub watch_directories: Vec<PathBuf>,

    #[serde(default = "default_refresh")]
    pub refresh_interval_secs: u64,

    #[serde(default = "default_depth")]
    pub max_scan_depth: usize,

    #[serde(default)]
    pub editor: Option<String>,

    #[serde(default = "default_show_clean")]
    pub show_clean: bool,

    #[serde(default)]
    pub ignored_repos: Vec<String>,

    /// Use filesystem events (notify crate) instead of polling for auto-refresh.
    /// More responsive but slightly higher resource use. Default: false.
    #[serde(default)]
    pub watch_mode: bool,

    /// Directories that exist in config but were not found on disk (populated at load time, never serialised).
    #[serde(skip)]
    pub missing_directories: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_directories: default_directories(),
            refresh_interval_secs: default_refresh(),
            max_scan_depth: default_depth(),
            editor: None,
            show_clean: true,
            ignored_repos: Vec::new(),
            watch_mode: false,
            missing_directories: Vec::new(),
        }
    }
}

pub fn default_directories() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join("Developer"),
        home.join("Projects"),
        home.join("repos"),
    ]
}

fn default_refresh() -> u64 {
    60
}

fn default_depth() -> usize {
    3
}

fn default_show_clean() -> bool {
    true
}

/// Default config file location: `~/.config/gitpulse/config.toml`.
pub fn default_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("gitpulse")
        .join("config.toml")
}

/// Load config, creating a default file on first run if none exists.
pub fn load_config(config_path: Option<&PathBuf>) -> Result<Config> {
    let path = config_path.cloned().unwrap_or_else(default_config_path);

    if !path.exists() {
        // First run: write a default config with explanatory comments.
        // Ignore errors (e.g. read-only path, permission denied) — just use defaults.
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_ok() {
                let _ = std::fs::write(&path, default_config_toml());
            }
        }
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&path)?;
    let mut config: Config = toml::from_str(&contents)?;

    // Expand ~ and $HOME in watch_directories
    let home = dirs::home_dir().unwrap_or_default();
    config.watch_directories = config
        .watch_directories
        .into_iter()
        .map(|p| expand_home(p, &home))
        .collect();

    // Validate: record directories that don't exist (non-fatal)
    config.missing_directories = config
        .watch_directories
        .iter()
        .filter(|p| !p.exists())
        .cloned()
        .collect();

    Ok(config)
}

/// Expand `~` and `$HOME` prefixes to the actual home directory.
fn expand_home(path: PathBuf, home: &Path) -> PathBuf {
    let s = path.to_string_lossy();

    if let Some(stripped) = s.strip_prefix("~/") {
        return home.join(stripped);
    }
    if s == "~" {
        return home.to_path_buf();
    }
    if let Some(stripped) = s.strip_prefix("$HOME/") {
        return home.join(stripped);
    }
    if s == "$HOME" {
        return home.to_path_buf();
    }

    path
}

fn default_config_toml() -> &'static str {
    r#"# GitPulse configuration
# ~/.config/gitpulse/config.toml

# Directories to scan recursively for git repositories.
# Supports ~ and $HOME expansion.
watch_directories = [
    "~/Developer",
    "~/Projects",
    "~/repos",
]

# How often to auto-refresh status (seconds).
refresh_interval_secs = 60

# Maximum directory depth to recurse when looking for .git folders.
max_scan_depth = 3

# Editor command used when you press Enter on a repo.
# Defaults to $EDITOR env var, then "code" (VS Code).
# editor = "cursor"

# Set to false to hide clean repos (only show dirty ones).
show_clean = true

# Repository directory names to skip entirely.
# ignored_repos = ["old-project", "archived-thing"]
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.refresh_interval_secs, 60);
        assert_eq!(cfg.max_scan_depth, 3);
        assert!(cfg.show_clean);
        assert!(cfg.editor.is_none());
        assert!(cfg.ignored_repos.is_empty());
    }

    #[test]
    fn test_load_config_missing_file() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let cfg = load_config(Some(&path)).unwrap();
        assert_eq!(cfg.refresh_interval_secs, 60);
    }

    #[test]
    fn test_load_config_partial_toml() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("gitpulse_test_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "refresh_interval_secs = 30").unwrap();
        let cfg = load_config(Some(&path)).unwrap();
        assert_eq!(cfg.refresh_interval_secs, 30);
        assert_eq!(cfg.max_scan_depth, 3); // still default
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_expand_home_tilde() {
        let home = PathBuf::from("/home/user");
        let p = expand_home(PathBuf::from("~/Projects"), &home);
        assert_eq!(p, PathBuf::from("/home/user/Projects"));
    }

    #[test]
    fn test_expand_home_dollar() {
        let home = PathBuf::from("/home/user");
        let p = expand_home(PathBuf::from("$HOME/code"), &home);
        assert_eq!(p, PathBuf::from("/home/user/code"));
    }

    #[test]
    fn test_missing_dir_recorded() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("gitpulse_test_missing");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "watch_directories = [\"/nonexistent/dir/xyz\"]").unwrap();
        let cfg = load_config(Some(&path)).unwrap();
        assert_eq!(cfg.missing_directories.len(), 1);
        std::fs::remove_file(&path).unwrap();
    }
}
