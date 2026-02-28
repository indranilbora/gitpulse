use super::theme;
use crate::app::App;
use crate::dashboard::DashboardSection;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let all = DashboardSection::all();
    let mut items: Vec<ListItem> = Vec::new();
    let mut current_category: Option<&str> = None;

    for (idx, section) in all.iter().enumerate() {
        let cat = section.category();

        // Insert category header when category changes
        if current_category != Some(cat) {
            // Add spacing before non-first categories
            if current_category.is_some() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {}", cat),
                Style::default()
                    .fg(theme::FG_DIMMED)
                    .add_modifier(Modifier::BOLD),
            ))));
            current_category = Some(cat);
        }

        let is_active = *section == app.section;
        let count = app.section_row_count(*section);

        let indicator = if is_active { "▸" } else { " " };
        let num = idx + 1;
        let label = section.title();

        // Build the label portion
        let count_str = count.to_string();
        // Compute padding so count aligns to right edge
        // Area inner width minus border (2) minus left padding (1)
        let inner_width = area.width.saturating_sub(2) as usize;
        let label_part = format!(" {} {}. {}", indicator, num, label);
        let padding = inner_width
            .saturating_sub(label_part.len())
            .saturating_sub(count_str.len())
            .saturating_sub(1); // 1 for trailing space

        let line = if is_active {
            Line::from(vec![
                Span::styled(
                    label_part,
                    Style::default()
                        .fg(theme::ACCENT_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ".repeat(padding)),
                Span::styled(
                    count_str,
                    Style::default().fg(theme::FG_DIMMED),
                ),
                Span::raw(" "),
            ])
        } else {
            Line::from(vec![
                Span::styled(label_part, Style::default().fg(theme::FG_SECONDARY)),
                Span::raw(" ".repeat(padding)),
                Span::styled(
                    count_str,
                    Style::default().fg(theme::FG_DIMMED),
                ),
                Span::raw(" "),
            ])
        };

        items.push(ListItem::new(line));
    }

    let list = List::new(items).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::BORDER_NORMAL))
            .title(" AgentPulse ")
            .title_style(
                Style::default()
                    .fg(theme::ACCENT_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(list, area);
}
