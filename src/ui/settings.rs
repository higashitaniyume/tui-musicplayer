use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, SettingsSection};
use super::{border_style, dim, normal, title_span, truncate};

pub fn render_settings(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style(true))
        .title(title_span(app.locale.settings_title()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let sections = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Percentage(30),
        Constraint::Percentage(36),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    render_folders_section(f, sections[0], app);
    render_xspf_section(f, sections[1], app);
    render_devices_section(f, sections[2], app);
    render_audio_host_line(f, sections[3], app);
    render_language_line(f, sections[4], app);
}

fn render_folders_section(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.settings_section == SettingsSection::Folders;
    let title_text = app.locale.music_folders_title(app.config.music_folders.len());
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(border_style(focused))
        .title(title_span(&title_text));
    let inner = block.inner(area);
    app.settings_folder_inner = inner;
    f.render_widget(block, area);

    if app.config.music_folders.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(app.locale.settings_no_folders(), dim()))
                .block(Block::new()),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .config
        .music_folders
        .iter()
        .map(|p| {
            let txt = truncate(&p.display().to_string(), 60);
            ListItem::from(txt).style(normal())
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::new())
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        inner,
        &mut app.settings_folder_state,
    );
}

fn render_devices_section(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.settings_section == SettingsSection::Devices;
    let current = app.config.audio_host.as_deref().unwrap_or("-");
    let title_text = format!(" Audio Output [{}] ", if current == "-" { "default" } else { current });
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(border_style(focused))
        .title(title_span(&title_text));
    let inner = block.inner(area);
    app.settings_device_inner = inner;
    f.render_widget(block, area);

    if app.audio_device_list.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("No devices found", dim())).block(Block::new()),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = app.audio_device_list.iter().map(|(host, name, _current)| {
        let is_current = Some(name.as_str()) == app.config.audio_host.as_deref();
        let prefix = if is_current { "✓ " } else { "  " };
        let txt = format!("{}{} [{}]", prefix, truncate(name, 40), host);
        let s = if is_current { super::accent() } else { normal() };
        ListItem::from(txt).style(s)
    }).collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::new())
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        inner,
        &mut app.settings_device_state,
    );
}

fn render_xspf_section(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.settings_section == SettingsSection::Playlists;
    let title_text = app.locale.xspf_playlists_title(app.config.xspf_playlists.len());
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(border_style(focused))
        .title(title_span(&title_text));
    let inner = block.inner(area);
    app.settings_xspf_inner = inner;
    f.render_widget(block, area);

    if app.config.xspf_playlists.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(app.locale.settings_no_xspf(), dim()))
                .block(Block::new()),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .config
        .xspf_playlists
        .iter()
        .map(|p| {
            let txt = truncate(&p.display().to_string(), 60);
            ListItem::from(txt).style(normal())
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::new())
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan)),
        inner,
        &mut app.settings_xspf_state,
    );
}

fn render_audio_host_line(f: &mut Frame, area: Rect, app: &App) {
    let current = app.config.audio_host.as_deref().unwrap_or("-");
    let text = format!(
        " {}: {}  |  Enter: select  Tab: switch section",
        app.locale.audio_host_label(),
        if current == "-" { "System Default" } else { current },
    );
    f.render_widget(
        Paragraph::new(Span::styled(text, dim())),
        area,
    );
}

fn render_language_line(f: &mut Frame, area: Rect, app: &App) {
    let text = format!(
        " {}: {}  |  L {}",
        app.locale.language_label(),
        app.locale.language_value(),
        app.locale.language_toggle_hint(),
    );
    f.render_widget(
        Paragraph::new(Span::styled(text, dim())),
        area,
    );
}

pub fn render_input_bar(f: &mut Frame, area: Rect, app: &App) {
    if let crate::app::InputMode::EnteringPath { buffer, cursor, purpose } = &app.input_mode {
        let prompt = match purpose {
            crate::app::PathPurpose::AddMusicFolder => app.locale.input_prompt_folder(),
            crate::app::PathPurpose::ImportXspf => app.locale.input_prompt_xspf(),
        };

        let display: String = if *cursor >= buffer.len() {
            format!("{buffer}█")
        } else {
            let before: String = buffer.chars().take(*cursor).collect();
            let at: String = buffer.chars().skip(*cursor).take(1).collect();
            let after: String = buffer.chars().skip(*cursor + 1).collect();
            format!("{before}█{at}{after}")
        };

        let text = format!("> {display}");
        let hint = app.locale.input_confirm_hint();

        let lines = vec![
            Line::from(Span::styled(
                format!(" {prompt} "),
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(&text, normal())),
            Line::from(Span::styled(format!(" {hint}"), dim())),
        ];

        f.render_widget(
            Paragraph::new(lines).block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(Color::Yellow)),
            ),
            area,
        );
    }
}
