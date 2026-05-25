use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Panel};
use crate::audio::PlayerStatus;
use super::{border_style, dim, normal, playing, title_span, truncate, viz};

// ═══════════════════════════════════════════════
// Content router
// ═══════════════════════════════════════════════

pub fn render_content(f: &mut Frame, area: Rect, app: &mut App) {
    if app.library_visible {
        let cols = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(55),
        ]).split(area);
        render_library(f, cols[0], app);
        render_playlist(f, cols[1], app);
        render_right_panel(f, cols[2], app);
    } else {
        let cols = Layout::horizontal([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ]).split(area);
        render_playlist(f, cols[0], app);
        render_right_panel(f, cols[1], app);
    }
}

// ═══════════════════════════════════════════════
// Library
// ═══════════════════════════════════════════════

fn render_library(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == Panel::Library;
    let title = app.locale.library_title(app.library_count());
    let block = Block::new()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(border_style(focused))
        .title(title_span(&title));
    app.library_inner = block.inner(area);

    if app.library.is_empty() {
        f.render_widget(
            List::new(vec![ListItem::from(Span::styled(app.locale.no_tracks_hint(), dim()))]).block(block),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app.library.iter().enumerate().map(|(i, t)| {
        let in_pl = app.playlist.contains(&i);
        let txt = format!("{}{:3}. {}", if in_pl { "✓" } else { " " }, i + 1, truncate(&t.display_title(), 15));
        ListItem::from(txt).style(normal())
    }).collect();

    f.render_stateful_widget(
        List::new(items).block(block).highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        area, app.library_state_mut(),
    );
}

// ═══════════════════════════════════════════════
// Playlist
// ═══════════════════════════════════════════════

fn render_playlist(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == Panel::Playlist;
    let title = match app.active_playlist_idx {
        Some(idx) if idx < app.saved_playlists.len() =>
            app.locale.playlist_title_named(&app.saved_playlists[idx].0, app.playlist_count()),
        _ => app.locale.playlist_title(app.playlist_count()),
    };
    let block = Block::new()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(border_style(focused))
        .title(title_span(&title));
    app.playlist_inner = block.inner(area);

    if app.playlist.is_empty() {
        f.render_widget(
            List::new(vec![ListItem::from(Span::styled(app.locale.playlist_empty_hint(), dim()))]).block(block),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app.playlist.iter().enumerate().map(|(pi, &li)| {
        let is_current = app.current_index == Some(li);
        let prefix = if is_current {
            match app.status {
                PlayerStatus::Playing => "▶",
                PlayerStatus::Paused => "⏸",
                PlayerStatus::Stopped => "■",
            }
        } else { " " };
        let t = &app.library[li];
        let txt = format!("{}{:2}. {}", prefix, pi + 1, truncate(&t.display_title(), 18));
        let s = if is_current { playing() } else { normal() };
        ListItem::from(txt).style(s)
    }).collect();

    f.render_stateful_widget(
        List::new(items).block(block).highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        area, app.playlist_state_mut(),
    );
}

// ═══════════════════════════════════════════════
// Right panel: Track Info + Lyrics + File badge
// ═══════════════════════════════════════════════

fn render_right_panel(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::new()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(dim())
        .title(title_span(" Now Playing "));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.current_track().is_none() {
        f.render_widget(
            Paragraph::new(Span::styled(app.locale.info_no_track(), dim())).alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let (track_title, track_subtitle, track_badge) = {
        let t = app.current_track().unwrap();
        let title = t.title.clone();
        let subtitle = if t.artist.is_empty() {
            t.file_stem()
        } else {
            format!("{} · {}", t.artist, t.album)
        };
        let badge = format!(
            "{} · {} · {} · {} │ {}",
            t.format,
            t.bitrate.map(|b| format!("{b} kbps")).unwrap_or_else(|| "?".into()),
            t.sample_rate.map(|s| format!("{s} Hz")).unwrap_or_else(|| "?".into()),
            t.format_size(),
            t.format_duration(),
        );
        (title, subtitle, badge)
    };

    let right = Layout::vertical([
        Constraint::Length(2),   // track info
        Constraint::Min(8),      // viz + lyrics
        Constraint::Length(1),   // file badge
    ]).split(inner);

    f.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(truncate(&track_title, 42), Style::new().fg(Color::White).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(track_subtitle, normal())),
        ])),
        right[0],
    );

    // Split middle: visualization (top half) + lyrics (bottom half)
    let middle = Layout::vertical([
        Constraint::Ratio(2, 5),  // viz gets 2/5
        Constraint::Ratio(3, 5),  // lyrics get 3/5
    ]).split(right[1]);

    render_viz_section(f, middle[0], app);
    render_lyrics_section(f, middle[1], app);

    f.render_widget(
        Paragraph::new(Span::styled(track_badge, dim())),
        right[2],
    );
}

// ═══════════════════════════════════════════════
// Visualization section
// ═══════════════════════════════════════════════

fn render_viz_section(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::new()
        .borders(Borders::ALL).border_type(BorderType::Plain)
        .border_style(dim())
        .title(title_span(" ♪ "));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 4 || inner.height < 3 {
        return;
    }

    let elapsed = if app.status == PlayerStatus::Playing {
        app.elapsed
    } else {
        std::time::Duration::ZERO
    };

    f.render_widget(viz::VizWidget { elapsed }, inner);
}

// ═══════════════════════════════════════════════
// Lyrics section
// ═══════════════════════════════════════════════

fn render_lyrics_section(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == Panel::Lyrics;
    let block = Block::new()
        .borders(Borders::ALL).border_type(BorderType::Plain)
        .border_style(border_style(focused))
        .title(title_span(app.locale.lyrics_title()));
    let inner = block.inner(area);
    app.lyrics_inner = inner;
    f.render_widget(block, area);

    if app.lyrics.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(app.locale.no_lyrics(), dim())).alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let playing_idx = app.current_lyric_index;
    let max_width = inner.width.max(1) as usize;
    let mut items: Vec<ListItem> = Vec::new();
    app.lyric_line_map.clear();

    for (i, l) in app.lyrics.iter().enumerate() {
        let wrapped = wrap_text(&l.text, max_width.saturating_sub(2));
        let s = if Some(i) == playing_idx {
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else { dim() };
        for line in wrapped {
            items.push(ListItem::from(line).style(s));
            app.lyric_line_map.push(i);
        }
    }

    f.render_stateful_widget(
        List::new(items).block(Block::new()).highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        inner, app.lyrics_state_mut(),
    );
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 { return vec![text.to_string()]; }
    let mut lines = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.chars().count() <= max_width {
            lines.push(remaining.to_string());
            break;
        }
        let mut split = max_width;
        let mut found_space = false;
        for (j, c) in remaining.char_indices() {
            if j >= max_width { break; }
            if c == ' ' || c == '\u{3000}' {
                split = j;
                found_space = true;
            }
        }
        if !found_space {
            split = remaining
                .char_indices()
                .take(max_width)
                .last()
                .map(|(j, _)| j + 1)
                .unwrap_or(max_width);
        }
        let (first, rest) = remaining.split_at(split);
        lines.push(first.trim_end().to_string());
        remaining = rest.trim_start();
    }
    lines
}
