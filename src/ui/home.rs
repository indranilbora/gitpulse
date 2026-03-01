use super::{theme, widgets};
use crate::app::App;
use chrono::Local;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // greeting
        Constraint::Length(5), // stat cards
        Constraint::Fill(1),   // alerts list
    ])
    .split(area);

    render_greeting(frame, app, chunks[0]);
    render_stat_cards(frame, app, chunks[1]);
    render_alerts(frame, app, chunks[2]);
}

fn render_greeting(frame: &mut Frame, app: &App, area: Rect) {
    let hour = Local::now().hour();
    let greeting = match hour {
        5..=11 => "Good morning",
        12..=16 => "Good afternoon",
        17..=21 => "Good evening",
        _ => "Good night",
    };

    let overview = &app.dashboard.overview;
    let attention = overview.actionable_repos;
    let summary = if attention == 0 {
        format!(
            "{}. {} repos monitored, all clear.",
            greeting, overview.total_repos
        )
    } else {
        format!(
            "{}. {} repos monitored, {} need attention.",
            greeting, overview.total_repos, attention
        )
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_NORMAL));

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {}", summary),
            Style::default().fg(theme::FG_PRIMARY),
        )]))
        .block(block),
        area,
    );
}

fn render_stat_cards(frame: &mut Frame, app: &App, area: Rect) {
    let overview = &app.dashboard.overview;
    let cost = app.dashboard.total_estimated_cost_usd();

    let card_areas = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .split(area);

    widgets::render_stat_card(
        frame,
        card_areas[0],
        "Repos",
        &overview.total_repos.to_string(),
        theme::ACCENT_BLUE,
    );
    widgets::render_stat_card(
        frame,
        card_areas[1],
        "Dirty",
        &overview.dirty_repos.to_string(),
        if overview.dirty_repos > 0 {
            theme::ACCENT_YELLOW
        } else {
            theme::ACCENT_GREEN
        },
    );
    widgets::render_stat_card(
        frame,
        card_areas[2],
        "Procs",
        &overview.repo_processes.to_string(),
        if overview.repo_processes > 0 {
            theme::ACCENT_CYAN
        } else {
            theme::FG_DIMMED
        },
    );
    widgets::render_stat_card(
        frame,
        card_areas[3],
        "AI Cost",
        &format!("${:.2}", cost),
        if cost > 10.0 {
            theme::ACCENT_ORANGE
        } else if cost > 0.0 {
            theme::ACCENT_YELLOW
        } else {
            theme::FG_DIMMED
        },
    );
}

fn render_alerts(frame: &mut Frame, app: &App, area: Rect) {
    if app.dashboard.alerts.is_empty() {
        widgets::render_empty_state(frame, area, "✓", "No alerts. Workspace looks healthy.");
        return;
    }

    let items: Vec<ListItem> = app
        .dashboard
        .alerts
        .iter()
        .map(|a| {
            let sev_color = theme::severity_color(&a.severity);
            let dot = if a.severity == "critical" || a.severity == "high" {
                "●"
            } else {
                "○"
            };
            let action_text = a
                .action
                .as_ref()
                .map(|x| x.label.clone())
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", dot), Style::default().fg(sev_color)),
                Span::styled(
                    format!("{:<7}", a.severity),
                    Style::default().fg(sev_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate_str(&a.title, 32),
                    Style::default().fg(theme::FG_PRIMARY),
                ),
                Span::raw("  "),
                Span::styled(
                    truncate_str(&a.detail, 30),
                    Style::default().fg(theme::FG_SECONDARY),
                ),
                Span::raw("  "),
                Span::styled(action_text, Style::default().fg(theme::ACCENT_CYAN)),
            ]))
        })
        .collect();

    let title = format!("Alerts ({})", app.dashboard.alerts.len());
    let list = List::new(items)
        .block(theme::block_focused(&title))
        .highlight_style(theme::style_row_highlight());

    let mut state = ListState::default();
    state.select(Some(
        app.selected
            .min(app.dashboard.alerts.len().saturating_sub(1)),
    ));

    frame.render_stateful_widget(list, area, &mut state);
}

fn truncate_str(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

trait HourExt {
    fn hour(&self) -> u32;
}

impl HourExt for chrono::DateTime<Local> {
    fn hour(&self) -> u32 {
        chrono::Timelike::hour(self)
    }
}
