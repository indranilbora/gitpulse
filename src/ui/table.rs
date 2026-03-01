use super::{theme, widgets};
use crate::agent;
use crate::app::App;
use crate::dashboard::DashboardSection;
use crate::git::{Repo, StatusColor};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Cell, Paragraph, Row, Wrap},
    Frame,
};

// ─── grouping helpers (repos section) ──────────────────────────────────────

/// A display entry: either a non-selectable group header or a repo row.
enum Entry<'a> {
    Group(String),
    Repo(&'a Repo),
}

/// Build the flat list of entries (group headers interleaved with repos).
/// Returns `(entries, visual_index_of_each_repo_in_filtered_order)`.
fn build_entries<'a>(repos: &[&'a Repo], grouped: bool) -> (Vec<Entry<'a>>, Vec<usize>) {
    if !grouped {
        let visual: Vec<usize> = (0..repos.len()).collect();
        return (repos.iter().map(|r| Entry::Repo(r)).collect(), visual);
    }

    let home = dirs::home_dir().unwrap_or_default();
    let mut entries: Vec<Entry<'a>> = Vec::new();
    let mut visual: Vec<usize> = Vec::new();
    let mut current_parent: Option<String> = None;

    for repo in repos {
        let parent = repo
            .path
            .parent()
            .map(|p| {
                let s = p.to_string_lossy();
                if let Some(rest) = s.strip_prefix(&*home.to_string_lossy()) {
                    format!("~{}", rest)
                } else {
                    s.into_owned()
                }
            })
            .unwrap_or_default();

        if current_parent.as_deref() != Some(&parent) {
            entries.push(Entry::Group(parent.clone()));
            current_parent = Some(parent);
        }

        visual.push(entries.len());
        entries.push(Entry::Repo(repo));
    }

    (entries, visual)
}

// ─── top-level render ───────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = if area.height >= 8 {
        Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area)
    } else {
        Layout::vertical([Constraint::Fill(1)]).split(area)
    };
    let main = chunks[0];

    match app.section {
        DashboardSection::Home => {} // handled by home.rs
        DashboardSection::Repos => render_repos(frame, app, main),
        DashboardSection::Worktrees => render_worktrees(frame, app, main),
        DashboardSection::Processes => render_processes(frame, app, main),
        DashboardSection::Dependencies => render_dependencies(frame, app, main),
        DashboardSection::EnvAudit => render_env_audit(frame, app, main),
        DashboardSection::McpHealth => render_mcp(frame, app, main),
        DashboardSection::AiCosts => render_ai_costs(frame, app, main),
    }

    if chunks.len() > 1 {
        render_selected_detail(frame, app, chunks[1]);
    }
}

fn render_repos(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_repos();

    if filtered.is_empty() {
        let msg = if app.filter_text.is_empty() {
            "No repositories found — run `agentpulse --setup` to configure watch directories"
                .to_string()
        } else {
            format!("No repos matching \"{}\"", app.filter_text)
        };
        widgets::render_empty_state(frame, area, "◇", &msg);
        return;
    }

    let (entries, repo_to_visual) = build_entries(&filtered, app.group_by_dir);

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("NAME"),
        Cell::from("BRANCH"),
        Cell::from("DIRTY"),
        Cell::from("SYNC"),
        Cell::from("STASH"),
        Cell::from("NEXT"),
    ])
    .style(theme::style_header())
    .height(1);

    let mut data_row_idx: usize = 0;
    let rows: Vec<Row> = entries
        .iter()
        .map(|entry| match entry {
            Entry::Group(name) => Row::new(vec![
                Cell::from(""),
                Cell::from(format!(" {}", name)),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(
                Style::default()
                    .fg(theme::ACCENT_PURPLE)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),

            Entry::Repo(repo) => {
                let (indicator, color) = match repo.status_color() {
                    StatusColor::Clean => ("○", theme::ACCENT_GREEN),
                    StatusColor::Uncommitted => ("●", theme::ACCENT_YELLOW),
                    StatusColor::Unpushed => ("●", theme::ACCENT_BLUE),
                    StatusColor::Dirty => ("●", theme::ACCENT_RED),
                    StatusColor::NoRemote => ("○", theme::FG_DIMMED),
                };

                let dirty = if repo.status.uncommitted_count > 0 {
                    if repo.status.uncommitted_count == 1 {
                        "1 file".to_string()
                    } else {
                        format!("{} files", repo.status.uncommitted_count)
                    }
                } else {
                    "—".to_string()
                };

                let sync = if !repo.status.has_remote {
                    "n/a".to_string()
                } else {
                    let ahead = repo.status.unpushed_count;
                    let behind = repo.status.behind_count;
                    match (ahead, behind) {
                        (0, 0) => "—".to_string(),
                        (a, 0) => format!("↑{}", a),
                        (0, b) => format!("↓{}", b),
                        (a, b) => format!("↑{} ↓{}", a, b),
                    }
                };

                let stash = if repo.status.stash_count > 0 {
                    format!("⚑{}", repo.status.stash_count)
                } else {
                    String::new()
                };
                let rec = agent::recommend(repo);
                let next = if rec.short_action == "noop" {
                    "—".to_string()
                } else {
                    rec.short_action.to_string()
                };

                let (branch_text, branch_style) = if repo.status.is_detached {
                    (
                        "(detached)".to_string(),
                        Style::default().fg(theme::FG_DIMMED),
                    )
                } else {
                    (
                        repo.status.branch.clone(),
                        Style::default().fg(theme::FG_PRIMARY),
                    )
                };

                let rec_color = match rec.short_action {
                    "commit" | "add+commit" => theme::ACCENT_YELLOW,
                    "push" => theme::ACCENT_BLUE,
                    "pull" | "fetch+pull" => theme::ACCENT_CYAN,
                    "stash-or-commit" => theme::ACCENT_ORANGE,
                    _ => theme::ACCENT_CYAN,
                };

                let row = Row::new(vec![
                    Cell::from(indicator).style(Style::default().fg(color)),
                    Cell::from(repo.name.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                    Cell::from(branch_text).style(branch_style),
                    Cell::from(dirty).style(Style::default().fg(theme::FG_PRIMARY)),
                    Cell::from(sync).style(Style::default().fg(theme::FG_PRIMARY)),
                    Cell::from(stash).style(Style::default().fg(theme::ACCENT_PINK)),
                    Cell::from(next).style(Style::default().fg(rec_color)),
                ]);

                let styled_row = if data_row_idx % 2 == 1 {
                    row.style(theme::style_table_alt_row())
                } else {
                    row
                };
                data_row_idx += 1;
                styled_row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Length(13),
    ];

    let title = format!("Repos ({})", filtered.len());
    let table = ratatui::widgets::Table::new(rows, widths)
        .header(header)
        .block(theme::block_focused(&title))
        .row_highlight_style(theme::style_row_highlight());

    let len = filtered.len();
    let clamped = app.selected.min(len.saturating_sub(1));
    let visual_selected = repo_to_visual.get(clamped).copied();

    let mut state = ratatui::widgets::TableState::default();
    state.select(visual_selected);
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_worktrees(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.worktrees.is_empty() {
        widgets::render_empty_state(frame, area, "◇", "No worktree data yet.");
        return;
    }

    let header = Row::new(vec![
        Cell::from("REPO"),
        Cell::from("PATH"),
        Cell::from("BRANCH"),
        Cell::from("STATE"),
        Cell::from("ACTION"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .worktrees
        .iter()
        .map(|r| {
            let state_text = if r.detached {
                "detached"
            } else if r.bare {
                "bare"
            } else {
                "normal"
            };
            let state_color = if r.detached {
                theme::ACCENT_YELLOW
            } else if r.bare {
                theme::FG_DIMMED
            } else {
                theme::ACCENT_GREEN
            };
            Row::new(vec![
                Cell::from(r.repo.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(r.path.clone()).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(r.branch.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(state_text).style(Style::default().fg(state_color)),
                Cell::from(
                    r.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(theme::ACCENT_CYAN)),
            ])
        })
        .collect();

    let title = format!("Worktrees ({})", app.dashboard.worktrees.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(22),
            Constraint::Fill(1),
            Constraint::Length(18),
            Constraint::Length(10),
            Constraint::Length(16),
        ],
        app.selected,
        app.dashboard.worktrees.len(),
    );
}

fn render_processes(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.processes.is_empty() {
        widgets::render_empty_state(
            frame,
            area,
            "◇",
            "No repo-scoped running processes detected.",
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("REPO"),
        Cell::from("PID"),
        Cell::from("ELAPSED"),
        Cell::from("COMMAND"),
        Cell::from("ACTION"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .processes
        .iter()
        .map(|p| {
            let elapsed_color = elapsed_color(&p.elapsed);
            Row::new(vec![
                Cell::from(p.repo.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(p.pid.to_string()).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(p.elapsed.clone()).style(Style::default().fg(elapsed_color)),
                Cell::from(p.command.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(
                    p.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(theme::ACCENT_CYAN)),
            ])
        })
        .collect();

    let title = format!("Processes ({})", app.dashboard.processes.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(22),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(14),
        ],
        app.selected,
        app.dashboard.processes.len(),
    );
}

fn render_dependencies(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.dependencies.is_empty() {
        widgets::render_empty_state(
            frame,
            area,
            "◇",
            "No known dependency manifests found in scanned repos.",
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("REPO"),
        Cell::from("ECOSYSTEMS"),
        Cell::from("ISSUES"),
        Cell::from("DETAILS"),
        Cell::from("ACTION"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .dependencies
        .iter()
        .map(|d| {
            let (issue_text, issue_color) = if d.issue_count == 0 {
                ("✓".to_string(), theme::ACCENT_GREEN)
            } else {
                (
                    d.issue_count.to_string(),
                    if d.issue_count > 3 {
                        theme::ACCENT_RED
                    } else {
                        theme::ACCENT_YELLOW
                    },
                )
            };

            Row::new(vec![
                Cell::from(d.repo.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(d.ecosystems.join(", ")).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(issue_text).style(Style::default().fg(issue_color)),
                Cell::from(if d.issues.is_empty() {
                    "clean".to_string()
                } else {
                    d.issues.join("; ")
                })
                .style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(
                    d.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(theme::ACCENT_CYAN)),
            ])
        })
        .collect();

    let title = format!("Dependencies ({})", app.dashboard.dependencies.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(22),
            Constraint::Length(18),
            Constraint::Length(8),
            Constraint::Fill(1),
            Constraint::Length(16),
        ],
        app.selected,
        app.dashboard.dependencies.len(),
    );
}

fn render_env_audit(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.env_audit.is_empty() {
        widgets::render_empty_state(frame, area, "◇", "No .env files found in scanned repos.");
        return;
    }

    let header = Row::new(vec![
        Cell::from("REPO"),
        Cell::from("FILES"),
        Cell::from("MISSING"),
        Cell::from("EXTRA"),
        Cell::from("TRACKED"),
        Cell::from("ACTION"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .env_audit
        .iter()
        .map(|e| {
            let missing_count = e.missing_keys.len();
            let tracked_count = e.tracked_secret_files.len();

            Row::new(vec![
                Cell::from(e.repo.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(e.env_files.join(", ")).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(missing_count.to_string()).style(Style::default().fg(
                    if missing_count == 0 {
                        theme::ACCENT_GREEN
                    } else {
                        theme::ACCENT_YELLOW
                    },
                )),
                Cell::from(e.extra_keys.len().to_string())
                    .style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(tracked_count.to_string()).style(Style::default().fg(
                    if tracked_count == 0 {
                        theme::ACCENT_GREEN
                    } else {
                        theme::ACCENT_RED
                    },
                )),
                Cell::from(
                    e.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(theme::ACCENT_CYAN)),
            ])
        })
        .collect();

    let title = format!("Env Audit ({})", app.dashboard.env_audit.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(22),
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(16),
        ],
        app.selected,
        app.dashboard.env_audit.len(),
    );
}

fn render_mcp(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.mcp_servers.is_empty() {
        if app.is_scanning {
            widgets::render_empty_state(frame, area, "…", "Loading MCP health data…");
        } else {
            widgets::render_empty_state(frame, area, "◇", "No MCP configuration files detected.");
        }
        return;
    }

    let header = Row::new(vec![
        Cell::from("SERVER"),
        Cell::from("SOURCE"),
        Cell::from("HEALTH"),
        Cell::from("DETAIL"),
        Cell::from("ACTION"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .mcp_servers
        .iter()
        .map(|m| {
            let (health_text, health_color) = if m.healthy {
                ("● healthy", theme::ACCENT_GREEN)
            } else {
                ("● unhealthy", theme::ACCENT_RED)
            };
            Row::new(vec![
                Cell::from(m.server_name.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(m.source.clone()).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(health_text).style(Style::default().fg(health_color)),
                Cell::from(m.detail.clone()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(
                    m.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(theme::ACCENT_CYAN)),
            ])
        })
        .collect();

    let title = format!("MCP Health ({})", app.dashboard.mcp_servers.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(20),
            Constraint::Fill(1),
            Constraint::Length(14),
            Constraint::Length(28),
            Constraint::Length(14),
        ],
        app.selected,
        app.dashboard.mcp_servers.len(),
    );
}

fn render_ai_costs(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.providers.is_empty() {
        if app.is_scanning {
            widgets::render_empty_state(frame, area, "…", "Loading AI usage and cost data…");
        } else {
            widgets::render_empty_state(frame, area, "◇", "No AI provider data available yet.");
        }
        return;
    }

    let header = Row::new(vec![
        Cell::from("PROVIDER"),
        Cell::from("SOURCE"),
        Cell::from("UPDATED"),
        Cell::from("CONFIG"),
        Cell::from("SESSIONS"),
        Cell::from("INPUT TOKENS"),
        Cell::from("OUTPUT TOKENS"),
        Cell::from("COST USD"),
        Cell::from("NOTES"),
    ])
    .style(theme::style_header());

    let rows: Vec<Row> = app
        .dashboard
        .providers
        .iter()
        .map(|p| {
            let cost_color = if p.estimated_cost_usd > 10.0 {
                theme::ACCENT_RED
            } else if p.estimated_cost_usd > 1.0 {
                theme::ACCENT_ORANGE
            } else if p.estimated_cost_usd > 0.0 {
                theme::ACCENT_YELLOW
            } else {
                theme::FG_DIMMED
            };

            Row::new(vec![
                Cell::from(p.provider.as_str()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(p.data_source.clone()).style(Style::default().fg(theme::FG_SECONDARY)),
                Cell::from(format_updated_secs(p.source_updated_at_epoch_secs))
                    .style(Style::default().fg(theme::FG_DIMMED)),
                Cell::from(if p.configured { "yes" } else { "no" }).style(Style::default().fg(
                    if p.configured {
                        theme::ACCENT_GREEN
                    } else {
                        theme::ACCENT_YELLOW
                    },
                )),
                Cell::from(p.sessions.to_string()).style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(widgets::format_number(p.total_input_tokens))
                    .style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(widgets::format_number(p.total_output_tokens))
                    .style(Style::default().fg(theme::FG_PRIMARY)),
                Cell::from(format!("${:.2}", p.estimated_cost_usd))
                    .style(Style::default().fg(cost_color)),
                Cell::from(if p.notes.is_empty() {
                    "—".to_string()
                } else {
                    p.notes.join("; ")
                })
                .style(Style::default().fg(theme::FG_SECONDARY)),
            ])
        })
        .collect();

    let title = format!("AI Usage & Cost ({})", app.dashboard.providers.len());
    widgets::render_styled_table(
        frame,
        area,
        &title,
        header,
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Fill(1),
        ],
        app.selected,
        app.dashboard.providers.len(),
    );
}

fn format_updated_secs(epoch_secs: i64) -> String {
    if epoch_secs <= 0 {
        return "unknown".to_string();
    }
    let now = chrono::Utc::now().timestamp();
    let delta = now.saturating_sub(epoch_secs);
    if delta < 60 {
        format!("{}s ago", delta)
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

fn render_selected_detail(frame: &mut Frame, app: &App, area: Rect) {
    let text = selected_detail_text(app);
    frame.render_widget(
        Paragraph::new(text)
            .block(theme::block_default("Selected"))
            .style(Style::default().fg(theme::FG_SECONDARY))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn selected_detail_text(app: &App) -> String {
    match app.section {
        DashboardSection::Repos => {
            if let Some(repo) = app.selected_repo() {
                let rec = agent::recommend(repo);
                format!(
                    "repo={} path={} branch={} dirty={} ahead={} behind={} next={} reason={}",
                    repo.name,
                    repo.path.display(),
                    repo.status.branch,
                    repo.status.uncommitted_count,
                    repo.status.unpushed_count,
                    repo.status.behind_count,
                    rec.short_action,
                    rec.reason
                )
            } else {
                "No selected repo".to_string()
            }
        }
        DashboardSection::Worktrees => app
            .dashboard
            .worktrees
            .get(app.selected)
            .map(|wt| {
                format!(
                    "repo={} path={} branch={} detached={} bare={} action={}",
                    wt.repo,
                    wt.path,
                    wt.branch,
                    wt.detached,
                    wt.bare,
                    wt.action
                        .as_ref()
                        .map(|a| a.command.clone())
                        .unwrap_or_else(|| "none".to_string())
                )
            })
            .unwrap_or_else(|| "No selected worktree".to_string()),
        DashboardSection::Processes => app
            .dashboard
            .processes
            .get(app.selected)
            .map(|p| format!("repo={} pid={} elapsed={} cmd={}", p.repo, p.pid, p.elapsed, p.command))
            .unwrap_or_else(|| "No selected process".to_string()),
        DashboardSection::Dependencies => app
            .dashboard
            .dependencies
            .get(app.selected)
            .map(|d| {
                format!(
                    "repo={} ecosystems={} issues={} details={}",
                    d.repo,
                    d.ecosystems.join(","),
                    d.issue_count,
                    if d.issues.is_empty() {
                        "clean".to_string()
                    } else {
                        d.issues.join(" | ")
                    }
                )
            })
            .unwrap_or_else(|| "No selected dependency row".to_string()),
        DashboardSection::EnvAudit => app
            .dashboard
            .env_audit
            .get(app.selected)
            .map(|e| {
                format!(
                    "repo={} files={} missing=[{}] extra=[{}] tracked=[{}]",
                    e.repo,
                    e.env_files.join(","),
                    e.missing_keys.join(","),
                    e.extra_keys.join(","),
                    e.tracked_secret_files.join(",")
                )
            })
            .unwrap_or_else(|| "No selected env audit row".to_string()),
        DashboardSection::McpHealth => app
            .dashboard
            .mcp_servers
            .get(app.selected)
            .map(|m| {
                format!(
                    "server={} source={} healthy={} detail={} command={}",
                    m.server_name, m.source, m.healthy, m.detail, m.command
                )
            })
            .unwrap_or_else(|| "No selected MCP row".to_string()),
        DashboardSection::AiCosts => app
            .dashboard
            .providers
            .get(app.selected)
            .map(|p| {
                format!(
                    "provider={} source={} updated={} sessions={} input={} output={} cost=${:.2} notes={}",
                    p.provider.as_str(),
                    p.data_source,
                    format_updated_secs(p.source_updated_at_epoch_secs),
                    p.sessions,
                    p.total_input_tokens,
                    p.total_output_tokens,
                    p.estimated_cost_usd,
                    p.notes.join(" | ")
                )
            })
            .unwrap_or_else(|| "No selected provider row".to_string()),
        DashboardSection::Home => "Use Home for overview alerts".to_string(),
    }
}

/// Determine elapsed time color: green < 1m, yellow < 5m, orange < 30m, red >= 30m.
fn elapsed_color(elapsed: &str) -> ratatui::style::Color {
    // Elapsed is typically "Xm Ys" or "Xs" format
    let lower = elapsed.to_lowercase();
    if lower.contains('h') {
        return theme::ACCENT_RED;
    }
    // Try to extract minute value
    if let Some(m_pos) = lower.find('m') {
        if let Ok(mins) = lower[..m_pos].trim().parse::<u32>() {
            return if mins >= 30 {
                theme::ACCENT_RED
            } else if mins >= 5 {
                theme::ACCENT_ORANGE
            } else {
                theme::ACCENT_YELLOW
            };
        }
    }
    // Seconds only — short-lived
    theme::ACCENT_GREEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_seconds_is_green() {
        assert_eq!(elapsed_color("30s"), theme::ACCENT_GREEN);
        assert_eq!(elapsed_color("5s"), theme::ACCENT_GREEN);
    }

    #[test]
    fn elapsed_short_minutes_is_yellow() {
        assert_eq!(elapsed_color("1m 30s"), theme::ACCENT_YELLOW);
        assert_eq!(elapsed_color("4m"), theme::ACCENT_YELLOW);
    }

    #[test]
    fn elapsed_medium_minutes_is_orange() {
        assert_eq!(elapsed_color("5m"), theme::ACCENT_ORANGE);
        assert_eq!(elapsed_color("29m"), theme::ACCENT_ORANGE);
    }

    #[test]
    fn elapsed_long_minutes_is_red() {
        assert_eq!(elapsed_color("30m"), theme::ACCENT_RED);
        assert_eq!(elapsed_color("45m 10s"), theme::ACCENT_RED);
    }

    #[test]
    fn elapsed_hours_is_red() {
        assert_eq!(elapsed_color("1h 30m"), theme::ACCENT_RED);
        assert_eq!(elapsed_color("2h"), theme::ACCENT_RED);
    }
}
