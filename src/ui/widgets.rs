use super::theme;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Row, Table, TableState},
    Frame,
};

/// Render a stat card: bordered box with a big centered number, label below, and colored dot.
pub fn render_stat_card(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    dot_color: ratatui::style::Color,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_NORMAL));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    // Top line: big value
    let value_line = Line::from(vec![Span::styled(
        value.to_string(),
        Style::default()
            .fg(theme::FG_PRIMARY)
            .add_modifier(Modifier::BOLD),
    )])
    .alignment(Alignment::Center);

    // Bottom line: label + dot
    let label_line = Line::from(vec![
        Span::styled(format!("{} ", label), Style::default().fg(theme::FG_SECONDARY)),
        Span::styled("●", Style::default().fg(dot_color)),
    ])
    .alignment(Alignment::Center);

    let chunks =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);

    frame.render_widget(Paragraph::new(value_line), chunks[0]);
    frame.render_widget(Paragraph::new(label_line), chunks[1]);
}

/// Render a themed table with selection, rounded borders, alternating rows, and bg-based highlight.
#[allow(clippy::too_many_arguments)]
pub fn render_styled_table<const N: usize>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    header: Row<'_>,
    rows: Vec<Row<'_>>,
    widths: [Constraint; N],
    selected: usize,
    len: usize,
) {
    let styled_rows: Vec<Row> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            if i % 2 == 1 {
                row.style(theme::style_table_alt_row())
            } else {
                row
            }
        })
        .collect();

    let table = Table::new(styled_rows, widths)
        .header(header)
        .block(theme::block_focused(title))
        .row_highlight_style(theme::style_row_highlight());

    let mut state = TableState::default();
    state.select(Some(selected.min(len.saturating_sub(1))));
    frame.render_stateful_widget(table, area, &mut state);
}

/// Build a pair of spans for a keyboard hint: accent-colored key + muted description.
pub fn key_hint<'a>(key: &'a str, desc: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            key,
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {} ", desc), Style::default().fg(theme::FG_DIMMED)),
    ]
}

/// Render a styled empty state with centered icon and message.
pub fn render_empty_state(frame: &mut Frame, area: Rect, icon: &str, message: &str) {
    let text = format!("{}\n{}", icon, message);
    frame.render_widget(
        Paragraph::new(text)
            .block(theme::block_default(""))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme::FG_DIMMED)),
        area,
    );
}

/// Format a number with comma separators (e.g., 1234567 -> "1,234,567").
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_zero() {
        assert_eq!(format_number(0), "0");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(12_345), "12,345");
        assert_eq!(format_number(999_999), "999,999");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_000_000), "1,000,000");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn format_number_large() {
        assert_eq!(format_number(1_000_000_000), "1,000,000,000");
    }
}
