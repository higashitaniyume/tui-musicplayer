use std::path::PathBuf;

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use super::{move_list, select_at, App, InputMode, Panel, SettingsSection};

impl App {
    // ── Keyboard ──

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release { return; }

        // ── Text input mode ──
        if let InputMode::EnteringPath { ref buffer, cursor, ref purpose } = self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    let path = PathBuf::from(buffer.clone());
                    let purpose_clone = purpose.clone();
                    self.input_mode = InputMode::Normal;
                    match purpose_clone {
                        super::PathPurpose::AddMusicFolder => self.add_music_folder(path),
                        super::PathPurpose::ImportXspf => self.import_xspf_file(path),
                    }
                }
                KeyCode::Char(c) => {
                    self.input_mode = InputMode::EnteringPath {
                        buffer: {
                            let mut b = buffer.clone();
                            b.insert(cursor, c);
                            b
                        },
                        cursor: cursor + 1,
                        purpose: purpose.clone(),
                    };
                }
                KeyCode::Backspace => {
                    if cursor > 0 {
                        let mut b = buffer.clone();
                        b.remove(cursor - 1);
                        self.input_mode = InputMode::EnteringPath {
                            buffer: b,
                            cursor: cursor - 1,
                            purpose: purpose.clone(),
                        };
                    }
                }
                KeyCode::Delete => {
                    if cursor < buffer.len() {
                        let mut b = buffer.clone();
                        b.remove(cursor);
                        self.input_mode = InputMode::EnteringPath {
                            buffer: b,
                            cursor,
                            purpose: purpose.clone(),
                        };
                    }
                }
                KeyCode::Left => {
                    if cursor > 0 {
                        self.input_mode = InputMode::EnteringPath {
                            buffer: buffer.clone(),
                            cursor: cursor - 1,
                            purpose: purpose.clone(),
                        };
                    }
                }
                KeyCode::Right => {
                    if cursor < buffer.len() {
                        self.input_mode = InputMode::EnteringPath {
                            buffer: buffer.clone(),
                            cursor: cursor + 1,
                            purpose: purpose.clone(),
                        };
                    }
                }
                KeyCode::Home => {
                    self.input_mode = InputMode::EnteringPath {
                        buffer: buffer.clone(),
                        cursor: 0,
                        purpose: purpose.clone(),
                    };
                }
                KeyCode::End => {
                    self.input_mode = InputMode::EnteringPath {
                        buffer: buffer.clone(),
                        cursor: buffer.len(),
                        purpose: purpose.clone(),
                    };
                }
                _ => {}
            }
            return;
        }

        // ── Ctrl+C ──
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key.code {
                self.request_quit();
            }
            return;
        }

        // ── Settings mode keys ──
        if self.show_settings {
            match key.code {
                KeyCode::Esc | KeyCode::F(2) => self.toggle_settings(),
                KeyCode::Char('q') => self.request_quit(),
                KeyCode::Tab => {
                    self.settings_section = match self.settings_section {
                        SettingsSection::Folders => SettingsSection::Playlists,
                        SettingsSection::Playlists => SettingsSection::Devices,
                        SettingsSection::Devices => SettingsSection::Folders,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    match self.settings_section {
                        SettingsSection::Folders => move_list(
                            &mut self.settings_folder_state,
                            self.config.music_folders.len(),
                            -1,
                        ),
                        SettingsSection::Playlists => move_list(
                            &mut self.settings_xspf_state,
                            self.config.xspf_playlists.len(),
                            -1,
                        ),
                        SettingsSection::Devices => move_list(
                            &mut self.settings_device_state,
                            self.audio_device_list.len(),
                            -1,
                        ),
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    match self.settings_section {
                        SettingsSection::Folders => move_list(
                            &mut self.settings_folder_state,
                            self.config.music_folders.len(),
                            1,
                        ),
                        SettingsSection::Playlists => move_list(
                            &mut self.settings_xspf_state,
                            self.config.xspf_playlists.len(),
                            1,
                        ),
                        SettingsSection::Devices => move_list(
                            &mut self.settings_device_state,
                            self.audio_device_list.len(),
                            1,
                        ),
                    }
                }
                KeyCode::Char('a') if self.settings_section != SettingsSection::Devices => {
                    self.input_mode = InputMode::EnteringPath {
                        buffer: String::new(),
                        cursor: 0,
                        purpose: super::PathPurpose::AddMusicFolder,
                    };
                }
                KeyCode::Char('i') if self.settings_section != SettingsSection::Devices => {
                    self.input_mode = InputMode::EnteringPath {
                        buffer: String::new(),
                        cursor: 0,
                        purpose: super::PathPurpose::ImportXspf,
                    };
                }
                KeyCode::Char('d') if self.settings_section != SettingsSection::Devices => self.remove_settings_item(),
                KeyCode::Char('l') => self.toggle_language(),
                KeyCode::Enter => match self.settings_section {
                    SettingsSection::Devices => self.select_audio_device(),
                    _ => self.rescan_all(),
                },
                _ => {}
            }
            return;
        }

        // ── Normal mode keys ──
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::F(2) => self.toggle_settings(),
            KeyCode::Char('l') => {
                self.library_visible = !self.library_visible;
                if self.library_visible { self.focus = Panel::Library; }
                else if self.focus == Panel::Library { self.focus = Panel::Playlist; }
            }
            KeyCode::Char('a') if self.focus == Panel::Library => self.add_selected_to_playlist(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('n') => self.next_track(),
            KeyCode::Char('p') => self.prev_track(),
            KeyCode::Char('s') => self.stop(),
            KeyCode::Char('m') => self.toggle_play_mode(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.volume_up(),
            KeyCode::Char('-') => self.volume_down(),
            KeyCode::Char('d') if self.focus == Panel::Playlist => self.remove_from_playlist(),
            KeyCode::Char('w') => self.cycle_playlist(),
            KeyCode::Up | KeyCode::Char('k') => self.move_focus_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_focus_selection(1),
            KeyCode::Left => match self.focus {
                Panel::Lyrics => self.move_lyrics_selection(-5),
                _ => self.seek(-5.0),
            },
            KeyCode::Right => match self.focus {
                Panel::Lyrics => self.move_lyrics_selection(5),
                _ => self.seek(5.0),
            },
            _ => {}
        }
    }

    fn request_quit(&mut self) {
        use crate::audio::PlayerStatus;
        if self.status == PlayerStatus::Playing {
            self.start_fade(super::FadeAction::Quit, self.volume, 0.0, 300);
        } else if self.status == PlayerStatus::Paused {
            self.player.stop();
            self.status = PlayerStatus::Stopped;
            self.reset_timing();
            self.should_quit = true;
        } else {
            self.should_quit = true;
        }
    }

    fn cycle_focus(&mut self) {
        let panels = self.visible_panels();
        let idx = panels.iter().position(|&p| p == self.focus).unwrap_or(0);
        self.focus = panels[(idx + 1) % panels.len()];
    }

    fn handle_enter(&mut self) {
        match self.focus {
            Panel::Library => self.add_selected_to_playlist(),
            Panel::Playlist => self.play_selected(),
            Panel::Lyrics => {
                if let Some(phys) = self.lyrics_state.selected() {
                    if let Some(&log) = self.lyric_line_map.get(phys) {
                        if log < self.lyrics.len() {
                            let secs = self.lyrics[log].timestamp.as_secs_f64();
                            self.seek_to(secs);
                            self.current_lyric_index = Some(log);
                        }
                    }
                }
            }
        }
    }

    fn add_selected_to_playlist(&mut self) {
        if let Some(lib_idx) = self.library_state.selected() {
            if lib_idx < self.library.len() && !self.playlist.contains(&lib_idx) {
                self.playlist.push(lib_idx);
            }
        }
    }

    fn remove_from_playlist(&mut self) {
        let pl_idx = match self.playlist_state.selected() {
            Some(i) => i, None => return,
        };
        if pl_idx >= self.playlist.len() { return; }
        let lib_idx = self.playlist[pl_idx];
        if self.current_index == Some(lib_idx) { self.stop(); }
        self.playlist.remove(pl_idx);
        if self.playlist.is_empty() {
            self.playlist_state.select(None);
        } else if pl_idx >= self.playlist.len() {
            self.playlist_state.select(Some(self.playlist.len() - 1));
        }
    }

    fn move_focus_selection(&mut self, delta: i32) {
        match self.focus {
            Panel::Library => move_list(&mut self.library_state, self.library.len(), delta),
            Panel::Playlist => move_list(&mut self.playlist_state, self.playlist.len(), delta),
            Panel::Lyrics => self.move_lyrics_selection(delta),
        }
    }

    fn move_lyrics_selection(&mut self, delta: i32) {
        if self.lyrics.is_empty() || self.lyric_line_map.is_empty() { return; }
        let max = self.lyric_line_map.len();
        let cur = self.lyrics_state.selected().unwrap_or(0);
        let new = if delta < 0 {
            cur.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            (cur + delta as usize).min(max.saturating_sub(1))
        };
        self.lyrics_state.select(Some(new));
        self.lyrics_idle_ticks = 0;
    }

    // ── Mouse ──

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        let pos: ratatui::layout::Position = (mouse.column, mouse.row).into();
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                self.handle_click(pos);
            }
            MouseEventKind::ScrollUp => self.handle_scroll(pos, -3),
            MouseEventKind::ScrollDown => self.handle_scroll(pos, 3),
            _ => {}
        }
    }

    fn handle_scroll(&mut self, pos: ratatui::layout::Position, delta: i32) {
        if self.show_settings {
            if self.settings_folder_inner.contains(pos) {
                self.settings_section = SettingsSection::Folders;
                move_list(&mut self.settings_folder_state, self.config.music_folders.len(), delta);
            } else if self.settings_xspf_inner.contains(pos) {
                self.settings_section = SettingsSection::Playlists;
                move_list(&mut self.settings_xspf_state, self.config.xspf_playlists.len(), delta);
            }
            return;
        }
        if self.library_visible && self.library_inner.contains(pos) {
            self.focus = Panel::Library;
            move_list(&mut self.library_state, self.library.len(), delta);
        } else if self.playlist_inner.contains(pos) {
            self.focus = Panel::Playlist;
            move_list(&mut self.playlist_state, self.playlist.len(), delta);
        } else if self.lyrics_inner.contains(pos) {
            self.focus = Panel::Lyrics;
            self.move_lyrics_selection(delta);
        }
    }

    fn handle_click(&mut self, pos: ratatui::layout::Position) {
        if self.show_settings {
            if self.settings_folder_inner.contains(pos) {
                self.settings_section = SettingsSection::Folders;
                select_at(&mut self.settings_folder_state, &self.settings_folder_inner, pos.y, self.config.music_folders.len());
            } else if self.settings_xspf_inner.contains(pos) {
                self.settings_section = SettingsSection::Playlists;
                select_at(&mut self.settings_xspf_state, &self.settings_xspf_inner, pos.y, self.config.xspf_playlists.len());
            }
            self.handle_bar_clicks(pos);
            return;
        }

        if self.library_visible && self.library_inner.contains(pos) {
            self.focus = Panel::Library;
            select_at(&mut self.library_state, &self.library_inner, pos.y, self.library.len());
            return;
        }
        if self.playlist_inner.contains(pos) {
            self.focus = Panel::Playlist;
            select_at(&mut self.playlist_state, &self.playlist_inner, pos.y, self.playlist.len());
            if let Some(pl_idx) = self.playlist_state.selected() {
                if pl_idx < self.playlist.len() {
                    self.play_track(self.playlist[pl_idx]);
                }
            }
            return;
        }
        if self.lyrics_inner.contains(pos) {
            self.focus = Panel::Lyrics;
            let physical_len = self.lyric_line_map.len().max(1);
            select_at(&mut self.lyrics_state, &self.lyrics_inner, pos.y, physical_len);
            self.lyrics_idle_ticks = 0;
            return;
        }
        self.handle_bar_clicks(pos);
    }

    fn handle_bar_clicks(&mut self, pos: ratatui::layout::Position) {
        if self.progress_area != Rect::ZERO && self.progress_area.contains(pos) {
            let ratio = (pos.x.saturating_sub(self.progress_area.x)) as f64
                / self.progress_area.width.max(1) as f64;
            if let Some(d) = self.duration {
                self.seek_to(d.as_secs_f64() * ratio);
            }
            return;
        }
        if self.volume_area != Rect::ZERO && self.volume_area.contains(pos) {
            let ratio = (pos.x.saturating_sub(self.volume_area.x)) as f64
                / self.volume_area.width.max(1) as f64;
            self.volume = (ratio.clamp(0.0, 1.0)) as f32;
            self.player.set_volume(self.volume);
        }
    }
}
