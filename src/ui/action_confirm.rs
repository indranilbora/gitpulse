use super::theme;
use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let Some(action) = app.pending_action.as_ref() else {
        return;
    };

    let area = centered_rect(88, 16, frame.area());
    let risk = action.action.risk_level();
    let risk_color = match risk {
        "high" => theme::ACCENT_RED,
        "medium" => theme::ACCENT_YELLOW,
        _ => theme::ACCENT_CYAN,
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Action: ", Style::default().fg(theme::FG_DIMMED)),
            Span::styled(
                action.label.clone(),
                Style::default()
                    .fg(theme::FG_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Risk:   ", Style::default().fg(theme::FG_DIMMED)),
            Span::styled(
                risk.to_uppercase(),
                Style::default().fg(risk_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Command preview:",
            Style::default().fg(theme::FG_DIMMED),
        )]),
        Line::from(vec![Span::styled(
            format!("  {}", action.command),
            Style::default().fg(theme::FG_PRIMARY),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {}", action.action.cancel_reassurance()),
            Style::default().fg(theme::FG_SECONDARY),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enter / y",
                Style::default()
                    .fg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" run once   ", Style::default().fg(theme::FG_DIMMED)),
            Span::styled(
                "Esc / n",
                Style::default()
                    .fg(theme::ACCENT_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cancel", Style::default().fg(theme::FG_DIMMED)),
        ]),
    ];

    if action.action.is_destructive() {
        lines.insert(
            1,
            Line::from(vec![Span::styled(
                "  Destructive action: review command before running.",
                Style::default()
                    .fg(theme::ACCENT_RED)
                    .add_modifier(Modifier::BOLD),
            )]),
        );
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER_FOCUSED))
                    .title(" Confirm Action ")
                    .title_style(
                        Style::default()
                            .fg(theme::ACCENT_BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(Style::default().bg(theme::BG_ELEVATED)),
        area,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
