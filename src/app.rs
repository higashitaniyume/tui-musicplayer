use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::{layout::Rect, widgets::ListState};

use crate::locale::Locale;
use crate::player::{
    is_audio_file, parse_xspf, scan_directory, AudioMeta, LyricLine, PlayMode,
    PlayerEngine, PlayerStatus, Track,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Panel {
    Library,
    Playlist,
    Lyrics,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FadeAction {
    None,
    Pause,
    Stop,
    LoadTrack(usize),
    Quit,
}

pub(crate) struct FadeState {
    pub(crate) action: FadeAction,
    pub(crate) from_vol: f32,
    pub(crate) to_vol: f32,
    pub(crate) start: Instant,
    pub(crate) duration: Duration,
}

pub struct App {
    pub library: Vec<Track>,
    pub playlist: Vec<usize>,
    pub current_index: Option<usize>,

    pub status: PlayerStatus,
    pub play_mode: PlayMode,
    pub volume: f32,
    pub elapsed: Duration,
    pub duration: Option<Duration>,

    pub should_quit: bool,
    pub status_message: String,
    pub locale: Locale,
    pub lyrics: Vec<LyricLine>,
    pub current_lyric_index: Option<usize>,
    pub(crate) lyrics_idle_ticks: u32,
    pub(crate) fade: Option<FadeState>,

    pub track_meta: Option<AudioMeta>,

    pub focus: Panel,
    pub library_visible: bool,

    pub library_state: ListState,
    pub playlist_state: ListState,
    pub lyrics_state: ListState,

    pub library_inner: Rect,
    pub playlist_inner: Rect,
    pub lyrics_inner: Rect,
    pub progress_area: Rect,
    pub volume_area: Rect,

    pub(crate) player: PlayerEngine,
    pub(crate) play_start: Option<Instant>,
    pub(crate) accumulated: Duration,
}

impl App {
    pub fn new(paths: Vec<PathBuf>, locale: Locale) -> anyhow::Result<Self> {
        let player = PlayerEngine::new()?;
        let mut library = Vec::new();

        for path in &paths {
            if path.extension().map(|e| e == "xspf").unwrap_or(false) {
                match parse_xspf(path) {
                    Ok(files) => {
                        for f in files {
                            if is_audio_file(&f) { library.push(Track::from_path(f)); }
                        }
                    }
                    Err(e) => eprintln!("XSPF import error: {e}"),
                }
            } else if path.is_dir() {
                let mut scanned = scan_directory(path)?;
                library.append(&mut scanned);
            } else if path.is_file() && is_audio_file(path) {
                library.push(Track::from_path(path.clone()));
            }
        }

        library.sort_by(|a, b| {
            a.file_stem().to_lowercase().cmp(&b.file_stem().to_lowercase())
        });

        let playlist: Vec<usize> = (0..library.len()).collect();

        let mut lib_state = ListState::default();
        let mut pl_state = ListState::default();
        let lrc_state = ListState::default();
        if !library.is_empty() {
            lib_state.select(Some(0));
            pl_state.select(Some(0));
        }

        Ok(Self {
            library, playlist,
            current_index: None,
            status: PlayerStatus::Stopped,
            play_mode: PlayMode::Sequential,
            volume: 0.8,
            elapsed: Duration::ZERO,
            duration: None,
            should_quit: false,
            status_message: String::new(),
            locale,
            lyrics: Vec::new(),
            current_lyric_index: None,
            lyrics_idle_ticks: 0,
            fade: None,
            track_meta: None,
            focus: Panel::Playlist,
            library_visible: false,
            library_state: lib_state,
            playlist_state: pl_state,
            lyrics_state: lrc_state,
            library_inner: Rect::ZERO,
            playlist_inner: Rect::ZERO,
            lyrics_inner: Rect::ZERO,
            progress_area: Rect::ZERO,
            volume_area: Rect::ZERO,
            player,
            play_start: None,
            accumulated: Duration::ZERO,
        })
    }

    // ── Accessors ──

    pub fn library_state_mut(&mut self) -> &mut ListState { &mut self.library_state }
    pub fn playlist_state_mut(&mut self) -> &mut ListState { &mut self.playlist_state }
    pub fn lyrics_state_mut(&mut self) -> &mut ListState { &mut self.lyrics_state }
    pub fn library_count(&self) -> usize { self.library.len() }
    pub fn playlist_count(&self) -> usize { self.playlist.len() }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|i| self.library.get(i))
    }

    pub fn current_playlist_position(&self) -> Option<usize> {
        self.current_index.and_then(|lib_idx| self.playlist.iter().position(|&i| i == lib_idx))
    }

    pub(crate) fn visible_panels(&self) -> Vec<Panel> {
        let mut panels = vec![Panel::Playlist, Panel::Lyrics];
        if self.library_visible { panels.insert(0, Panel::Library); }
        panels
    }

    // ── Update loop ──

    pub fn update(&mut self) {
        self.update_fade();
        self.update_elapsed();
        self.update_lyric_index();

        // Auto-scroll lyrics
        if self.focus == Panel::Lyrics {
            let selection = self.lyrics_state.selected();
            if selection != self.current_lyric_index {
                self.lyrics_idle_ticks += 1;
                if self.lyrics_idle_ticks > 30 {
                    if let Some(idx) = self.current_lyric_index {
                        self.lyrics_state.select(Some(idx));
                    }
                    self.lyrics_idle_ticks = 0;
                }
            } else {
                self.lyrics_idle_ticks = 0;
            }
        } else if !self.lyrics.is_empty() {
            if let Some(idx) = self.current_lyric_index {
                self.lyrics_state.select(Some(idx));
            }
        }

        if self.status == PlayerStatus::Playing && self.player.is_empty() && self.fade.is_none() {
            self.advance_track();
        }
    }

    fn update_elapsed(&mut self) {
        if self.status == PlayerStatus::Playing {
            if let Some(start) = self.play_start {
                self.elapsed = self.accumulated + start.elapsed();
                if let Some(dur) = self.duration {
                    if self.elapsed > dur { self.elapsed = dur; }
                }
            }
        }
    }

    fn update_lyric_index(&mut self) {
        if self.lyrics.is_empty() { return; }
        let idx = self.lyrics.partition_point(|l| l.timestamp <= self.elapsed);
        self.current_lyric_index = if idx > 0 { Some(idx - 1) } else { None };
    }

    pub(crate) fn reset_timing(&mut self) {
        self.play_start = None;
        self.accumulated = Duration::ZERO;
        self.elapsed = Duration::ZERO;
        self.duration = None;
    }
}

// ── Shared list helpers ──

pub(crate) fn move_list(state: &mut ListState, len: usize, delta: i32) {
    if len == 0 { return; }
    let cur = state.selected().unwrap_or(0);
    let new = if delta < 0 {
        cur.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        (cur + delta as usize).min(len.saturating_sub(1))
    };
    state.select(Some(new));
}

pub(crate) fn select_at(state: &mut ListState, area: &Rect, mouse_y: u16, len: usize) {
    if len == 0 { return; }
    let offset = state.offset();
    let idx = offset.saturating_add((mouse_y as usize).saturating_sub(area.y as usize));
    if idx < len { state.select(Some(idx)); }
}
