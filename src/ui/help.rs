use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, _app: &App) {
    let area = centered_rect(72, 28, frame.area());

    let rows: &[(&str, &str)] = &[
        ("h/l or Tab", "Switch dashboard section"),
        ("1..8", "Jump to specific section"),
        ("j / ↓", "Move down"),
        ("k / ↑", "Move up"),
        ("x", "Run selected action"),
        ("r", "Force refresh now"),
        ("/", "Filter search (Repos section)"),
        ("Enter", "Open repo in editor (Repos)"),
        ("o", "Open repo in file manager (Repos)"),
        ("f", "Git fetch (Repos)"),
        ("p", "Git pull (Repos)"),
        ("P", "Git push (Repos)"),
        ("c", "Commit tracked changes (Repos)"),
        ("g", "Toggle group by directory (Repos)"),
        ("A", "Toggle actionable-only repo mode (Repos)"),
        ("s", "Setup — change watch dirs"),
        ("?", "Toggle this help"),
        ("q / Ctrl-C", "Quit"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            " Dashboard Shortcuts ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for (key, desc) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<16}", key), Style::default().fg(Color::Yellow)),
            Span::raw(*desc),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Press any key to close",
        Style::default().fg(Color::DarkGray),
    )]));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title(" Help ")
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        area,
    );
}

/// Compute a centered Rect of fixed character dimensions within `area`.
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
