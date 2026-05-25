use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::app::{move_list, select_at, App, Panel};

impl App {
    // ── Keyboard ──

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release { return; }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key.code {
                self.request_quit();
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Char('l') => {
                self.library_visible = !self.library_visible;
                if self.library_visible { self.focus = Panel::Library; }
                else if self.focus == Panel::Library { self.focus = Panel::Playlist; }
            }
            KeyCode::Char('i') => self.import_xspf_dialog(),
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
        use crate::player::PlayerStatus;
        if self.status == PlayerStatus::Playing {
            self.start_fade(crate::app::FadeAction::Quit, self.volume, 0.0, 300);
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
                if let Some(idx) = self.lyrics_state.selected() {
                    if idx < self.lyrics.len() {
                        let secs = self.lyrics[idx].timestamp.as_secs_f64();
                        self.seek_to(secs);
                        self.current_lyric_index = Some(idx);
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
        if self.lyrics.is_empty() { return; }
        let cur = self.lyrics_state.selected().unwrap_or(0);
        let new = if delta < 0 {
            cur.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            (cur + delta as usize).min(self.lyrics.len().saturating_sub(1))
        };
        self.lyrics_state.select(Some(new));
        self.lyrics_idle_ticks = 0;
    }

    fn import_xspf_dialog(&mut self) {
        self.status_message = match self.locale.lang() {
            crate::locale::Lang::En => "Use: tui-musicplayer playlist.xspf".into(),
            crate::locale::Lang::ZhCn => "用法: tui-musicplayer playlist.xspf".into(),
        };
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
            select_at(&mut self.lyrics_state, &self.lyrics_inner, pos.y, self.lyrics.len());
            self.lyrics_idle_ticks = 0;
            return;
        }
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
