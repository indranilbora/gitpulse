use crate::agent;
use crate::app::App;
use crate::dashboard::DashboardSection;
use crate::git::{Repo, StatusColor};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
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
    match app.section {
        DashboardSection::Home => render_home(frame, app, area),
        DashboardSection::Repos => render_repos(frame, app, area),
        DashboardSection::Worktrees => render_worktrees(frame, app, area),
        DashboardSection::Processes => render_processes(frame, app, area),
        DashboardSection::Dependencies => render_dependencies(frame, app, area),
        DashboardSection::EnvAudit => render_env_audit(frame, app, area),
        DashboardSection::McpHealth => render_mcp(frame, app, area),
        DashboardSection::AiCosts => render_ai_costs(frame, app, area),
    }
}

fn render_home(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).split(area);

    let overview = &app.dashboard.overview;
    let cost = app.dashboard.total_estimated_cost_usd();
    let info = [
        format!(
            "repos: {}   actionable: {}   dirty: {}   ahead: {}   behind: {}",
            overview.total_repos,
            overview.actionable_repos,
            overview.dirty_repos,
            overview.repos_ahead,
            overview.repos_behind
        ),
        format!(
            "worktrees: {}   repo processes: {}   dep issues: {}   env issues: {}   mcp unhealthy: {}   est ai cost: ${:.2}",
            overview.total_worktrees,
            overview.repo_processes,
            overview.dep_issues,
            overview.env_issues,
            overview.mcp_unhealthy,
            cost,
        ),
    ]
    .join("\n");

    frame.render_widget(
        Paragraph::new(info)
            .block(Block::bordered().title(" Overview "))
            .style(Style::default().fg(Color::White)),
        chunks[0],
    );

    if app.dashboard.alerts.is_empty() {
        render_empty(
            frame,
            chunks[1],
            "No alerts. Workspace looks healthy across repos, deps, env, MCP, and AI configs.",
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("SEV"),
        Cell::from("TITLE"),
        Cell::from("DETAIL"),
        Cell::from("ACTION"),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .alerts
        .iter()
        .map(|a| {
            let color = match a.severity.as_str() {
                "critical" | "high" => Color::Red,
                "warn" => Color::Yellow,
                _ => Color::Blue,
            };
            Row::new(vec![
                Cell::from(a.severity.clone()).style(Style::default().fg(color)),
                Cell::from(a.title.clone()),
                Cell::from(a.detail.clone()),
                Cell::from(
                    a.action
                        .as_ref()
                        .map(|x| x.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(34),
            Constraint::Fill(1),
            Constraint::Length(24),
        ],
    )
    .header(header)
    .block(Block::bordered().title(" Alerts (x to run selected action) "))
    .row_highlight_style(
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = TableState::default();
    state.select(Some(
        app.selected
            .min(app.dashboard.alerts.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, chunks[1], &mut state);
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
        render_empty(frame, area, &msg);
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
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

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
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),

            Entry::Repo(repo) => {
                let (indicator, color) = match repo.status_color() {
                    StatusColor::Clean => ("○", Color::Green),
                    StatusColor::Uncommitted => ("●", Color::Yellow),
                    StatusColor::Unpushed => ("●", Color::Blue),
                    StatusColor::Dirty => ("●", Color::Red),
                    StatusColor::NoRemote => ("○", Color::DarkGray),
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
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    (repo.status.branch.clone(), Style::default())
                };

                Row::new(vec![
                    Cell::from(indicator).style(Style::default().fg(color)),
                    Cell::from(repo.name.clone()),
                    Cell::from(branch_text).style(branch_style),
                    Cell::from(dirty),
                    Cell::from(sync),
                    Cell::from(stash).style(Style::default().fg(Color::Magenta)),
                    Cell::from(next).style(Style::default().fg(Color::Cyan)),
                ])
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

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title(" Repos (x to run NEXT action) "))
        .row_highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    let len = filtered.len();
    let clamped = app.selected.min(len.saturating_sub(1));
    let visual_selected = repo_to_visual.get(clamped).copied();

    let mut state = TableState::default();
    state.select(visual_selected);
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_worktrees(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.worktrees.is_empty() {
        render_empty(frame, area, "No worktree data yet.");
        return;
    }

    let header = Row::new(vec![
        Cell::from("REPO"),
        Cell::from("PATH"),
        Cell::from("BRANCH"),
        Cell::from("STATE"),
        Cell::from("ACTION"),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .worktrees
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(r.repo.clone()),
                Cell::from(r.path.clone()),
                Cell::from(r.branch.clone()),
                Cell::from(if r.detached {
                    "detached"
                } else if r.bare {
                    "bare"
                } else {
                    "normal"
                }),
                Cell::from(
                    r.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " Worktrees ",
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
        render_empty(
            frame,
            area,
            "No repo-scoped running processes detected from current command lines.",
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
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .processes
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(p.repo.clone()),
                Cell::from(p.pid.to_string()),
                Cell::from(p.elapsed.clone()),
                Cell::from(p.command.clone()),
                Cell::from(
                    p.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " Processes (x to run action) ",
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
        render_empty(
            frame,
            area,
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
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .dependencies
        .iter()
        .map(|d| {
            Row::new(vec![
                Cell::from(d.repo.clone()),
                Cell::from(d.ecosystems.join(", ")),
                Cell::from(d.issue_count.to_string()).style(Style::default().fg(
                    if d.issue_count > 0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    },
                )),
                Cell::from(if d.issues.is_empty() {
                    "clean".to_string()
                } else {
                    d.issues.join("; ")
                }),
                Cell::from(
                    d.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " Dependencies ",
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
        render_empty(frame, area, "No .env files found in scanned repos.");
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
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .env_audit
        .iter()
        .map(|e| {
            Row::new(vec![
                Cell::from(e.repo.clone()),
                Cell::from(e.env_files.join(", ")),
                Cell::from(e.missing_keys.len().to_string()).style(Style::default().fg(
                    if e.missing_keys.is_empty() {
                        Color::Green
                    } else {
                        Color::Yellow
                    },
                )),
                Cell::from(e.extra_keys.len().to_string()),
                Cell::from(e.tracked_secret_files.len().to_string()).style(Style::default().fg(
                    if e.tracked_secret_files.is_empty() {
                        Color::Green
                    } else {
                        Color::Red
                    },
                )),
                Cell::from(
                    e.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " Env Audit ",
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
        render_empty(frame, area, "No MCP configuration files detected.");
        return;
    }

    let header = Row::new(vec![
        Cell::from("SERVER"),
        Cell::from("SOURCE"),
        Cell::from("HEALTH"),
        Cell::from("DETAIL"),
        Cell::from("ACTION"),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .mcp_servers
        .iter()
        .map(|m| {
            Row::new(vec![
                Cell::from(m.server_name.clone()),
                Cell::from(m.source.clone()),
                Cell::from(if m.healthy { "healthy" } else { "unhealthy" })
                    .style(Style::default().fg(if m.healthy { Color::Green } else { Color::Red })),
                Cell::from(m.detail.clone()),
                Cell::from(
                    m.action
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or_else(|| "—".to_string()),
                )
                .style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " MCP Health ",
        header,
        rows,
        [
            Constraint::Length(20),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(28),
            Constraint::Length(14),
        ],
        app.selected,
        app.dashboard.mcp_servers.len(),
    );
}

fn render_ai_costs(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.providers.is_empty() {
        render_empty(frame, area, "No AI provider data available yet.");
        return;
    }

    let header = Row::new(vec![
        Cell::from("PROVIDER"),
        Cell::from("CONFIG"),
        Cell::from("SESSIONS"),
        Cell::from("INPUT TOKENS"),
        Cell::from("OUTPUT TOKENS"),
        Cell::from("EST COST"),
        Cell::from("NOTES"),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .dashboard
        .providers
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(p.provider.as_str()),
                Cell::from(if p.configured { "yes" } else { "no" }).style(Style::default().fg(
                    if p.configured {
                        Color::Green
                    } else {
                        Color::Yellow
                    },
                )),
                Cell::from(p.sessions.to_string()),
                Cell::from(p.total_input_tokens.to_string()),
                Cell::from(p.total_output_tokens.to_string()),
                Cell::from(format!("${:.2}", p.estimated_cost_usd)).style(Style::default().fg(
                    if p.estimated_cost_usd > 0.0 {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    },
                )),
                Cell::from(if p.notes.is_empty() {
                    "—".to_string()
                } else {
                    p.notes.join("; ")
                }),
            ])
        })
        .collect();

    render_table_with_selection(
        frame,
        area,
        " AI Usage & Cost ",
        header,
        rows,
        [
            Constraint::Length(12),
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

#[allow(clippy::too_many_arguments)]
fn render_table_with_selection<const N: usize>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    header: Row,
    rows: Vec<Row>,
    widths: [Constraint; N],
    selected: usize,
    len: usize,
) {
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title(title))
        .row_highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(Some(selected.min(len.saturating_sub(1))));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_empty(frame: &mut Frame, area: Rect, message: &str) {
    frame.render_widget(
        Paragraph::new(message)
            .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray)))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
