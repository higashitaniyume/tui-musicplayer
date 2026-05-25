use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui_panels;

const ACCENT: Color = Color::Cyan;
const BG_DARK: Color = Color::Rgb(20, 20, 40);
const BG_HINT: Color = Color::Rgb(30, 30, 50);

const fn title_style() -> Style { Style::new().fg(Color::White).add_modifier(Modifier::BOLD) }
pub(crate) fn normal() -> Style { Style::new().fg(Color::Gray) }
pub(crate) fn dim() -> Style { Style::new().fg(Color::DarkGray) }
pub(crate) fn accent() -> Style { Style::new().fg(ACCENT) }
pub(crate) fn playing() -> Style { Style::new().fg(Color::Green) }
pub(crate) fn paused() -> Style { Style::new().fg(Color::Yellow) }
pub(crate) fn border_style(focused: bool) -> Style { Style::new().fg(if focused { ACCENT } else { Color::DarkGray }) }
pub(crate) fn title_span(text: &str) -> Span<'_> { Span::styled(text, accent().add_modifier(Modifier::BOLD)) }

// ═══════════════════════════════════════
// Main layout
// ═══════════════════════════════════════

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let main = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(2),
        Constraint::Length(1),
    ]).split(area);

    render_title(f, main[0]);
    ui_panels::render_content(f, main[1], app);
    render_player_bar(f, main[2], app);
    render_key_hints(f, main[3], app);
}

fn render_title(f: &mut Frame, area: Rect) {
    let t = Line::from(vec![
        Span::styled(" tui-musicplayer ", title_style()),
        Span::styled("v0.1.0", dim()),
    ]);
    f.render_widget(
        Paragraph::new("").block(Block::new().style(Style::new().bg(BG_DARK)).title_top(t.right_aligned())),
        area,
    );
}

// ═══════════════════════════════════════
// Player bar (2 rows)
// ═══════════════════════════════════════

fn render_player_bar(f: &mut Frame, area: Rect, app: &mut App) {
    use crate::player::PlayerStatus;
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    // Row 1: Status + progress
    let status = match app.status {
        PlayerStatus::Playing => Span::styled(format!("▶ {}", app.locale.playing()), playing()),
        PlayerStatus::Paused => Span::styled(format!("⏸ {}", app.locale.paused()), paused()),
        PlayerStatus::Stopped => Span::styled(format!("■ {}", app.locale.stopped()), dim()),
    };
    let pos = app.current_playlist_position()
        .map(|p| format!("{}/{}", p + 1, app.playlist_count()))
        .unwrap_or_else(|| "-/-".into());
    let pbar = render_progress(app, rows[0].width.saturating_sub(50).max(10) as usize);
    let time = format!(
        " {} / {} ",
        format_duration(app.elapsed),
        app.duration.map(format_duration).unwrap_or_else(|| "--:--".into())
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            status,
            Span::styled(" │ ", dim()),
            Span::styled(pos, normal()),
            Span::styled(" │ ", dim()),
            Span::styled(&pbar, Style::new().fg(Color::Green)),
            Span::styled(&time, normal()),
            Span::styled(" │ ", dim()),
            Span::styled(app.locale.mode_name(&app.play_mode), accent()),
        ])).block(Block::new().style(Style::new().bg(BG_DARK))),
        rows[0],
    );

    // Row 2: Controls
    let vol_bar = render_volume_bar(app.volume, 16);
    let vol_pct = (app.volume * 100.0) as u32;
    let seek_label = match app.focus {
        crate::app::Panel::Lyrics => match app.locale.lang() {
            crate::locale::Lang::En => "←→:Scroll  Enter:Seek",
            crate::locale::Lang::ZhCn => "←→:滚动  Enter:跳转",
        },
        _ => match app.locale.lang() {
            crate::locale::Lang::En => "←→:Seek ±5s",
            crate::locale::Lang::ZhCn => "←→:快进/退 5秒",
        },
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◀◀  ", if app.status != PlayerStatus::Stopped { normal() } else { dim() }),
            match app.status {
                PlayerStatus::Playing => Span::styled(" ▶  ", playing()),
                PlayerStatus::Paused => Span::styled(" ⏸  ", paused()),
                PlayerStatus::Stopped => Span::styled(" ▶  ", dim()),
            },
            Span::styled(" ■  ", if app.status != PlayerStatus::Stopped { normal() } else { dim() }),
            Span::styled(" ▷▷  ", if app.status != PlayerStatus::Stopped { normal() } else { dim() }),
            Span::styled(format!("{vol_bar} {vol_pct}%  "), dim()),
            Span::styled(seek_label, dim()),
        ])).block(Block::new().style(Style::new().bg(BG_DARK))),
        rows[1],
    );

    app.progress_area = Rect { x: rows[0].x, y: rows[0].y, width: rows[0].width, height: 1 };
    app.volume_area = Rect { x: rows[1].x + 16, y: rows[1].y, width: 18, height: 1 };
}

// ═══════════════════════════════════════
// Key hints
// ═══════════════════════════════════════

fn render_key_hints(f: &mut Frame, area: Rect, app: &App) {
    let loc = &app.locale;
    let hl = |s| Span::styled(s, Style::new().fg(Color::Black).bg(ACCENT));
    let enter_label = match app.focus {
        crate::app::Panel::Library => if matches!(loc.lang(), crate::locale::Lang::En) { "Add" } else { "添加" },
        crate::app::Panel::Playlist => if matches!(loc.lang(), crate::locale::Lang::En) { "Play" } else { "播放" },
        crate::app::Panel::Lyrics => if matches!(loc.lang(), crate::locale::Lang::En) { "Seek" } else { "跳转" },
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            hl(" Tab "), Span::styled("Switch  ", normal()),
            hl(" Enter "), Span::styled(enter_label, normal()),
            Span::styled("  ", dim()),
            hl(" Space "), Span::styled(loc.hint_pause(), normal()),
            hl(" N "), Span::styled(loc.hint_next(), normal()),
            hl(" P "), Span::styled(loc.hint_prev(), normal()),
            hl(" S "), Span::styled(loc.hint_stop(), normal()),
            hl(" M "), Span::styled(loc.hint_mode(), normal()),
            hl(" L "), Span::styled("Library  ", normal()),
            hl(" A "), Span::styled("Add  ", normal()),
            hl(" D "), Span::styled("Del  ", normal()),
            hl(" Q "), Span::styled(loc.hint_quit(), normal()),
        ])).block(Block::new().style(Style::new().bg(BG_HINT))),
        area,
    );
}

// ═══════════════════════════════════════
// Shared helpers
// ═══════════════════════════════════════

pub fn format_duration(d: std::time::Duration) -> String {
    let total = d.as_secs();
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max { format!("{}…", s.chars().take(max - 1).collect::<String>()) } else { s.to_string() }
}

fn render_volume_bar(volume: f32, width: usize) -> String {
    let filled = (volume.clamp(0.0, 1.0) * width as f32) as usize;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(width - filled))
}

fn render_progress(app: &App, bar_width: usize) -> String {
    let p = match app.duration {
        Some(d) if !d.is_zero() => (app.elapsed.as_secs_f64() / d.as_secs_f64()).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let filled = (p * bar_width as f64) as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled))
}
