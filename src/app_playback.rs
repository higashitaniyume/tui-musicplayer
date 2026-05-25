use std::time::{Duration, Instant};

use log::{debug, info, warn};

use crate::app::{App, FadeAction, FadeState};
use crate::player::{parse_lrc, PlayMode, PlayerStatus};

impl App {
    // ── Fade ──

    pub(crate) fn start_fade(&mut self, action: FadeAction, from: f32, to: f32, ms: u64) {
        debug!("Fade: from {:.0}% to {:.0}% over {ms}ms", from * 100.0, to * 100.0);
        self.fade = Some(FadeState {
            action, from_vol: from, to_vol: to,
            start: Instant::now(),
            duration: Duration::from_millis(ms),
        });
        self.player.set_volume(from);
    }

    pub(crate) fn update_fade(&mut self) {
        let completed = match &self.fade {
            Some(f) => {
                let t = (f.start.elapsed().as_secs_f32() / f.duration.as_secs_f32()).clamp(0.0, 1.0);
                self.player.set_volume(f.from_vol + (f.to_vol - f.from_vol) * t);
                t >= 1.0
            }
            None => return,
        };
        if completed {
            let f = self.fade.take().unwrap();
            self.player.set_volume(f.to_vol);
            match f.action {
                FadeAction::Pause => {
                    debug!("Fade complete: pause");
                    if let Some(start) = self.play_start.take() { self.accumulated += start.elapsed(); }
                    self.player.pause();
                    self.status = PlayerStatus::Paused;
                }
                FadeAction::Stop => {
                    debug!("Fade complete: stop");
                    self.player.stop();
                    self.status = PlayerStatus::Stopped;
                    self.reset_timing();
                }
                FadeAction::LoadTrack(idx) => {
                    debug!("Fade complete: load track index={idx}");
                    self.player.stop();
                    self.do_play_track(idx, true);
                }
                FadeAction::Quit => {
                    debug!("Fade complete: quit");
                    self.player.stop();
                    self.status = PlayerStatus::Stopped;
                    self.reset_timing();
                    self.should_quit = true;
                }
                FadeAction::None => {}
            }
        }
    }

    // ── Playback ──

    pub(crate) fn play_selected(&mut self) {
        let pl_idx = match self.playlist_state.selected() {
            Some(i) => i, None => return,
        };
        if pl_idx >= self.playlist.len() { return; }
        self.play_track(self.playlist[pl_idx]);
    }

    pub(crate) fn play_track(&mut self, lib_idx: usize) {
        self.do_play_track(lib_idx, false);
    }

    fn do_play_track(&mut self, lib_idx: usize, fade_in: bool) {
        if lib_idx >= self.library.len() { return; }
        let track = &self.library[lib_idx];
        info!("Playing track [{}]: {} - {}", lib_idx, track.display_title(), track.path.display());
        self.fade = None;
        self.player.stop();
        self.reset_timing();
        self.lyrics = parse_lrc(&self.library[lib_idx].path).unwrap_or_default();
        if !self.lyrics.is_empty() {
            debug!("  Lyrics: {} lines loaded", self.lyrics.len());
        }
        self.current_lyric_index = None;
        self.lyrics_state.select(Some(0));

        match self.player.load(&self.library[lib_idx].path, Duration::ZERO) {
            Ok(meta) => {
                self.duration = meta.duration;
                self.current_index = Some(lib_idx);
                self.status = PlayerStatus::Playing;
                self.play_start = Some(Instant::now());
                self.player.play();
                if fade_in {
                    self.start_fade(FadeAction::None, 0.0, self.volume, 300);
                } else {
                    self.player.set_volume(self.volume);
                }
                let track = &mut self.library[lib_idx];
                if track.duration.is_none() { track.duration = meta.duration; }
                track.sample_rate = Some(meta.sample_rate);
                track.channels = Some(meta.channels);
                if let (Some(dur), Some(size)) = (meta.duration, track.file_size) {
                    if !dur.is_zero() {
                        track.bitrate = Some((size as f64 * 8.0 / dur.as_secs_f64() / 1000.0) as u32);
                    }
                }
                self.track_meta = Some(meta);
            }
            Err(e) => {
                warn!("Failed to play track {}: {e}", self.library[lib_idx].path.display());
                self.status_message = format!("{}: {e}", self.locale.error_prefix());
            }
        }
    }

    pub(crate) fn toggle_pause(&mut self) {
        match self.status {
            PlayerStatus::Playing => {
                info!("Pause");
                self.start_fade(FadeAction::Pause, self.volume, 0.0, 250);
            }
            PlayerStatus::Paused => {
                info!("Resume");
                self.play_start = Some(Instant::now());
                self.player.play();
                self.status = PlayerStatus::Playing;
                self.start_fade(FadeAction::None, 0.0, self.volume, 250);
            }
            PlayerStatus::Stopped => {
                if self.focus == crate::app::Panel::Playlist { self.play_selected(); }
            }
        }
    }

    pub(crate) fn next_track(&mut self) {
        if let Some(idx) = self.compute_next_lib_index() {
            info!("Next track: index={idx}");
            self.start_fade(FadeAction::LoadTrack(idx), self.volume, 0.0, 200);
        } else {
            info!("Next track: end of playlist");
        }
    }

    pub(crate) fn prev_track(&mut self) {
        if let Some(idx) = self.compute_prev_lib_index() {
            info!("Previous track: index={idx}");
            self.start_fade(FadeAction::LoadTrack(idx), self.volume, 0.0, 200);
        }
    }

    fn compute_next_lib_index(&self) -> Option<usize> {
        if self.playlist.is_empty() { return None; }
        let cur = self.current_index?;
        let pos = self.playlist.iter().position(|&i| i == cur)?;
        match self.play_mode {
            PlayMode::RepeatOne => Some(cur),
            PlayMode::RepeatAll => Some(self.playlist[(pos + 1) % self.playlist.len()]),
            PlayMode::Sequential | PlayMode::Shuffle => {
                if pos + 1 < self.playlist.len() { Some(self.playlist[pos + 1]) } else { None }
            }
        }
    }

    fn compute_prev_lib_index(&self) -> Option<usize> {
        if self.playlist.is_empty() { return None; }
        let cur = self.current_index?;
        if self.elapsed > Duration::from_secs(3) { return Some(cur); }
        let pos = self.playlist.iter().position(|&i| i == cur)?;
        if pos > 0 { Some(self.playlist[pos - 1]) }
        else if self.play_mode == PlayMode::RepeatAll { Some(self.playlist[self.playlist.len() - 1]) }
        else { Some(self.playlist[0]) }
    }

    pub(crate) fn stop(&mut self) {
        info!("Stop requested");
        if self.status == PlayerStatus::Playing || self.status == PlayerStatus::Paused {
            self.start_fade(FadeAction::Stop, self.volume, 0.0, 200);
        } else {
            self.player.stop();
            self.status = PlayerStatus::Stopped;
            self.reset_timing();
        }
    }

    pub(crate) fn advance_track(&mut self) {
        if let Some(idx) = self.compute_next_lib_index() {
            debug!("Auto-advance to index={idx}");
            self.start_fade(FadeAction::LoadTrack(idx), self.volume, 0.0, 200);
        } else {
            info!("End of playlist reached, stopping");
            self.start_fade(FadeAction::Stop, self.volume, 0.0, 200);
        }
    }

    pub(crate) fn toggle_play_mode(&mut self) {
        self.play_mode = self.play_mode.next();
        info!("Play mode: {:?}", self.play_mode);
    }

    pub(crate) fn volume_up(&mut self) {
        self.volume = (self.volume + 0.05).min(1.0);
        self.player.set_volume(self.volume);
    }

    pub(crate) fn volume_down(&mut self) {
        self.volume = (self.volume - 0.05).max(0.0);
        self.player.set_volume(self.volume);
    }

    pub(crate) fn seek(&mut self, delta: f64) {
        if self.status == PlayerStatus::Stopped || self.current_index.is_none() { return; }
        let new = if delta < 0.0 {
            (self.elapsed.as_secs_f64() + delta).max(0.0)
        } else {
            let cap = self.duration.map(|d| d.as_secs_f64()).unwrap_or(f64::MAX);
            (self.elapsed.as_secs_f64() + delta).min(cap)
        };
        self.seek_to(new);
    }

    pub(crate) fn seek_to(&mut self, secs: f64) {
        if self.status == PlayerStatus::Stopped || self.current_index.is_none() { return; }
        debug!("Seek to {:.1}s", secs);
        self.fade = None;
        let lib_idx = self.current_index.unwrap();
        let seek = Duration::from_secs_f64(secs);
        self.player.stop();
        self.accumulated = seek;
        self.elapsed = seek;
        if let Err(e) = self.player.load(&self.library[lib_idx].path, seek) {
            warn!("Seek failed: {e}");
            self.status_message = format!("{}: {e}", self.locale.seek_error());
            return;
        }
        self.play_start = Some(std::time::Instant::now());
        self.player.play();
        self.status = PlayerStatus::Playing;
    }
}
