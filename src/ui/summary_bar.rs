use super::theme;
use crate::app::App;
use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let ov = &app.dashboard.overview;

    // Left: section icon + name
    let section_name = app.section.title();

    // Center: metrics separated by ·
    let cost = app.dashboard.total_estimated_cost_usd();

    // Right: scan status
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

    let active_count = app.active_row_count();
    let counter = if active_count > 0 {
        let display_index = app.selected.min(active_count.saturating_sub(1));
        format!("{}/{}", display_index + 1, active_count)
    } else {
        String::new()
    };

    // Build main status line
    let mut spans: Vec<Span> = vec![
        Span::styled(
            format!(" {} ", section_name),
            Style::default()
                .fg(theme::ACCENT_BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)),
        Span::styled(
            format!("{} repos", ov.total_repos),
            Style::default().fg(theme::FG_SECONDARY),
        ),
        Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)),
        Span::styled(
            format!("{} dirty", ov.dirty_repos),
            Style::default().fg(if ov.dirty_repos > 0 {
                theme::ACCENT_YELLOW
            } else {
                theme::FG_SECONDARY
            }),
        ),
        Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)),
        Span::styled(
            format!("{} proc", ov.repo_processes),
            Style::default().fg(theme::FG_SECONDARY),
        ),
        Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)),
        Span::styled(
            format!("${:.2}", cost),
            Style::default().fg(if cost > 0.0 {
                theme::ACCENT_YELLOW
            } else {
                theme::FG_SECONDARY
            }),
        ),
    ];

    // Filters
    if !app.filter_text.is_empty() {
        spans.push(Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)));
        spans.push(Span::styled(
            format!("filter: \"{}\"", app.filter_text),
            Style::default().fg(theme::ACCENT_CYAN),
        ));
    }
    if app.agent_focus_mode {
        spans.push(Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)));
        spans.push(Span::styled(
            "focus: actionable",
            Style::default().fg(theme::ACCENT_CYAN),
        ));
    }

    // Right side: scan + counter
    spans.push(Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)));
    spans.push(Span::styled(
        scan_info,
        Style::default().fg(if app.is_scanning {
            theme::ACCENT_YELLOW
        } else {
            theme::FG_DIMMED
        }),
    ));

    if !counter.is_empty() {
        spans.push(Span::styled(" · ", Style::default().fg(theme::FG_DIMMED)));
        spans.push(Span::styled(counter, Style::default().fg(theme::FG_DIMMED)));
    }

    let mut lines = vec![Line::from(spans)];

    // Missing directories warning
    if !app.config.missing_directories.is_empty() {
        let names: Vec<String> = app
            .config
            .missing_directories
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        lines.push(Line::from(vec![
            Span::styled(" ⚠ not found: ", Style::default().fg(theme::ACCENT_YELLOW)),
            Span::styled(names.join(", "), Style::default().fg(theme::FG_DIMMED)),
        ]));
    }

    let para = Paragraph::new(lines).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::BORDER_NORMAL))
            .title(" AgentPulse Dashboard ")
            .title_style(
                Style::default()
                    .fg(theme::ACCENT_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(para, area);
}
