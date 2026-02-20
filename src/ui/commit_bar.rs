use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        " Commit: {}▌  (Enter to confirm · Esc to cancel)",
        app.commit_message
    );
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Green)),
        area,
    );
}
