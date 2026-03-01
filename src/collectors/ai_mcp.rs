use crate::dashboard::{ActionCommand, ActionKind, McpServerHealth, ProviderKind, ProviderUsage};
use crate::git::Repo;
use chrono::{Datelike, Duration as ChronoDuration, TimeZone, Utc};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct ReportWindow {
    start_epoch_secs: i64,
    end_epoch_secs: i64,
    start_rfc3339: String,
    end_rfc3339: String,
    label: String,
}

#[derive(Debug, Clone, Default)]
struct ProviderLiveData {
    sessions: Option<usize>,
    total_input_tokens: Option<u64>,
    total_output_tokens: Option<u64>,
    cost_usd: Option<f64>,
    notes: Vec<String>,
}

type LiveFetchResult = Result<Option<ProviderLiveData>, String>;

#[derive(Clone)]
struct CachedProviderResult {
    fetched_at: Instant,
    window_key: String,
    result: LiveFetchResult,
}

#[derive(Default)]
struct ProviderApiCache {
    claude: Option<CachedProviderResult>,
    gemini: Option<CachedProviderResult>,
    openai: Option<CachedProviderResult>,
}

impl ProviderApiCache {
    fn slot_mut(&mut self, provider: ProviderKind) -> &mut Option<CachedProviderResult> {
        match provider {
            ProviderKind::Claude => &mut self.claude,
            ProviderKind::Gemini => &mut self.gemini,
            ProviderKind::OpenAi => &mut self.openai,
        }
    }
}

static PROVIDER_API_CACHE: OnceLock<Mutex<ProviderApiCache>> = OnceLock::new();

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
                    Some(ActionCommand::new(
                        "probe server",
                        ActionKind::ProbeBinaryHelp {
                            binary: binary.to_string(),
                        },
                    ))
                }
            } else if binary.is_empty() {
                None
            } else {
                Some(ActionCommand::new(
                    "check binary",
                    ActionKind::CheckBinaryInPath {
                        binary: binary.to_string(),
                    },
                ))
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
    let window = report_window();

    vec![
        collect_provider(
            ProviderKind::Claude,
            &["ANTHROPIC_ADMIN_API_KEY", "ANTHROPIC_API_KEY"],
            &candidate_claude_roots(),
            3.0,
            15.0,
            &window,
            Some(fetch_claude_live_data),
        ),
        collect_provider(
            ProviderKind::Gemini,
            &[
                "GEMINI_API_KEY",
                "GOOGLE_API_KEY",
                "AGENTPULSE_GEMINI_BQ_TABLE",
            ],
            &candidate_gemini_roots(),
            1.25,
            5.0,
            &window,
            Some(fetch_gemini_live_data),
        ),
        collect_provider(
            ProviderKind::OpenAi,
            &["OPENAI_ADMIN_KEY", "OPENAI_API_KEY"],
            &candidate_openai_roots(),
            5.0,
            15.0,
            &window,
            Some(fetch_openai_live_data),
        ),
    ]
}

fn collect_provider(
    provider: ProviderKind,
    env_keys: &[&str],
    roots: &[PathBuf],
    price_in_per_million: f64,
    price_out_per_million: f64,
    window: &ReportWindow,
    live_fetch: Option<fn(&ReportWindow) -> LiveFetchResult>,
) -> ProviderUsage {
    let mut configured = false;
    let mut config_sources = Vec::new();
    let mut notes = Vec::new();
    let mut source_updated_at_epoch_secs: i64 = 0;

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
    let mut has_local_data = false;

    for file in &log_files {
        let Ok(metadata) = fs::metadata(file) else {
            continue;
        };
        if let Some(ts) = modified_epoch_secs(&metadata) {
            source_updated_at_epoch_secs = source_updated_at_epoch_secs.max(ts);
        }
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
            has_local_data = true;
            sessions = sessions.saturating_add(estimated_sessions(&value));
            scan_json_value(
                &value,
                &mut input_tokens,
                &mut output_tokens,
                &mut explicit_cost,
            );
            continue;
        }

        for line in trimmed.lines() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                has_local_data = true;
                sessions = sessions.saturating_add(1);
                scan_json_value(
                    &value,
                    &mut input_tokens,
                    &mut output_tokens,
                    &mut explicit_cost,
                );
            }
        }
    }

    // Merge supplementary local data sources that the generic log scan misses.
    match provider {
        ProviderKind::Claude => {
            if let Some((s, i, o, c, n)) = collect_claude_code_stats() {
                configured = true;
                has_local_data = true;
                // stats-cache.json is a superset — use whichever is larger.
                sessions = sessions.max(s);
                input_tokens = input_tokens.max(i);
                output_tokens = output_tokens.max(o);
                if c > explicit_cost {
                    explicit_cost = c;
                }
                notes.extend(n);
            }
        }
        ProviderKind::OpenAi => {
            if let Some((s, i, o, _c, n)) = collect_codex_session_usage() {
                configured = true;
                has_local_data = true;
                // Codex data is separate from OpenAI API data — add it.
                sessions = sessions.saturating_add(s);
                input_tokens = input_tokens.saturating_add(i);
                output_tokens = output_tokens.saturating_add(o);
                notes.extend(n);
            }
        }
        _ => {}
    }

    let estimated_from_tokens = (input_tokens as f64 / 1_000_000.0) * price_in_per_million
        + (output_tokens as f64 / 1_000_000.0) * price_out_per_million;

    let mut estimated_cost_usd = if explicit_cost > 0.0 {
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

    let mut data_source = if !configured {
        "unconfigured".to_string()
    } else if has_local_data {
        "local_logs".to_string()
    } else {
        "heuristic".to_string()
    };

    if let Some(fetch) = live_fetch {
        match fetch_live_data_cached(provider, window, fetch) {
            Ok(Some(live)) => {
                notes.retain(|n| n != "no local usage logs found in common paths");
                if let Some(n) = live.sessions {
                    sessions = n;
                }
                if let Some(n) = live.total_input_tokens {
                    input_tokens = n;
                }
                if let Some(n) = live.total_output_tokens {
                    output_tokens = n;
                }
                if let Some(v) = live.cost_usd {
                    estimated_cost_usd = v;
                }
                notes.extend(live.notes);
                notes.push(format!("live provider window: {}", window.label));
                data_source = "live".to_string();
                source_updated_at_epoch_secs = Utc::now().timestamp();
            }
            Ok(None) => {}
            Err(e) => notes.push(format!("live provider query failed: {}", e)),
        }
    }

    if source_updated_at_epoch_secs == 0 {
        source_updated_at_epoch_secs = Utc::now().timestamp();
    }

    ProviderUsage {
        provider,
        configured,
        config_sources,
        data_source,
        source_updated_at_epoch_secs,
        sessions,
        total_input_tokens: input_tokens,
        total_output_tokens: output_tokens,
        estimated_cost_usd,
        notes,
    }
}

fn fetch_live_data_cached(
    provider: ProviderKind,
    window: &ReportWindow,
    fetch: fn(&ReportWindow) -> LiveFetchResult,
) -> LiveFetchResult {
    let cache = PROVIDER_API_CACHE.get_or_init(|| Mutex::new(ProviderApiCache::default()));
    let ttl = Duration::from_secs(read_env_u64("AGENTPULSE_PROVIDER_CACHE_SECS", 60));
    let key = format!("{}:{}", window.start_epoch_secs, window.end_epoch_secs);

    if let Ok(mut guard) = cache.lock() {
        if let Some(entry) = guard.slot_mut(provider).as_ref() {
            if entry.window_key == key && entry.fetched_at.elapsed() < ttl {
                return entry.result.clone();
            }
        }
    }

    let fresh = fetch(window);

    if let Ok(mut guard) = cache.lock() {
        *guard.slot_mut(provider) = Some(CachedProviderResult {
            fetched_at: Instant::now(),
            window_key: key,
            result: fresh.clone(),
        });
    }

    fresh
}

fn report_window() -> ReportWindow {
    let now = Utc::now();
    let days = read_env_i64("AGENTPULSE_COST_LOOKBACK_DAYS", 0);
    let start = if days > 0 {
        now - ChronoDuration::days(days)
    } else {
        Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
            .single()
            .unwrap_or(now - ChronoDuration::days(30))
    };

    let label = if days > 0 {
        format!("last {} days", days)
    } else {
        "month-to-date".to_string()
    };

    ReportWindow {
        start_epoch_secs: start.timestamp(),
        end_epoch_secs: now.timestamp(),
        start_rfc3339: start.to_rfc3339(),
        end_rfc3339: now.to_rfc3339(),
        label,
    }
}

fn fetch_openai_live_data(window: &ReportWindow) -> LiveFetchResult {
    let Some(api_key) = first_env_value(&["OPENAI_ADMIN_KEY", "OPENAI_API_KEY"]) else {
        return Ok(None);
    };

    let mut page: Option<String> = None;
    let mut pages = 0usize;
    let mut sessions = 0usize;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    loop {
        let mut params = vec![
            ("start_time", window.start_epoch_secs.to_string()),
            ("end_time", window.end_epoch_secs.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
        ];
        if let Some(cursor) = page.as_ref() {
            params.push(("page", cursor.clone()));
        }

        let value = http_get_json(
            "https://api.openai.com/v1/organization/usage/completions",
            &[
                ("Accept", "application/json".to_string()),
                ("Content-Type", "application/json".to_string()),
                ("Authorization", format!("Bearer {}", api_key)),
            ],
            &params,
        )?;

        accumulate_openai_usage_metrics(
            &value,
            &mut sessions,
            &mut input_tokens,
            &mut output_tokens,
        );

        pages += 1;
        if pages >= read_env_usize("AGENTPULSE_PROVIDER_MAX_PAGES", 6) {
            break;
        }

        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            break;
        }
        page = value
            .get("next_page")
            .and_then(Value::as_str)
            .map(str::to_string);
        if page.is_none() {
            break;
        }
    }

    let mut page: Option<String> = None;
    let mut pages = 0usize;
    let mut cost_usd = 0.0f64;
    loop {
        let mut params = vec![
            ("start_time", window.start_epoch_secs.to_string()),
            ("end_time", window.end_epoch_secs.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
        ];
        if let Some(cursor) = page.as_ref() {
            params.push(("page", cursor.clone()));
        }

        let value = http_get_json(
            "https://api.openai.com/v1/organization/costs",
            &[
                ("Accept", "application/json".to_string()),
                ("Content-Type", "application/json".to_string()),
                ("Authorization", format!("Bearer {}", api_key)),
            ],
            &params,
        )?;

        accumulate_openai_cost(&value, &mut cost_usd);

        pages += 1;
        if pages >= read_env_usize("AGENTPULSE_PROVIDER_MAX_PAGES", 6) {
            break;
        }

        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            break;
        }
        page = value
            .get("next_page")
            .and_then(Value::as_str)
            .map(str::to_string);
        if page.is_none() {
            break;
        }
    }

    Ok(Some(ProviderLiveData {
        sessions: Some(sessions),
        total_input_tokens: Some(input_tokens),
        total_output_tokens: Some(output_tokens),
        cost_usd: Some(cost_usd),
        notes: vec!["source: OpenAI org usage/cost APIs".to_string()],
    }))
}

fn fetch_claude_live_data(window: &ReportWindow) -> LiveFetchResult {
    let Some(api_key) = first_env_value(&["ANTHROPIC_ADMIN_API_KEY", "ANTHROPIC_API_KEY"]) else {
        return Ok(None);
    };

    let mut page: Option<String> = None;
    let mut pages = 0usize;
    let mut sessions = 0usize;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    loop {
        let mut params = vec![
            ("starting_at", window.start_rfc3339.clone()),
            ("ending_at", window.end_rfc3339.clone()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
        ];
        if let Some(cursor) = page.as_ref() {
            params.push(("page", cursor.clone()));
        }

        let value = http_get_json(
            "https://api.anthropic.com/v1/organizations/usage_report/messages",
            &[
                ("Accept", "application/json".to_string()),
                ("x-api-key", api_key.clone()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
            &params,
        )?;

        accumulate_claude_usage_metrics(
            &value,
            &mut sessions,
            &mut input_tokens,
            &mut output_tokens,
        );

        pages += 1;
        if pages >= read_env_usize("AGENTPULSE_PROVIDER_MAX_PAGES", 6) {
            break;
        }

        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            break;
        }
        page = value
            .get("next_page")
            .and_then(Value::as_str)
            .map(str::to_string);
        if page.is_none() {
            break;
        }
    }

    let mut page: Option<String> = None;
    let mut pages = 0usize;
    let mut cost_usd = 0.0f64;
    loop {
        let mut params = vec![
            ("starting_at", window.start_rfc3339.clone()),
            ("ending_at", window.end_rfc3339.clone()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
        ];
        if let Some(cursor) = page.as_ref() {
            params.push(("page", cursor.clone()));
        }

        let value = http_get_json(
            "https://api.anthropic.com/v1/organizations/cost_report",
            &[
                ("Accept", "application/json".to_string()),
                ("x-api-key", api_key.clone()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
            &params,
        )?;

        accumulate_claude_cost(&value, &mut cost_usd);

        pages += 1;
        if pages >= read_env_usize("AGENTPULSE_PROVIDER_MAX_PAGES", 6) {
            break;
        }

        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            break;
        }
        page = value
            .get("next_page")
            .and_then(Value::as_str)
            .map(str::to_string);
        if page.is_none() {
            break;
        }
    }

    Ok(Some(ProviderLiveData {
        sessions: Some(sessions),
        total_input_tokens: Some(input_tokens),
        total_output_tokens: Some(output_tokens),
        cost_usd: Some(cost_usd),
        notes: vec![
            "source: Anthropic usage/cost report APIs".to_string(),
            "session count uses request fields when available".to_string(),
        ],
    }))
}

fn fetch_gemini_live_data(window: &ReportWindow) -> LiveFetchResult {
    let Some(table) = std::env::var("AGENTPULSE_GEMINI_BQ_TABLE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    else {
        return Ok(None);
    };

    if !is_safe_bq_table_identifier(&table) {
        return Err(
            "AGENTPULSE_GEMINI_BQ_TABLE must be project.dataset.table using only [A-Za-z0-9_.-]"
                .to_string(),
        );
    }

    let service_filter = std::env::var("AGENTPULSE_GEMINI_BQ_SERVICE_FILTER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            "LOWER(service.description) LIKE '%generative language%' \
             OR LOWER(service.description) LIKE '%vertex ai%' \
             OR LOWER(sku.description) LIKE '%gemini%'"
                .to_string()
        });

    let start = Utc
        .timestamp_opt(window.start_epoch_secs, 0)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339();
    let end = Utc
        .timestamp_opt(window.end_epoch_secs, 0)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339();

    let sql = format!(
        "SELECT COALESCE(SUM(cost), 0) AS total_cost_usd \
         FROM `{table}` \
         WHERE usage_start_time >= TIMESTAMP('{start}') \
           AND usage_start_time < TIMESTAMP('{end}') \
           AND ({service_filter})"
    );

    let output = Command::new("bq")
        .args(["query", "--use_legacy_sql=false", "--format=json", &sql])
        .output()
        .map_err(|e| format!("failed to run bq CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("bq query failed with exit code {:?}", output.status.code())
        } else {
            format!("bq query failed: {}", stderr)
        });
    }

    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("invalid bq json output: {}", e))?;

    let mut cost_usd = 0.0f64;
    if let Some(rows) = value.as_array() {
        for row in rows {
            if let Some(v) = row.get("total_cost_usd") {
                cost_usd += value_as_f64(v).unwrap_or(0.0);
            }
        }
    }

    Ok(Some(ProviderLiveData {
        sessions: None,
        total_input_tokens: None,
        total_output_tokens: None,
        cost_usd: Some(cost_usd),
        notes: vec![
            format!("source: BigQuery billing export ({})", table),
            "Gemini API does not expose org usage/cost endpoint; using billing export totals"
                .to_string(),
        ],
    }))
}

fn http_get_json(
    url: &str,
    headers: &[(&str, String)],
    query_params: &[(&str, String)],
) -> Result<Value, String> {
    let timeout_secs = read_env_u64("AGENTPULSE_PROVIDER_TIMEOUT_SECS", 8);
    let mut full_url = url.to_string();
    if !query_params.is_empty() {
        full_url.push('?');
        let joined = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode_component(k), url_encode_component(v)))
            .collect::<Vec<String>>()
            .join("&");
        full_url.push_str(&joined);
    }

    let mut cmd = Command::new("curl");
    cmd.arg("--silent")
        .arg("--show-error")
        .arg("--fail-with-body")
        .arg("--connect-timeout")
        .arg(timeout_secs.min(4).to_string())
        .arg("--max-time")
        .arg(timeout_secs.to_string());

    for (k, v) in headers {
        cmd.arg("-H").arg(format!("{}: {}", k, v));
    }
    cmd.arg(&full_url);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run curl for {}: {}", url, e))?;
    if !output.status.success() {
        let mut detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if detail.is_empty() {
            detail = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        let compact = detail.split_whitespace().collect::<Vec<_>>().join(" ");
        return Err(format!(
            "curl request failed for {}: {}",
            url,
            compact
                .trim()
                .trim_end_matches('.')
                .chars()
                .take(400)
                .collect::<String>()
        ));
    }

    serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|e| format!("failed to parse provider response json: {}", e))
}

fn url_encode_component(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn accumulate_openai_usage_metrics(
    value: &Value,
    sessions: &mut usize,
    input_tokens: &mut u64,
    output_tokens: &mut u64,
) {
    if let Some(buckets) = value.get("data").and_then(Value::as_array) {
        for bucket in buckets {
            if let Some(results) = bucket.get("results").and_then(Value::as_array) {
                for result in results {
                    *sessions = sessions.saturating_add(
                        result
                            .get("num_model_requests")
                            .and_then(value_as_u64)
                            .unwrap_or(0) as usize,
                    );
                    *input_tokens = input_tokens.saturating_add(
                        result
                            .get("input_tokens")
                            .and_then(value_as_u64)
                            .unwrap_or(0),
                    );
                    *output_tokens = output_tokens.saturating_add(
                        result
                            .get("output_tokens")
                            .and_then(value_as_u64)
                            .unwrap_or(0),
                    );
                }
            }
        }
    }
}

fn accumulate_openai_cost(value: &Value, cost_usd: &mut f64) {
    if let Some(buckets) = value.get("data").and_then(Value::as_array) {
        for bucket in buckets {
            if let Some(results) = bucket.get("results").and_then(Value::as_array) {
                for result in results {
                    if let Some(amount) = result.get("amount").and_then(Value::as_object) {
                        if let Some(v) = amount.get("value") {
                            *cost_usd += value_as_f64(v).unwrap_or(0.0);
                        }
                    }
                }
            }
        }
    }
}

fn accumulate_claude_usage_metrics(
    value: &Value,
    sessions: &mut usize,
    input_tokens: &mut u64,
    output_tokens: &mut u64,
) {
    if let Some(buckets) = value.get("data").and_then(Value::as_array) {
        for bucket in buckets {
            if let Some(results) = bucket.get("results").and_then(Value::as_array) {
                for result in results {
                    *sessions = sessions.saturating_add(
                        result
                            .get("request_count")
                            .and_then(value_as_u64)
                            .or_else(|| result.get("requests").and_then(value_as_u64))
                            .or_else(|| result.get("num_model_requests").and_then(value_as_u64))
                            .unwrap_or(0) as usize,
                    );

                    let uncached = result
                        .get("uncached_input_tokens")
                        .and_then(value_as_u64)
                        .unwrap_or(0);
                    let cache_read = result
                        .get("cache_read_input_tokens")
                        .and_then(value_as_u64)
                        .unwrap_or(0);
                    let cache_creation = result
                        .get("cache_creation")
                        .and_then(Value::as_object)
                        .map(|cache| {
                            cache
                                .get("ephemeral_5m_input_tokens")
                                .and_then(value_as_u64)
                                .unwrap_or(0)
                                .saturating_add(
                                    cache
                                        .get("ephemeral_1h_input_tokens")
                                        .and_then(value_as_u64)
                                        .unwrap_or(0),
                                )
                        })
                        .unwrap_or(0);
                    let output = result
                        .get("output_tokens")
                        .and_then(value_as_u64)
                        .unwrap_or(0);

                    *input_tokens = input_tokens.saturating_add(
                        uncached
                            .saturating_add(cache_read)
                            .saturating_add(cache_creation),
                    );
                    *output_tokens = output_tokens.saturating_add(output);
                }
            }
        }
    }
}

fn accumulate_claude_cost(value: &Value, cost_usd: &mut f64) {
    if let Some(buckets) = value.get("data").and_then(Value::as_array) {
        for bucket in buckets {
            if let Some(results) = bucket.get("results").and_then(Value::as_array) {
                for result in results {
                    if let Some(amount) = result.get("amount") {
                        let cents = value_as_f64(amount).unwrap_or(0.0);
                        *cost_usd += cents / 100.0;
                    }
                }
            }
        }
    }
}

fn value_as_u64(v: &Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(n) = v.as_i64() {
        return Some(n.max(0) as u64);
    }
    if let Some(s) = v.as_str() {
        if let Ok(parsed) = s.parse::<u64>() {
            return Some(parsed);
        }
        if let Ok(parsed) = s.parse::<f64>() {
            return Some(parsed.max(0.0).round() as u64);
        }
    }
    v.as_f64().map(|n| n.max(0.0).round() as u64)
}

fn value_as_f64(v: &Value) -> Option<f64> {
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        return s.parse::<f64>().ok();
    }
    if let Some(n) = v.as_i64() {
        return Some(n as f64);
    }
    v.as_u64().map(|n| n as f64)
}

fn first_env_value(keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| std::env::var(k).ok())
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

fn read_env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default_value)
}

fn read_env_usize(key: &str, default_value: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default_value)
}

fn read_env_i64(key: &str, default_value: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v >= 0)
        .unwrap_or(default_value)
}

fn is_safe_bq_table_identifier(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.matches('.').count() < 2 {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

fn modified_epoch_secs(metadata: &fs::Metadata) -> Option<i64> {
    let modified = metadata.modified().ok()?;
    let ts = chrono::DateTime::<Utc>::from(modified).timestamp();
    Some(ts)
}

fn estimated_sessions(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.len().max(1),
        Value::Object(_) => 1,
        _ => 0,
    }
}

fn scan_json_value(
    value: &Value,
    input_tokens: &mut u64,
    output_tokens: &mut u64,
    explicit_cost: &mut f64,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                scan_json_value(item, input_tokens, output_tokens, explicit_cost);
            }
        }
        Value::Object(map) => {
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

                scan_json_value(v, input_tokens, output_tokens, explicit_cost);
            }
        }
        _ => {}
    }
}

/// Parse `~/.claude/stats-cache.json` for pre-aggregated Claude Code usage.
/// Returns (sessions, input_tokens, output_tokens, cost_usd, notes).
fn collect_claude_code_stats() -> Option<(usize, u64, u64, f64, Vec<String>)> {
    let path = home_join(".claude/stats-cache.json")?;
    let raw = fs::read_to_string(&path).ok()?;
    let value: Value = serde_json::from_str(raw.trim()).ok()?;

    let sessions = value
        .get("totalSessions")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize;

    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cost_usd = 0.0f64;

    if let Some(model_usage) = value.get("modelUsage").and_then(Value::as_object) {
        for (_model, usage) in model_usage {
            let input = usage.get("inputTokens").and_then(value_as_u64).unwrap_or(0);
            let cache_read = usage
                .get("cacheReadInputTokens")
                .and_then(value_as_u64)
                .unwrap_or(0);
            let cache_creation = usage
                .get("cacheCreationInputTokens")
                .and_then(value_as_u64)
                .unwrap_or(0);
            let output = usage
                .get("outputTokens")
                .and_then(value_as_u64)
                .unwrap_or(0);
            let cost = value_as_f64(usage.get("costUSD").unwrap_or(&Value::Null)).unwrap_or(0.0);

            input_tokens = input_tokens
                .saturating_add(input)
                .saturating_add(cache_read)
                .saturating_add(cache_creation);
            output_tokens = output_tokens.saturating_add(output);
            cost_usd += cost;
        }
    }

    if sessions == 0 && input_tokens == 0 && output_tokens == 0 {
        return None;
    }

    Some((
        sessions,
        input_tokens,
        output_tokens,
        cost_usd,
        vec![format!("source: {}", path.to_string_lossy())],
    ))
}

/// Walk `~/.codex/sessions/` recursively for `.jsonl` files containing
/// `payload.type = "token_count"` entries. Each session file's LAST such
/// entry gives cumulative token usage for that session.
/// Returns (sessions, input_tokens, output_tokens, cost_usd=0, notes).
fn collect_codex_session_usage() -> Option<(usize, u64, u64, f64, Vec<String>)> {
    let sessions_dir = home_join(".codex/sessions")?;
    if !sessions_dir.is_dir() {
        return None;
    }

    let mut session_files = Vec::new();
    collect_jsonl_files_recursive(&sessions_dir, 0, 6, &mut session_files);

    if session_files.is_empty() {
        return None;
    }

    let mut total_sessions = 0usize;
    let mut total_input = 0u64;
    let mut total_output = 0u64;

    for file in &session_files {
        let Ok(raw) = fs::read_to_string(file) else {
            continue;
        };

        // Find the last token_count entry in this session file.
        let mut last_input = 0u64;
        let mut last_output = 0u64;
        let mut found = false;

        for line in raw.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };

            let is_token_count = value
                .get("payload")
                .and_then(|p| p.get("type"))
                .and_then(Value::as_str)
                == Some("token_count");

            if !is_token_count {
                continue;
            }

            found = true;
            if let Some(usage) = value
                .get("payload")
                .and_then(|p| p.get("total_token_usage"))
            {
                last_input = usage
                    .get("input_tokens")
                    .and_then(value_as_u64)
                    .unwrap_or(0);
                last_output = usage
                    .get("output_tokens")
                    .and_then(value_as_u64)
                    .unwrap_or(0);
            }
        }

        if found {
            total_sessions += 1;
            total_input = total_input.saturating_add(last_input);
            total_output = total_output.saturating_add(last_output);
        }
    }

    if total_sessions == 0 {
        return None;
    }

    Some((
        total_sessions,
        total_input,
        total_output,
        0.0,
        vec![format!("source: {}", sessions_dir.to_string_lossy())],
    ))
}

fn collect_jsonl_files_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
) {
    if depth > max_depth {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_jsonl_files_recursive(&p, depth + 1, max_depth, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            out.push(p);
        }
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

    if let Some(resolved) = resolve_binary_in_path(&binary) {
        (
            true,
            format!("resolved in PATH: {}", resolved.display()),
            binary.to_string(),
        )
    } else {
        (false, "binary not found in PATH".to_string(), binary)
    }
}

fn resolve_binary_in_path(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists() && candidate.is_file())
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
            if file_name.starts_with('.')
                && file_name != ".claude"
                && file_name != ".openai"
                && file_name != ".codex"
            {
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
        ".codex",
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

    #[test]
    fn accumulates_openai_usage_and_cost_payloads() {
        let usage: Value = serde_json::from_str(
            r#"{
              "data": [
                {
                  "results": [
                    { "input_tokens": 1200, "output_tokens": 3400, "num_model_requests": 7 },
                    { "input_tokens": 10, "output_tokens": 20, "num_model_requests": 1 }
                  ]
                }
              ]
            }"#,
        )
        .unwrap();

        let mut sessions = 0usize;
        let mut in_tokens = 0u64;
        let mut out_tokens = 0u64;
        accumulate_openai_usage_metrics(&usage, &mut sessions, &mut in_tokens, &mut out_tokens);

        assert_eq!(sessions, 8);
        assert_eq!(in_tokens, 1210);
        assert_eq!(out_tokens, 3420);

        let cost: Value = serde_json::from_str(
            r#"{
              "data": [
                {
                  "results": [
                    { "amount": { "value": 1.25, "currency": "usd" } },
                    { "amount": { "value": "0.75", "currency": "usd" } }
                  ]
                }
              ]
            }"#,
        )
        .unwrap();
        let mut usd = 0.0;
        accumulate_openai_cost(&cost, &mut usd);
        assert!((usd - 2.0).abs() < 0.0001);
    }

    #[test]
    fn accumulates_claude_cost_from_cents() {
        let cost: Value = serde_json::from_str(
            r#"{
              "data": [
                { "results": [ { "amount": "123.45" }, { "amount": 50 } ] }
              ]
            }"#,
        )
        .unwrap();
        let mut usd = 0.0;
        accumulate_claude_cost(&cost, &mut usd);
        assert!((usd - 1.7345).abs() < 0.0001);
    }

    #[test]
    fn validates_bq_identifier_shape() {
        assert!(is_safe_bq_table_identifier(
            "my-project.billing_export.gcp_billing_export_v1_abc"
        ));
        assert!(!is_safe_bq_table_identifier("bad table name"));
        assert!(!is_safe_bq_table_identifier("only.one"));
    }

    #[test]
    fn url_encodes_query_values() {
        assert_eq!(
            url_encode_component("2026-02-23T12:30:00+00:00"),
            "2026-02-23T12%3A30%3A00%2B00%3A00"
        );
    }

    #[test]
    fn estimated_sessions_for_json_shapes() {
        let obj: Value = serde_json::json!({"a":1});
        let arr: Value = serde_json::json!([{"a":1}, {"b":2}]);
        assert_eq!(estimated_sessions(&obj), 1);
        assert_eq!(estimated_sessions(&arr), 2);
    }
}
