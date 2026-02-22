pub mod commit_bar;
pub mod filter;
pub mod help;
pub mod summary_bar;
pub mod table;

use crate::app::{App, AppMode};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 10;

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
        Constraint::Length(3), // summary bar (with border = 1 line content + 2 borders)
        Constraint::Fill(1),   // main table
        Constraint::Length(1), // status / filter / commit bar
    ])
    .split(frame.area());

    summary_bar::render(frame, app, chunks[0]);
    table::render(frame, app, chunks[1]);

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
        format!(
            " ↵ open  o finder  r refresh  ·  f fetch  p pull  P push  c commit  ·  g group  a agent  / filter  s setup  ? help  q quit{}",
            scanning
        )
    };

    let style = if app.notification.is_some() {
        Style::default().bg(Color::DarkGray).fg(Color::Green)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}
