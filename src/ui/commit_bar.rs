use super::theme;
use crate::app::App;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" Commit: ", Style::default().fg(theme::ACCENT_GREEN)),
        Span::styled(&app.commit_message, Style::default().fg(theme::FG_PRIMARY)),
        Span::styled("▌", Style::default().fg(theme::ACCENT_BLUE)),
        Span::styled(
            "  Enter to confirm · Esc to cancel",
            Style::default().fg(theme::FG_DIMMED),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG_SECONDARY)),
        area,
    );
}
