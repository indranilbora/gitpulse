use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Show filter prompt with a simulated cursor
    let text = format!(" / {}▌", app.filter_text);
    let para = Paragraph::new(text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(para, area);
}
