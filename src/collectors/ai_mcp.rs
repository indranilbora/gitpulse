use crate::dashboard::{ActionCommand, McpServerHealth, ProviderKind, ProviderUsage};
use crate::git::Repo;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn collect_mcp_servers(repos: &[Repo]) -> Vec<McpServerHealth> {
    let mut config_paths = BTreeSet::new();
    for p in candidate_global_mcp_paths() {
        if p.exists() {
            config_paths.insert(p);
        }
    }

    for repo in repos {
        for rel in [
            ".mcp.json",
            "mcp.json",
            ".cursor/mcp.json",
            ".vscode/mcp.json",
        ] {
            let p = repo.path.join(rel);
            if p.exists() {
                config_paths.insert(p);
            }
        }
    }

    let mut out = Vec::new();
    for path in config_paths {
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                out.push(McpServerHealth {
                    source: path.to_string_lossy().to_string(),
                    server_name: "(config read failed)".to_string(),
                    command: String::new(),
                    healthy: false,
                    detail: e.to_string(),
                    action: None,
                });
                continue;
            }
        };

        let value: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                out.push(McpServerHealth {
                    source: path.to_string_lossy().to_string(),
                    server_name: "(invalid json)".to_string(),
                    command: String::new(),
                    healthy: false,
                    detail: e.to_string(),
                    action: None,
                });
                continue;
            }
        };

        let servers = extract_mcp_servers(&value);
        if servers.is_empty() {
            out.push(McpServerHealth {
                source: path.to_string_lossy().to_string(),
                server_name: "(no servers)".to_string(),
                command: String::new(),
                healthy: false,
                detail: "No mcpServers/servers entries found".to_string(),
                action: None,
            });
            continue;
        }

        for (name, command) in servers {
            let (healthy, detail, binary) = check_server_command(&command);
            let action = if healthy {
                if command.starts_with("http://") || command.starts_with("https://") {
                    None
                } else {
                    Some(ActionCommand {
                        label: "probe server".to_string(),
                        command: format!("{} --help", binary),
                    })
                }
            } else if binary.is_empty() {
                None
            } else {
                Some(ActionCommand {
                    label: "check binary".to_string(),
                    command: format!("command -v {}", binary),
                })
            };

            out.push(McpServerHealth {
                source: path.to_string_lossy().to_string(),
                server_name: name,
                command,
                healthy,
                detail,
                action,
            });
        }
    }

    out.sort_by(|a, b| {
        a.healthy
            .cmp(&b.healthy)
            .then_with(|| a.server_name.cmp(&b.server_name))
            .then_with(|| a.source.cmp(&b.source))
    });
    out
}

pub fn collect_provider_usage() -> Vec<ProviderUsage> {
    vec![
        collect_provider(
            ProviderKind::Claude,
            &["ANTHROPIC_API_KEY"],
            &candidate_claude_roots(),
            3.0,
            15.0,
        ),
        collect_provider(
            ProviderKind::Gemini,
            &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
            &candidate_gemini_roots(),
            1.25,
            5.0,
        ),
        collect_provider(
            ProviderKind::OpenAi,
            &["OPENAI_API_KEY"],
            &candidate_openai_roots(),
            5.0,
            15.0,
        ),
    ]
}

fn collect_provider(
    provider: ProviderKind,
    env_keys: &[&str],
    roots: &[PathBuf],
    price_in_per_million: f64,
    price_out_per_million: f64,
) -> ProviderUsage {
    let mut configured = false;
    let mut config_sources = Vec::new();
    let mut notes = Vec::new();

    for key in env_keys {
        if std::env::var(key).is_ok() {
            configured = true;
            config_sources.push(format!("env:{}", key));
        }
    }

    let mut log_files = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        configured = true;
        config_sources.push(root.to_string_lossy().to_string());
        log_files.extend(find_usage_like_files(root, 3));
    }

    log_files.sort();
    log_files.dedup();

    let mut sessions = 0usize;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut explicit_cost = 0.0f64;

    for file in &log_files {
        let Ok(metadata) = fs::metadata(file) else {
            continue;
        };
        if metadata.len() > 5 * 1024 * 1024 {
            notes.push(format!("skipped large file: {}", file.to_string_lossy()));
            continue;
        }

        let Ok(raw) = fs::read_to_string(file) else {
            continue;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            scan_json_value(
                &value,
                &mut sessions,
                &mut input_tokens,
                &mut output_tokens,
                &mut explicit_cost,
            );
            continue;
        }

        for line in trimmed.lines() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                scan_json_value(
                    &value,
                    &mut sessions,
                    &mut input_tokens,
                    &mut output_tokens,
                    &mut explicit_cost,
                );
            }
        }
    }

    let estimated_from_tokens = (input_tokens as f64 / 1_000_000.0) * price_in_per_million
        + (output_tokens as f64 / 1_000_000.0) * price_out_per_million;

    let estimated_cost_usd = if explicit_cost > 0.0 {
        explicit_cost
    } else {
        estimated_from_tokens
    };

    if !configured {
        notes.push("not configured (no known env/config detected)".to_string());
    }
    if log_files.is_empty() {
        notes.push("no local usage logs found in common paths".to_string());
    }

    ProviderUsage {
        provider,
        configured,
        config_sources,
        sessions,
        total_input_tokens: input_tokens,
        total_output_tokens: output_tokens,
        estimated_cost_usd,
        notes,
    }
}

fn scan_json_value(
    value: &Value,
    sessions: &mut usize,
    input_tokens: &mut u64,
    output_tokens: &mut u64,
    explicit_cost: &mut f64,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                scan_json_value(item, sessions, input_tokens, output_tokens, explicit_cost);
            }
        }
        Value::Object(map) => {
            *sessions += 1;
            for (k, v) in map {
                let key = k.to_ascii_lowercase();
                if key.contains("input") && key.contains("token") {
                    if let Some(n) = v.as_u64() {
                        *input_tokens = input_tokens.saturating_add(n);
                    }
                } else if (key.contains("output") || key.contains("completion"))
                    && key.contains("token")
                {
                    if let Some(n) = v.as_u64() {
                        *output_tokens = output_tokens.saturating_add(n);
                    }
                } else if key == "cost"
                    || key == "usd"
                    || key == "amount"
                    || key == "total_cost"
                    || key == "estimated_cost_usd"
                {
                    if let Some(n) = v.as_f64() {
                        *explicit_cost += n;
                    }
                }

                scan_json_value(v, sessions, input_tokens, output_tokens, explicit_cost);
            }
        }
        _ => {}
    }
}

fn extract_mcp_servers(value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for top_key in ["mcpServers", "servers"] {
        let Some(obj) = value.get(top_key).and_then(|v| v.as_object()) else {
            continue;
        };

        for (name, cfg) in obj {
            let mut command = cfg
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if command.is_empty() {
                if let Some(url) = cfg.get("url").and_then(|v| v.as_str()) {
                    command = url.to_string();
                }
            }

            if !command.is_empty() {
                if let Some(args) = cfg.get("args").and_then(|v| v.as_array()) {
                    let suffix = args
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .join(" ");
                    if !suffix.is_empty() {
                        command = format!("{} {}", command, suffix);
                    }
                }
                out.push((name.clone(), command));
            }
        }
    }

    out
}

fn check_server_command(command: &str) -> (bool, String, String) {
    if command.starts_with("http://") || command.starts_with("https://") {
        return (
            true,
            "remote endpoint configured".to_string(),
            command.to_string(),
        );
    }

    let binary = command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    if binary.is_empty() {
        return (false, "missing command".to_string(), String::new());
    }

    if Path::new(&binary).is_absolute() {
        if Path::new(&binary).exists() {
            return (true, "binary path exists".to_string(), binary);
        }
        return (false, "binary path does not exist".to_string(), binary);
    }

    match Command::new("sh")
        .args(["-lc", &format!("command -v {}", binary)])
        .output()
    {
        Ok(o) if o.status.success() => {
            let resolved = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (
                true,
                format!("resolved in PATH: {}", resolved),
                binary.to_string(),
            )
        }
        _ => (false, "binary not found in PATH".to_string(), binary),
    }
}

fn find_usage_like_files(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_usage_files(root, 0, max_depth, &mut out);
    out
}

fn walk_usage_files(path: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(v) => v,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let p = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if p.is_dir() {
            if file_name.starts_with('.') && file_name != ".claude" && file_name != ".openai" {
                continue;
            }
            walk_usage_files(&p, depth + 1, max_depth, out);
            continue;
        }

        let lower = file_name.to_ascii_lowercase();
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let usagey = lower.contains("usage")
            || lower.contains("session")
            || lower.contains("cost")
            || lower.contains("billing")
            || lower.contains("events");
        let data_ext = ext == "json" || ext == "jsonl" || ext == "log" || ext == "csv";

        if usagey && data_ext {
            out.push(p);
        }
    }
}

fn home_join(path: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(path))
}

fn candidate_global_mcp_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for p in [
        ".config/claude/claude_desktop_config.json",
        ".claude/claude_desktop_config.json",
        ".cursor/mcp.json",
        ".config/agentpulse/mcp.json",
    ] {
        if let Some(path) = home_join(p) {
            paths.push(path);
        }
    }
    paths
}

fn candidate_claude_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for p in [
        ".claude",
        ".config/claude",
        ".config/claude-code",
        ".anthropic",
    ] {
        if let Some(path) = home_join(p) {
            roots.push(path);
        }
    }
    roots
}

fn candidate_gemini_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for p in [
        ".gemini",
        ".config/gemini",
        ".config/google-gemini",
        ".config/google",
    ] {
        if let Some(path) = home_join(p) {
            roots.push(path);
        }
    }
    roots
}

fn candidate_openai_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for p in [
        ".openai",
        ".config/openai",
        ".config/OpenAI",
        ".cache/openai",
    ] {
        if let Some(path) = home_join(p) {
            roots.push(path);
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_mcp_servers() {
        let raw = r#"{
          "mcpServers": {
            "github": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"]},
            "docs": {"url": "http://localhost:3000/mcp"}
          }
        }"#;
        let value: Value = serde_json::from_str(raw).unwrap();
        let servers = extract_mcp_servers(&value);
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().any(|(n, _)| n == "github"));
    }

    #[test]
    fn check_remote_endpoint_is_healthy() {
        let (healthy, _, _) = check_server_command("https://example.com/mcp");
        assert!(healthy);
    }
}
