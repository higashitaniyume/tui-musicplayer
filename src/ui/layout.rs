use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use super::{BG_DARK, BG_HINT, accent, dim, normal, playing, paused};
use super::{format_duration, settings, panels};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let main = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(2),
        Constraint::Length(1),
    ]).split(area);

    render_title(f, main[0]);

    if app.show_settings {
        settings::render_settings(f, main[1], app);
    } else {
        panels::render_content(f, main[1], app);
    }

    render_player_bar(f, main[2], app);

    if matches!(app.input_mode, crate::app::InputMode::EnteringPath { .. }) {
        let input_area = centered_rect(main[1], 60, 5);
        settings::render_input_bar(f, input_area, app);
        f.render_widget(
            Paragraph::new("").block(Block::new().style(Style::new().bg(BG_HINT))),
            main[3],
        );
    } else {
        render_key_hints(f, main[3], app);
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}

fn render_title(f: &mut Frame, area: Rect) {
    let t = Line::from(vec![
        Span::styled(" tui-musicplayer ", super::title_style()),
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
    use crate::audio::PlayerStatus;
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

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
    let hl = |s| Span::styled(s, Style::new().fg(Color::Black).bg(super::ACCENT));

    if app.show_settings {
        let add_label = match app.settings_section {
            crate::app::SettingsSection::Folders => loc.settings_hint_add_folder(),
            crate::app::SettingsSection::Playlists => loc.settings_hint_import_xspf(),
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                hl(" Tab "), Span::styled("Switch  ", normal()),
                hl(" A "), Span::styled(add_label, normal()),
                Span::styled("  ", dim()),
                hl(" I "), Span::styled(loc.settings_hint_import_xspf(), normal()),
                Span::styled("  ", dim()),
                hl(" D "), Span::styled(loc.settings_hint_remove(), normal()),
                Span::styled("  ", dim()),
                hl(" L "), Span::styled(loc.language_toggle_hint(), normal()),
                Span::styled("  ", dim()),
                hl(" Enter "), Span::styled(loc.settings_hint_rescan(), normal()),
                Span::styled("  ", dim()),
                hl(" Esc "), Span::styled(loc.settings_hint_back(), normal()),
            ])).block(Block::new().style(Style::new().bg(BG_HINT))),
            area,
        );
        return;
    }

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
            hl(" W "), Span::styled(loc.playlist_switch_hint(), normal()),
            hl(" F2 "), Span::styled(loc.settings_key_hint(), normal()),
            hl(" Q "), Span::styled(loc.hint_quit(), normal()),
        ])).block(Block::new().style(Style::new().bg(BG_HINT))),
        area,
    );
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
