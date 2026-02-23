pub mod commit_bar;
pub mod filter;
pub mod help;
pub mod summary_bar;
pub mod table;

use crate::app::{App, AppMode};
use crate::dashboard::DashboardSection;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph},
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
                .style(Style::default().fg(Color::Yellow)),
            area,
        );
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3), // summary
        Constraint::Fill(1),   // sidebar + section content
        Constraint::Length(1), // status / filter / commit
    ])
    .split(frame.area());

    let body = Layout::horizontal([Constraint::Length(24), Constraint::Fill(1)]).split(chunks[1]);

    summary_bar::render(frame, app, chunks[0]);
    render_sidebar(frame, app, body[0]);
    table::render(frame, app, body[1]);

    match app.mode {
        AppMode::Search => filter::render(frame, app, chunks[2]),
        AppMode::Commit => commit_bar::render(frame, app, chunks[2]),
        _ => render_status_bar(frame, app, chunks[2]),
    }

    if app.mode == AppMode::Help {
        help::render(frame, app);
    }
}

fn render_sidebar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let all = DashboardSection::all();
    let items: Vec<ListItem> = all
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            ListItem::new(format!(" {}. {}", idx + 1, s.title())).style(Style::default().fg(
                if *s == app.section {
                    Color::Cyan
                } else {
                    Color::Gray
                },
            ))
        })
        .collect();

    let mut state = ListState::default();
    let selected_idx = all.iter().position(|s| *s == app.section).unwrap_or(0);
    state.select(Some(selected_idx));

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" Sections ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Show transient notification if present, otherwise show key hints
    let text = if let Some((msg, _)) = &app.notification {
        format!(" {}", msg)
    } else {
        let scanning = if app.is_scanning {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let frame = ((millis / 100) as usize) % SPINNER.len();
            format!("  {} scanning", SPINNER[frame])
        } else {
            String::new()
        };

        let section_specific = if app.section == DashboardSection::Repos {
            "  ·  ↵ open  o finder  f fetch  p pull  P push  c commit  g group  A focus"
        } else {
            ""
        };

        format!(
            " h/l or tab: section  j/k: row  x: run action  r: refresh  /: repo filter  ?: help  q: quit{}{}",
            section_specific, scanning
        )
    };

    let style = if app.notification.is_some() {
        Style::default().bg(Color::DarkGray).fg(Color::Green)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}
