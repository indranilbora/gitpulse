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
        Span::styled(" / ", Style::default().fg(theme::ACCENT_CYAN)),
        Span::styled(&app.filter_text, Style::default().fg(theme::FG_PRIMARY)),
        Span::styled("▌", Style::default().fg(theme::ACCENT_BLUE)),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG_SECONDARY)),
        area,
    );
}
