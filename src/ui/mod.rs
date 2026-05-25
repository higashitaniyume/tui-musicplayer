use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub mod layout;
pub mod panels;
pub mod settings;

pub use layout::render;

pub(crate) const ACCENT: Color = Color::Cyan;
pub(crate) const BG_DARK: Color = Color::Rgb(20, 20, 40);
pub(crate) const BG_HINT: Color = Color::Rgb(30, 30, 50);

pub(crate) const fn title_style() -> Style { Style::new().fg(Color::White).add_modifier(Modifier::BOLD) }
pub(crate) fn normal() -> Style { Style::new().fg(Color::Gray) }
pub(crate) fn dim() -> Style { Style::new().fg(Color::DarkGray) }
pub(crate) fn accent() -> Style { Style::new().fg(ACCENT) }
pub(crate) fn playing() -> Style { Style::new().fg(Color::Green) }
pub(crate) fn paused() -> Style { Style::new().fg(Color::Yellow) }
pub(crate) fn border_style(focused: bool) -> Style { Style::new().fg(if focused { ACCENT } else { Color::DarkGray }) }
pub(crate) fn title_span(text: &str) -> Span<'_> { Span::styled(text, accent().add_modifier(Modifier::BOLD)) }

// ── Shared helpers ──

pub fn format_duration(d: std::time::Duration) -> String {
    let total = d.as_secs();
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max { format!("{}…", s.chars().take(max - 1).collect::<String>()) } else { s.to_string() }
}
