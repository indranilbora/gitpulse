use crate::app::App;
use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.repos.len();
    let dirty = app
        .repos
        .iter()
        .filter(|r| r.status.uncommitted_count > 0)
        .count();
    let unpushed = app
        .repos
        .iter()
        .filter(|r| r.status.unpushed_count > 0)
        .count();

    let scan_info = if app.is_scanning {
        "Scanning…".to_string()
    } else if let Some(t) = &app.last_scan {
        let secs = Local::now().signed_duration_since(*t).num_seconds();
        if secs < 60 {
            format!("{}s ago", secs)
        } else {
            format!("{}m ago", secs / 60)
        }
    } else {
        "Never".to_string()
    };

    let filter_hint = if !app.filter_text.is_empty() {
        format!("  │  filter: \"{}\"", app.filter_text)
    } else {
        String::new()
    };

    let status_line = format!(
        " {} repos  │  {} dirty  │  {} unpushed  │  {}{}",
        total, dirty, unpushed, scan_info, filter_hint
    );

    let filtered_count = app.filtered_repos().len();
    let mut status_spans = vec![Span::raw(status_line)];
    if filtered_count > 0 {
        let display_index = app.selected.min(filtered_count.saturating_sub(1));
        status_spans.push(Span::raw("  │  "));
        status_spans.push(Span::styled(
            format!("{} of {}", display_index + 1, filtered_count),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Warn about configured directories that don't exist on disk
    let mut lines = vec![Line::from(status_spans)];
    if !app.config.missing_directories.is_empty() {
        let names: Vec<String> = app
            .config
            .missing_directories
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        lines.push(Line::from(vec![
            Span::styled(" ⚠ not found: ", Style::default().fg(Color::Yellow)),
            Span::styled(names.join(", "), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let para = Paragraph::new(lines)
        .block(
            Block::bordered()
                .title(" GitPulse ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, area);
}
