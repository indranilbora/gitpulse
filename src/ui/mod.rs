pub mod commit_bar;
pub mod filter;
pub mod help;
pub mod home;
pub mod sidebar;
pub mod summary_bar;
pub mod table;
pub mod theme;
pub mod widgets;

use crate::app::{App, AppMode};
use crate::dashboard::DashboardSection;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 18;

/// Top-level render: lays out the screen and delegates to sub-renderers.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Guard: tell the user to resize if the terminal is too small
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = format!(
            "Terminal too small ({}×{})\nPlease resize to at least {}×{}",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT
        );
        frame.render_widget(
            Paragraph::new(msg)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(theme::ACCENT_YELLOW)),
            area,
        );
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3), // summary
        Constraint::Fill(1),  // sidebar + section content
        Constraint::Length(1), // status / filter / commit
    ])
    .split(frame.area());

    let body = Layout::horizontal([Constraint::Length(24), Constraint::Fill(1)]).split(chunks[1]);

    summary_bar::render(frame, app, chunks[0]);
    sidebar::render(frame, app, body[0]);

    // Route Home to home::render, everything else to table::render
    if app.section == DashboardSection::Home {
        home::render(frame, app, body[1]);
    } else {
        table::render(frame, app, body[1]);
    }

    match app.mode {
        AppMode::Search => filter::render(frame, app, chunks[2]),
        AppMode::Commit => commit_bar::render(frame, app, chunks[2]),
        _ => render_status_bar(frame, app, chunks[2]),
    }

    if app.mode == AppMode::Help {
        help::render(frame, app);
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Show transient notification if present
    if let Some((msg, _)) = &app.notification {
        let line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(msg, Style::default().fg(theme::ACCENT_GREEN)),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(theme::BG_SECONDARY)),
            area,
        );
        return;
    }

    let mut spans: Vec<Span> = vec![Span::raw(" ")];

    // Core navigation hints
    let hints: &[(&str, &str)] = &[
        ("h/l", "section"),
        ("j/k", "row"),
        ("x", "action"),
        ("r", "refresh"),
        ("/", "filter"),
        ("?", "help"),
        ("q", "quit"),
    ];

    for (key, desc) in hints {
        spans.extend(widgets::key_hint(key, desc));
    }

    // Section-specific hints for Repos
    if app.section == DashboardSection::Repos {
        spans.push(Span::styled("│ ", Style::default().fg(theme::FG_DIMMED)));
        let repo_hints: &[(&str, &str)] = &[
            ("↵", "open"),
            ("f", "fetch"),
            ("p", "pull"),
            ("P", "push"),
            ("c", "commit"),
            ("g", "group"),
        ];
        for (key, desc) in repo_hints {
            spans.extend(widgets::key_hint(key, desc));
        }
    }

    // Scanning spinner on the right
    if app.is_scanning {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let frame_idx = ((millis / 100) as usize) % SPINNER.len();
        spans.push(Span::styled(
            format!(" {} scanning", SPINNER[frame_idx]),
            Style::default().fg(theme::ACCENT_YELLOW),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG_SECONDARY)),
        area,
    );
}
