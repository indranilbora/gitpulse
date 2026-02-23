use crate::dashboard::{ActionCommand, DependencyHealth, EnvAuditResult, RepoProcess};
use crate::git::Repo;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn collect_repo_processes(repos: &[Repo]) -> Vec<RepoProcess> {
    let repo_paths: Vec<(String, String)> = repos
        .iter()
        .map(|r| (r.name.clone(), r.path.to_string_lossy().to_string()))
        .collect();

    let output = match Command::new("ps")
        .args(["-axo", "pid=,etime=,command="])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut rows = Vec::new();

    for line in raw.lines() {
        let mut parts = line.trim().splitn(3, char::is_whitespace);
        let pid_raw = parts.next().unwrap_or_default();
        let elapsed = parts.next().unwrap_or_default().to_string();
        let command = parts.next().unwrap_or_default().to_string();

        if pid_raw.is_empty() || command.is_empty() {
            continue;
        }

        let pid = match pid_raw.parse::<i32>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        for (repo_name, repo_path) in &repo_paths {
            if command.contains(repo_path) {
                rows.push(RepoProcess {
                    repo: repo_name.clone(),
                    pid,
                    elapsed: elapsed.clone(),
                    command: trim_command(&command, 160),
                    action: Some(ActionCommand {
                        label: "kill process".to_string(),
                        command: format!("kill {}", pid),
                    }),
                });
                break;
            }
        }
    }

    rows.sort_by(|a, b| a.repo.cmp(&b.repo).then_with(|| a.pid.cmp(&b.pid)));
    rows.truncate(200);
    rows
}

pub fn collect_dependency_health(repos: &[Repo]) -> Vec<DependencyHealth> {
    let mut out = Vec::new();

    for repo in repos {
        let root = &repo.path;
        let mut ecosystems = Vec::new();
        let mut issues = Vec::new();
        let mut action: Option<ActionCommand> = None;

        let has = |f: &str| root.join(f).exists();

        if has("package.json") {
            ecosystems.push("node".to_string());
            if !(has("package-lock.json")
                || has("yarn.lock")
                || has("pnpm-lock.yaml")
                || has("bun.lockb"))
            {
                issues.push("package.json without lockfile".to_string());
                action = Some(ActionCommand {
                    label: "create lockfile".to_string(),
                    command: format!("cd {:?} && npm install --package-lock-only", root),
                });
            }
        }

        if has("Cargo.toml") {
            ecosystems.push("rust".to_string());
            if !has("Cargo.lock") {
                issues.push("Cargo.toml without Cargo.lock".to_string());
                action.get_or_insert(ActionCommand {
                    label: "generate lockfile".to_string(),
                    command: format!("cd {:?} && cargo generate-lockfile", root),
                });
            }
        }

        if has("pyproject.toml") || has("requirements.txt") {
            ecosystems.push("python".to_string());
            if has("pyproject.toml")
                && !(has("poetry.lock") || has("uv.lock") || has("requirements.txt"))
            {
                issues.push("pyproject.toml without lock/export file".to_string());
                action.get_or_insert(ActionCommand {
                    label: "lock python deps".to_string(),
                    command: format!("cd {:?} && uv lock", root),
                });
            }
            if has("requirements.txt") {
                let unconstrained =
                    count_unconstrained_requirements(&root.join("requirements.txt"));
                if unconstrained > 0 {
                    issues.push(format!(
                        "requirements.txt has {} unconstrained entries",
                        unconstrained
                    ));
                    action.get_or_insert(ActionCommand {
                        label: "pin requirements".to_string(),
                        command: format!("cd {:?} && pip-compile requirements.txt", root),
                    });
                }
            }
        }

        if has("go.mod") {
            ecosystems.push("go".to_string());
            if !has("go.sum") {
                issues.push("go.mod without go.sum".to_string());
                action.get_or_insert(ActionCommand {
                    label: "generate go.sum".to_string(),
                    command: format!("cd {:?} && go mod tidy", root),
                });
            }
        }

        if has("Gemfile") {
            ecosystems.push("ruby".to_string());
            if !has("Gemfile.lock") {
                issues.push("Gemfile without Gemfile.lock".to_string());
                action.get_or_insert(ActionCommand {
                    label: "generate Gemfile.lock".to_string(),
                    command: format!("cd {:?} && bundle lock", root),
                });
            }
        }

        if ecosystems.is_empty() {
            continue;
        }

        out.push(DependencyHealth {
            repo: repo.name.clone(),
            path: root.to_string_lossy().to_string(),
            ecosystems,
            issue_count: issues.len(),
            issues,
            action,
        });
    }

    out.sort_by(|a, b| {
        b.issue_count
            .cmp(&a.issue_count)
            .then_with(|| a.repo.cmp(&b.repo))
    });
    out
}

pub fn collect_env_audit(repos: &[Repo]) -> Vec<EnvAuditResult> {
    let mut out = Vec::new();

    for repo in repos {
        let root = &repo.path;
        let env_files = discover_env_files(root);
        if env_files.is_empty() {
            continue;
        }

        let mut expected = BTreeSet::new();
        let mut actual = BTreeSet::new();
        let mut tracked_secret_files = Vec::new();
        let mut display_files = Vec::new();

        for file in &env_files {
            let rel = file
                .strip_prefix(root)
                .unwrap_or(file)
                .to_string_lossy()
                .to_string();
            display_files.push(rel.clone());

            let keys = parse_env_keys(file);
            if is_example_env_file(file) {
                expected.extend(keys.iter().cloned());
            } else {
                actual.extend(keys.iter().cloned());
            }

            if !is_example_env_file(file)
                && contains_sensitive_keys(&keys)
                && is_tracked_file(root, &rel)
            {
                tracked_secret_files.push(rel);
            }
        }

        let missing_keys = expected
            .difference(&actual)
            .cloned()
            .collect::<Vec<String>>();
        let extra_keys = actual
            .difference(&expected)
            .cloned()
            .collect::<Vec<String>>();

        let action = if !tracked_secret_files.is_empty() {
            Some(ActionCommand {
                label: "ignore env files".to_string(),
                command: format!(
                    "cd {:?} && printf '\n.env*\n' >> .gitignore && git rm --cached {}",
                    root,
                    tracked_secret_files.join(" ")
                ),
            })
        } else if !missing_keys.is_empty() {
            Some(ActionCommand {
                label: "seed .env from example".to_string(),
                command: format!("cd {:?} && cp .env.example .env", root),
            })
        } else {
            None
        };

        out.push(EnvAuditResult {
            repo: repo.name.clone(),
            path: root.to_string_lossy().to_string(),
            env_files: display_files,
            missing_keys,
            extra_keys,
            tracked_secret_files,
            action,
        });
    }

    out.sort_by(|a, b| {
        b.tracked_secret_files
            .len()
            .cmp(&a.tracked_secret_files.len())
            .then_with(|| {
                (b.missing_keys.len() + b.extra_keys.len())
                    .cmp(&(a.missing_keys.len() + a.extra_keys.len()))
            })
            .then_with(|| a.repo.cmp(&b.repo))
    });
    out
}

fn discover_env_files(root: &Path) -> Vec<PathBuf> {
    let candidates = [
        ".env",
        ".env.local",
        ".env.development",
        ".env.production",
        ".env.test",
        ".env.example",
        ".env.sample",
    ];

    let mut out = Vec::new();
    for name in candidates {
        let p = root.join(name);
        if p.exists() {
            out.push(p);
        }
    }
    out
}

fn parse_env_keys(path: &Path) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return keys;
    };

    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let normalized = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, _)) = normalized.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            keys.insert(key.to_string());
        }
    }

    keys
}

fn contains_sensitive_keys(keys: &BTreeSet<String>) -> bool {
    keys.iter().any(|k| {
        let up = k.to_ascii_uppercase();
        up.contains("SECRET")
            || up.contains("TOKEN")
            || up.contains("PASSWORD")
            || up.contains("API_KEY")
            || up.contains("PRIVATE_KEY")
    })
}

fn is_example_env_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".example") || name.ends_with(".sample")
}

fn is_tracked_file(repo_root: &Path, rel_path: &str) -> bool {
    match Command::new("git")
        .args(["ls-files", "--error-unmatch", rel_path])
        .current_dir(repo_root)
        .output()
    {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn count_unconstrained_requirements(path: &Path) -> usize {
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| {
            !(l.contains("==")
                || l.contains(">=")
                || l.contains("<=")
                || l.contains("~=")
                || l.contains("@"))
        })
        .count()
}

fn trim_command(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out = s
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect::<String>();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_keys_without_values() {
        let tmp = std::env::temp_dir().join("agentpulse_env_keys_test");
        let _ = fs::remove_file(&tmp);
        fs::write(&tmp, "# comment\nAPI_KEY=abc\nexport DEBUG=true\n").unwrap();
        let keys = parse_env_keys(&tmp);
        assert!(keys.contains("API_KEY"));
        assert!(keys.contains("DEBUG"));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn counts_unconstrained_requirements() {
        let tmp = std::env::temp_dir().join("agentpulse_requirements_test.txt");
        let _ = fs::remove_file(&tmp);
        fs::write(&tmp, "flask\nrequests==2.0\n#x\nclick>=8\n").unwrap();
        assert_eq!(count_unconstrained_requirements(&tmp), 1);
        let _ = fs::remove_file(&tmp);
    }
}
