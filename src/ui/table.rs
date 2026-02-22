use crate::agent;
use crate::app::App;
use crate::git::{Repo, StatusColor};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

// ─── grouping helpers ────────────────────────────────────────────────────────

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

        visual.push(entries.len()); // visual index of this repo row
        entries.push(Entry::Repo(repo));
    }

    (entries, visual)
}

// ─── render ──────────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_repos();

    if filtered.is_empty() {
        let msg = if app.filter_text.is_empty() {
            "No repositories found — run `agentpulse --setup` to configure watch directories"
                .to_string()
        } else {
            format!("No repos matching \"{}\"", app.filter_text)
        };
        frame.render_widget(
            Paragraph::new(msg)
                .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray)))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            area,
        );
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
        .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray)))
        .row_highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

    // Map the repo selection index to its visual row (accounting for group headers)
    let len = filtered.len();
    let clamped = app.selected.min(len.saturating_sub(1));
    let visual_selected = repo_to_visual.get(clamped).copied();

    let mut state = TableState::default();
    state.select(visual_selected);
    frame.render_stateful_widget(table, area, &mut state);
}
