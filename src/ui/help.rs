use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, _app: &App) {
    let area = centered_rect(54, 24, frame.area());

    let rows: &[(&str, &str)] = &[
        ("j / ↓", "Move down"),
        ("k / ↑", "Move up"),
        ("Enter", "Open in editor"),
        ("o", "Open in file manager"),
        ("r", "Force refresh"),
        ("f", "Git fetch (background)"),
        ("p", "Git pull from remote"),
        ("P", "Git push to remote"),
        ("c", "Commit all changes"),
        ("g", "Toggle group by directory"),
        ("/", "Filter / search"),
        ("Esc", "Cancel filter / commit"),
        ("s", "Setup — change watch dirs"),
        ("?", "Toggle this help"),
        ("q / Ctrl-C", "Quit"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for (key, desc) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<14}", key), Style::default().fg(Color::Yellow)),
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
