use crate::collectors::{collect_all, CollectorOutput};
use crate::dashboard::models::{
    ActionCommand, ActionKind, DashboardAlert, DashboardSnapshot, OverviewMetrics, ProviderKind,
};
use crate::git::Repo;
use std::collections::HashSet;

pub fn collect_and_build(repos: &[Repo]) -> DashboardSnapshot {
    let collected = collect_all(repos);
    build_snapshot(repos, collected)
}

pub fn build_snapshot(repos: &[Repo], mut collected: CollectorOutput) -> DashboardSnapshot {
    let total_repos = repos.len();
    let actionable_repos = repos.iter().filter(|r| r.needs_attention()).count();
    let dirty_repos = repos
        .iter()
        .filter(|r| r.status.uncommitted_count > 0)
        .count();
    let repos_ahead = repos.iter().filter(|r| r.status.unpushed_count > 0).count();
    let repos_behind = repos.iter().filter(|r| r.status.behind_count > 0).count();
    let total_worktrees = collected.worktrees.len();
    let repo_processes = collected.processes.len();

    let env_issues = collected
        .env_audit
        .iter()
        .filter(|e| !e.missing_keys.is_empty() || !e.tracked_secret_files.is_empty())
        .count();
    let dep_issues = collected
        .dependencies
        .iter()
        .filter(|d| d.issue_count > 0)
        .count();
    let mcp_unhealthy = collected.mcp_servers.iter().filter(|m| !m.healthy).count();

    collected.alerts.extend(build_system_alerts(&collected));
    dedupe_alerts(&mut collected.alerts);
    collected.alerts.sort_by(|a, b| {
        severity_rank(&b.severity)
            .cmp(&severity_rank(&a.severity))
            .then_with(|| b.action.is_some().cmp(&a.action.is_some()))
            .then_with(|| a.title.cmp(&b.title))
    });
    collected.alerts.truncate(120);

    let mut providers = collected.providers;
    providers.sort_by(|a, b| {
        provider_rank(a.provider)
            .cmp(&provider_rank(b.provider))
            .then_with(|| b.estimated_cost_usd.total_cmp(&a.estimated_cost_usd))
    });

    DashboardSnapshot {
        generated_at_epoch_secs: chrono::Utc::now().timestamp(),
        overview: OverviewMetrics {
            total_repos,
            actionable_repos,
            dirty_repos,
            repos_ahead,
            repos_behind,
            total_worktrees,
            repo_processes,
            env_issues,
            dep_issues,
            mcp_unhealthy,
        },
        alerts: collected.alerts,
        repos: collected.repos,
        worktrees: collected.worktrees,
        processes: collected.processes,
        dependencies: collected.dependencies,
        env_audit: collected.env_audit,
        mcp_servers: collected.mcp_servers,
        providers,
    }
}

fn build_system_alerts(collected: &CollectorOutput) -> Vec<DashboardAlert> {
    let mut alerts = Vec::new();

    let dep_issues = collected
        .dependencies
        .iter()
        .filter(|d| d.issue_count > 0)
        .count();
    if dep_issues > 0 {
        alerts.push(DashboardAlert {
            severity: "warn".to_string(),
            title: "Dependency hygiene issues detected".to_string(),
            detail: format!("{} repo(s) with dependency issues", dep_issues),
            repo: None,
            action: Some(ActionCommand::new(
                "open dependency view",
                ActionKind::ShowMessage {
                    message: "Switch to Deps section in AgentPulse".to_string(),
                },
            )),
        });
    }

    let env_risky = collected
        .env_audit
        .iter()
        .filter(|e| !e.tracked_secret_files.is_empty())
        .count();
    if env_risky > 0 {
        alerts.push(DashboardAlert {
            severity: "high".to_string(),
            title: "Tracked env files may contain secrets".to_string(),
            detail: format!("{} repo(s) have tracked sensitive env files", env_risky),
            repo: None,
            action: Some(ActionCommand::new(
                "review env audit",
                ActionKind::ShowMessage {
                    message: "Switch to Env Audit section in AgentPulse".to_string(),
                },
            )),
        });
    }

    let mcp_bad = collected.mcp_servers.iter().filter(|m| !m.healthy).count();
    if mcp_bad > 0 {
        alerts.push(DashboardAlert {
            severity: "warn".to_string(),
            title: "MCP server health issues".to_string(),
            detail: format!("{} MCP server(s) unhealthy", mcp_bad),
            repo: None,
            action: Some(ActionCommand::new(
                "inspect MCP",
                ActionKind::ShowMessage {
                    message: "Switch to MCP Health section in AgentPulse".to_string(),
                },
            )),
        });
    }

    let provider_unconfigured = collected.providers.iter().filter(|p| !p.configured).count();
    if provider_unconfigured > 0 {
        alerts.push(DashboardAlert {
            severity: "info".to_string(),
            title: "AI provider not configured".to_string(),
            detail: format!("{} provider(s) missing config", provider_unconfigured),
            repo: None,
            action: None,
        });
    }

    alerts
}

fn dedupe_alerts(alerts: &mut Vec<DashboardAlert>) {
    let mut seen = HashSet::new();
    alerts.retain(|alert| {
        let key = format!(
            "{}|{}|{}|{}",
            alert.severity,
            alert.title,
            alert.detail,
            alert.repo.as_deref().unwrap_or_default()
        );
        seen.insert(key)
    });
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 4,
        "high" => 3,
        "warn" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn provider_rank(kind: ProviderKind) -> u8 {
    match kind {
        ProviderKind::Claude => 0,
        ProviderKind::Gemini => 1,
        ProviderKind::OpenAi => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_alerts_removes_duplicates() {
        let mut alerts = vec![
            DashboardAlert {
                severity: "warn".to_string(),
                title: "dup".to_string(),
                detail: "same".to_string(),
                repo: Some("r1".to_string()),
                action: None,
            },
            DashboardAlert {
                severity: "warn".to_string(),
                title: "dup".to_string(),
                detail: "same".to_string(),
                repo: Some("r1".to_string()),
                action: None,
            },
        ];
        dedupe_alerts(&mut alerts);
        assert_eq!(alerts.len(), 1);
    }
}
