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
    let ov = &app.dashboard.overview;

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

    let mut suffix = String::new();
    if !app.filter_text.is_empty() {
        suffix.push_str(&format!("  │  filter: \"{}\"", app.filter_text));
    }
    if app.agent_focus_mode {
        suffix.push_str("  │  repo-focus: actionable");
    }

    let status_line = format!(
        " section: {}  │  repos {} ({} actionable)  │  proc {}  │  dep {}  │  env {}  │  mcp {}  │  ai ${:.2}  │  {}{}",
        app.section.title(),
        ov.total_repos,
        ov.actionable_repos,
        ov.repo_processes,
        ov.dep_issues,
        ov.env_issues,
        ov.mcp_unhealthy,
        app.dashboard.total_estimated_cost_usd(),
        scan_info,
        suffix,
    );

    let active_count = app.active_row_count();
    let mut status_spans = vec![Span::raw(status_line)];
    if active_count > 0 {
        let display_index = app.selected.min(active_count.saturating_sub(1));
        status_spans.push(Span::raw("  │  "));
        status_spans.push(Span::styled(
            format!("{} of {}", display_index + 1, active_count),
            Style::default().fg(Color::DarkGray),
        ));
    }

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
                .title(" AgentPulse Dashboard ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, area);
}
