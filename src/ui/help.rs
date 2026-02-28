use super::theme;
use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, _app: &App) {
    let area = centered_rect(74, 30, frame.area());

    let categories: &[(&str, &[(&str, &str)])] = &[
        (
            "NAVIGATION",
            &[
                ("h/l Tab", "Switch section"),
                ("1..8", "Jump to section"),
                ("j / ↓", "Move down"),
                ("k / ↑", "Move up"),
            ],
        ),
        (
            "ACTIONS",
            &[
                ("x", "Run selected action"),
                ("r", "Force refresh"),
                ("/", "Filter search"),
                ("Enter", "Open in editor"),
                ("o", "Open in file manager"),
            ],
        ),
        (
            "GIT",
            &[
                ("f", "Fetch"),
                ("p", "Pull"),
                ("P", "Push"),
                ("c", "Commit tracked changes"),
            ],
        ),
        (
            "GENERAL",
            &[
                ("g", "Group by directory"),
                ("A", "Actionable-only mode"),
                ("s", "Setup watch dirs"),
                ("?", "Toggle help"),
                ("q", "Quit"),
            ],
        ),
    ];

    let mut lines: Vec<Line> = vec![Line::from(""), Line::from("")];

    for (cat_name, shortcuts) in categories {
        // Category header
        lines.push(Line::from(Span::styled(
            format!("  {}", cat_name),
            Style::default()
                .fg(theme::FG_DIMMED)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (key, desc) in *shortcuts {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<14}", key),
                    Style::default()
                        .fg(theme::ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::default().fg(theme::FG_PRIMARY)),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Press any key to close",
        Style::default().fg(theme::FG_DIMMED),
    )));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER_FOCUSED))
                    .title(" Help ")
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
