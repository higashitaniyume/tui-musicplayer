use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use log::{debug, info, warn};
use ratatui::{layout::Rect, widgets::ListState};

use crate::audio::{
    is_audio_file, parse_xspf, scan_directory, AudioMeta, LyricLine, PlayMode,
    PlayerEngine, PlayerStatus, Track,
};
use crate::config::Config;
use crate::locale::Locale;

pub mod input;
pub mod playback;

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

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsSection {
    Folders,
    Playlists,
}

#[derive(Clone, PartialEq)]
pub enum PathPurpose {
    AddMusicFolder,
    ImportXspf,
}

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    EnteringPath {
        buffer: String,
        cursor: usize,
        purpose: PathPurpose,
    },
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
    pub lyric_line_map: Vec<usize>,

    pub library_inner: Rect,
    pub playlist_inner: Rect,
    pub lyrics_inner: Rect,
    pub progress_area: Rect,
    pub volume_area: Rect,

    pub(crate) player: PlayerEngine,
    pub(crate) play_start: Option<Instant>,
    pub(crate) accumulated: Duration,

    // ── Settings ──
    pub config: Config,
    pub show_settings: bool,
    pub settings_section: SettingsSection,
    pub settings_folder_state: ListState,
    pub settings_xspf_state: ListState,
    pub settings_folder_inner: Rect,
    pub settings_xspf_inner: Rect,

    // ── Text input ──
    pub input_mode: InputMode,

    // ── Named playlists ──
    pub saved_playlists: Vec<(String, Vec<usize>)>,
    pub active_playlist_idx: Option<usize>,
}

impl App {
    pub fn new(paths: Vec<PathBuf>, mut config: Config, locale: Locale) -> anyhow::Result<Self> {
        let player = PlayerEngine::new()?;
        let mut library = Vec::new();

        let search_paths: Vec<PathBuf> = if paths.is_empty() {
            let mut p = Vec::new();
            for folder in &config.music_folders {
                p.push(folder.clone());
            }
            for xspf in &config.xspf_playlists {
                p.push(xspf.clone());
            }
            if p.is_empty() {
                vec![system_music_dir()]
            } else {
                p
            }
        } else {
            paths
        };

        let mut seen_paths: HashSet<PathBuf> = HashSet::new();
        let mut xspf_imports: Vec<(String, Vec<PathBuf>)> = Vec::new();

        for path in &search_paths {
            if path.extension().map(|e| e == "xspf").unwrap_or(false) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Playlist")
                    .to_string();
                let mut ordered_paths = Vec::new();
                match parse_xspf(path) {
                    Ok(files) => {
                        for f in files {
                            if is_audio_file(&f) {
                                ordered_paths.push(f.clone());
                                if !seen_paths.contains(&f) {
                                    seen_paths.insert(f.clone());
                                    library.push(Track::from_path(f));
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("XSPF import error: {e}"),
                }
                if !ordered_paths.is_empty() {
                    xspf_imports.push((name, ordered_paths));
                }
            } else if path.is_dir() {
                let scanned = scan_directory(path)?;
                for track in scanned {
                    if !seen_paths.contains(&track.path) {
                        seen_paths.insert(track.path.clone());
                        library.push(track);
                    }
                }
            } else if path.is_file() && is_audio_file(path) {
                if !seen_paths.contains(path) {
                    seen_paths.insert(path.clone());
                    library.push(Track::from_path(path.clone()));
                }
            }
        }

        library.sort_by(|a, b| {
            a.file_stem().to_lowercase().cmp(&b.file_stem().to_lowercase())
        });

        let path_to_idx: HashMap<&PathBuf, usize> = library.iter().enumerate()
            .map(|(i, t)| (&t.path, i))
            .collect();

        let playlist: Vec<usize> = (0..library.len()).collect();

        let mut saved_playlists: Vec<(String, Vec<usize>)> = Vec::new();
        for (name, ordered_paths) in &xspf_imports {
            let indices: Vec<usize> = ordered_paths.iter()
                .filter_map(|p| path_to_idx.get(p).copied())
                .collect();
            info!("Named playlist '{}': {} of {} XSPF tracks matched in library",
                name, indices.len(), ordered_paths.len());
            if !indices.is_empty() {
                saved_playlists.push((name.clone(), indices));
            }
        }

        let mut lib_state = ListState::default();
        let mut pl_state = ListState::default();
        let lrc_state = ListState::default();
        if !library.is_empty() {
            lib_state.select(Some(0));
            pl_state.select(Some(0));
        }

        if config.music_folders.is_empty() && !search_paths.is_empty() {
            for p in &search_paths {
                if p.is_dir() {
                    config.music_folders.push(p.clone());
                }
            }
        }

        let mut folder_state = ListState::default();
        if !config.music_folders.is_empty() {
            folder_state.select(Some(0));
        }
        let mut xspf_state = ListState::default();
        if !config.xspf_playlists.is_empty() {
            xspf_state.select(Some(0));
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
            lyric_line_map: Vec::new(),
            library_inner: Rect::ZERO,
            playlist_inner: Rect::ZERO,
            lyrics_inner: Rect::ZERO,
            progress_area: Rect::ZERO,
            volume_area: Rect::ZERO,
            player,
            play_start: None,
            accumulated: Duration::ZERO,
            config,
            show_settings: false,
            settings_section: SettingsSection::Folders,
            settings_folder_state: folder_state,
            settings_xspf_state: xspf_state,
            settings_folder_inner: Rect::ZERO,
            settings_xspf_inner: Rect::ZERO,
            input_mode: InputMode::Normal,
            saved_playlists,
            active_playlist_idx: None,
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

        if self.focus == Panel::Lyrics {
            let current_logical = self.lyrics_state.selected()
                .and_then(|phys| self.lyric_line_map.get(phys).copied());
            if current_logical != self.current_lyric_index {
                self.lyrics_idle_ticks += 1;
                if self.lyrics_idle_ticks > 30 {
                    if let Some(idx) = self.current_lyric_index {
                        if let Some(phys) = self.lyric_line_map.iter().position(|&log| log == idx) {
                            self.lyrics_state.select(Some(phys));
                        }
                    }
                    self.lyrics_idle_ticks = 0;
                }
            } else {
                self.lyrics_idle_ticks = 0;
            }
        } else if !self.lyrics.is_empty() {
            if let Some(idx) = self.current_lyric_index {
                if let Some(phys) = self.lyric_line_map.iter().position(|&log| log == idx) {
                    self.lyrics_state.select(Some(phys));
                }
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

    // ── Settings ──

    pub fn toggle_language(&mut self) {
        use crate::locale::Lang;
        let new_lang = match self.locale.lang() {
            Lang::En => Lang::ZhCn,
            Lang::ZhCn => Lang::En,
        };
        self.locale = crate::locale::Locale::from_lang(new_lang);
        self.config.language = Some(match new_lang {
            Lang::En => "en",
            Lang::ZhCn => "zh",
        }.to_string());
        let _ = self.config.save();
        info!("Language switched to: {:?}", new_lang);
        self.status_message = match new_lang {
            Lang::En => "Language: English".into(),
            Lang::ZhCn => "语言: 中文".into(),
        };
    }

    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.status_message = String::new();
        }
    }

    pub fn add_music_folder(&mut self, path: PathBuf) {
        let path = canonicalize_path(&path);
        info!("Adding music folder: {}", path.display());
        if !path.is_dir() {
            warn!("Path is not a directory: {}", path.display());
            self.status_message = self.locale.msg_path_not_found().to_string();
            return;
        }
        if self.config.music_folders.iter().any(|f| f == &path) {
            debug!("Folder already in config: {}", path.display());
            self.status_message = self.locale.msg_folder_added().to_string();
            return;
        }
        self.config.music_folders.push(path.clone());
        match scan_directory(&path) {
            Ok(scanned) => {
                info!("Scanned {}: {} tracks found", path.display(), scanned.len());
                self.merge_tracks(scanned);
            }
            Err(e) => warn!("Failed to scan {}: {e}", path.display()),
        }
        if self.config.music_folders.len() == 1 {
            self.settings_folder_state.select(Some(0));
        }
        if let Err(e) = self.config.save() {
            warn!("Failed to save config: {e}");
        }
        self.status_message = self.locale.msg_folder_added().to_string();
    }

    pub fn import_xspf_file(&mut self, path: PathBuf) {
        let path = canonicalize_path(&path);
        info!("Importing XSPF: {}", path.display());
        if path.extension().map(|e| e != "xspf").unwrap_or(true) {
            warn!("Not an .xspf file: {}", path.display());
            self.status_message = self.locale.msg_not_xspf().to_string();
            return;
        }
        if !path.exists() {
            warn!("XSPF file not found: {}", path.display());
            self.status_message = self.locale.msg_path_not_found().to_string();
            return;
        }
        if self.config.xspf_playlists.iter().any(|p| p == &path) {
            debug!("XSPF already imported: {}", path.display());
            self.status_message = self.locale.msg_xspf_imported().to_string();
            return;
        }
        self.config.xspf_playlists.push(path.clone());

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Playlist")
            .to_string();

        match parse_xspf(&path) {
            Ok(files) => {
                info!("Parsed XSPF '{}': {} file references", name, files.len());
                let ordered_paths: Vec<PathBuf> = files
                    .into_iter()
                    .filter(|f| is_audio_file(f))
                    .collect();
                let tracks: Vec<Track> = ordered_paths.iter().map(|f| Track::from_path(f.clone())).collect();
                self.merge_tracks(tracks);
                let path_to_idx: HashMap<&PathBuf, usize> = self.library.iter().enumerate()
                    .map(|(i, t)| (&t.path, i))
                    .collect();
                let indices: Vec<usize> = ordered_paths.iter()
                    .filter_map(|p| path_to_idx.get(p).copied())
                    .collect();
                info!("Created named playlist '{}' with {} tracks", name, indices.len());
                self.saved_playlists.push((name.clone(), indices));
                info!("XSPF import complete: {} (playlist: {})", path.display(), name);
            }
            Err(e) => {
                warn!("XSPF parse error for {}: {e}", path.display());
                self.status_message = format!("XSPF error: {e}");
                return;
            }
        }
        if self.config.xspf_playlists.len() == 1 {
            self.settings_xspf_state.select(Some(0));
        }
        if let Err(e) = self.config.save() {
            warn!("Failed to save config: {e}");
        }
        self.status_message = self.locale.msg_xspf_imported().to_string();
    }

    pub fn cycle_playlist(&mut self) {
        debug!("cycle_playlist: {} saved playlists, active_idx={:?}",
            self.saved_playlists.len(), self.active_playlist_idx);
        if self.saved_playlists.is_empty() {
            debug!("cycle_playlist: no saved playlists, showing hint");
            self.status_message = match self.locale.lang() {
                crate::locale::Lang::En => "No saved playlists — import .xspf in Settings (F2)",
                crate::locale::Lang::ZhCn => "无已保存的播放列表 — 在设置中导入 .xspf (F2)",
            }.to_string();
            return;
        }
        self.stop();
        let next = match self.active_playlist_idx {
            None => 0,
            Some(i) if i + 1 >= self.saved_playlists.len() => {
                info!("cycle_playlist: switching back to All Tracks");
                self.active_playlist_idx = None;
                self.playlist = (0..self.library.len()).collect();
                self.playlist_state.select(Some(0));
                self.status_message = self.locale.playlist_switched_all().to_string();
                return;
            }
            Some(i) => i + 1,
        };
        self.active_playlist_idx = Some(next);
        self.playlist = self.saved_playlists[next].1.clone();
        info!("cycle_playlist: switched to '{}' ({} tracks)",
            self.saved_playlists[next].0, self.playlist.len());
        if !self.playlist.is_empty() {
            self.playlist_state.select(Some(0));
        } else {
            self.playlist_state.select(None);
        }
        self.status_message = self.locale.playlist_switched(&self.saved_playlists[next].0);
    }

    pub fn remove_settings_item(&mut self) {
        match self.settings_section {
            SettingsSection::Folders => {
                let idx = match self.settings_folder_state.selected() {
                    Some(i) if i < self.config.music_folders.len() => i,
                    _ => return,
                };
                let removed = &self.config.music_folders[idx];
                info!("Removing music folder from config: {}", removed.display());
                self.config.music_folders.remove(idx);
                if self.config.music_folders.is_empty() {
                    self.settings_folder_state.select(None);
                } else if idx >= self.config.music_folders.len() {
                    self.settings_folder_state.select(Some(self.config.music_folders.len() - 1));
                }
                if let Err(e) = self.config.save() {
                    warn!("Failed to save config: {e}");
                }
                self.status_message = self.locale.msg_folder_removed().to_string();
            }
            SettingsSection::Playlists => {
                let idx = match self.settings_xspf_state.selected() {
                    Some(i) if i < self.config.xspf_playlists.len() => i,
                    _ => return,
                };
                let removed = &self.config.xspf_playlists[idx];
                info!("Removing XSPF playlist from config: {}", removed.display());
                self.config.xspf_playlists.remove(idx);
                if self.config.xspf_playlists.is_empty() {
                    self.settings_xspf_state.select(None);
                } else if idx >= self.config.xspf_playlists.len() {
                    self.settings_xspf_state.select(Some(self.config.xspf_playlists.len() - 1));
                }
                if let Err(e) = self.config.save() {
                    warn!("Failed to save config: {e}");
                }
                self.status_message = self.locale.msg_xspf_removed().to_string();
            }
        }
    }

    pub fn rescan_all(&mut self) {
        info!("Rescanning all configured sources...");
        self.stop();
        self.library.clear();
        self.playlist.clear();
        self.current_index = None;
        self.lyrics.clear();
        self.current_lyric_index = None;
        self.track_meta = None;
        self.reset_timing();

        let mut seen: HashSet<PathBuf> = HashSet::new();

        for folder in &self.config.music_folders {
            match scan_directory(folder) {
                Ok(scanned) => {
                    let count = scanned.len();
                    for track in scanned {
                        if seen.insert(track.path.clone()) {
                            self.library.push(track);
                        }
                    }
                    info!("  Scanned {}: {} tracks", folder.display(), count);
                }
                Err(e) => warn!("  Failed to scan {}: {e}", folder.display()),
            }
        }

        for xspf in &self.config.xspf_playlists {
            match parse_xspf(xspf) {
                Ok(files) => {
                    let count = files.len();
                    for f in files {
                        if is_audio_file(&f) && seen.insert(f.clone()) {
                            self.library.push(Track::from_path(f));
                        }
                    }
                    info!("  Parsed {}: {} files", xspf.display(), count);
                }
                Err(e) => warn!("  Failed to parse {}: {e}", xspf.display()),
            }
        }

        self.library.sort_by(|a, b| {
            a.file_stem().to_lowercase().cmp(&b.file_stem().to_lowercase())
        });

        self.playlist = (0..self.library.len()).collect();

        if !self.library.is_empty() {
            self.library_state.select(Some(0));
            self.playlist_state.select(Some(0));
        } else {
            self.library_state.select(None);
            self.playlist_state.select(None);
        }

        info!("Rescan complete: {} unique tracks in library", self.library.len());
        self.status_message = self.locale.msg_rescanned(self.library.len());
    }

    fn merge_tracks(&mut self, new_tracks: Vec<Track>) {
        let existing: HashSet<PathBuf> = self.library.iter().map(|t| t.path.clone()).collect();
        let added_count = new_tracks.iter().filter(|t| !existing.contains(&t.path)).count();
        let skipped = new_tracks.len() - added_count;
        for track in new_tracks {
            if !existing.contains(&track.path) {
                self.library.push(track);
            }
        }
        if added_count > 0 {
            debug!("Merged {added_count} new tracks (skipped {skipped} duplicates)");
            self.library.sort_by(|a, b| {
                a.file_stem().to_lowercase().cmp(&b.file_stem().to_lowercase())
            });
            self.playlist = (0..self.library.len()).collect();
            self.playlist_state.select(self.current_playlist_position());
        }
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

fn system_music_dir() -> PathBuf {
    if let Some(dir) = dirs::audio_dir() {
        if dir.is_dir() {
            return dir;
        }
    }
    if let Some(home) = dirs::home_dir() {
        let music = home.join("Music");
        if music.is_dir() {
            return music;
        }
    }
    PathBuf::from(".")
}

fn canonicalize_path(path: &PathBuf) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(_) => {
            if path.is_relative() {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|_| path.clone())
            } else {
                path.clone()
            }
        }
    }
}
