use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType},
};

// ─── Background tones ──────────────────────────────────────────────────────
pub const BG_HIGHLIGHT: Color = Color::Rgb(38, 40, 52); // selected row
pub const BG_SECONDARY: Color = Color::Rgb(30, 32, 42); // status bar / sidebar
pub const BG_ELEVATED: Color = Color::Rgb(44, 46, 60); // modals / cards
pub const BG_ALT_ROW: Color = Color::Rgb(28, 30, 38); // alternating rows

// ─── Foreground tones ──────────────────────────────────────────────────────
pub const FG_PRIMARY: Color = Color::Rgb(220, 222, 232); // main text
pub const FG_SECONDARY: Color = Color::Rgb(160, 164, 180); // labels / muted text
pub const FG_DIMMED: Color = Color::Rgb(100, 104, 120); // borders / category headers

// ─── Border tones ──────────────────────────────────────────────────────────
pub const BORDER_NORMAL: Color = Color::Rgb(60, 64, 78); // default borders
pub const BORDER_FOCUSED: Color = Color::Rgb(120, 140, 200); // active panel

// ─── Accent colors ────────────────────────────────────────────────────────
pub const ACCENT_BLUE: Color = Color::Rgb(120, 150, 255); // primary accent
pub const ACCENT_CYAN: Color = Color::Rgb(100, 210, 220); // actions / keys
pub const ACCENT_GREEN: Color = Color::Rgb(120, 210, 130); // healthy / clean
pub const ACCENT_YELLOW: Color = Color::Rgb(230, 200, 100); // warning
pub const ACCENT_RED: Color = Color::Rgb(230, 100, 100); // critical / error
pub const ACCENT_ORANGE: Color = Color::Rgb(230, 160, 80); // high priority
pub const ACCENT_PINK: Color = Color::Rgb(200, 140, 200); // stash / misc
pub const ACCENT_PURPLE: Color = Color::Rgb(160, 130, 220); // group headers

// ─── Prebuilt styles ───────────────────────────────────────────────────────

pub fn style_header() -> Style {
    Style::default()
        .fg(FG_SECONDARY)
        .add_modifier(Modifier::BOLD)
}

pub fn style_row_highlight() -> Style {
    Style::default()
        .bg(BG_HIGHLIGHT)
        .fg(FG_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn style_table_alt_row() -> Style {
    Style::default().bg(BG_ALT_ROW)
}

// ─── Block builders ────────────────────────────────────────────────────────

pub fn block_default(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_NORMAL))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(FG_SECONDARY))
}

pub fn block_focused(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_FOCUSED))
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(ACCENT_BLUE)
                .add_modifier(Modifier::BOLD),
        )
}

// ─── Color mappers ─────────────────────────────────────────────────────────

pub fn severity_color(severity: &str) -> Color {
    match severity {
        "critical" => ACCENT_RED,
        "high" => ACCENT_ORANGE,
        "warn" | "warning" => ACCENT_YELLOW,
        "info" => ACCENT_BLUE,
        _ => FG_SECONDARY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_critical() {
        assert_eq!(severity_color("critical"), ACCENT_RED);
    }

    #[test]
    fn severity_high() {
        assert_eq!(severity_color("high"), ACCENT_ORANGE);
    }

    #[test]
    fn severity_warn_variants() {
        assert_eq!(severity_color("warn"), ACCENT_YELLOW);
        assert_eq!(severity_color("warning"), ACCENT_YELLOW);
    }

    #[test]
    fn severity_info() {
        assert_eq!(severity_color("info"), ACCENT_BLUE);
    }

    #[test]
    fn severity_unknown_falls_back() {
        assert_eq!(severity_color("debug"), FG_SECONDARY);
        assert_eq!(severity_color(""), FG_SECONDARY);
    }
}
