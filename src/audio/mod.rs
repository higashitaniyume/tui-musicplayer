use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use log::{debug, info};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub mod lyrics;
pub mod scan;

pub use lyrics::{LyricLine, parse_lrc};
pub use scan::{is_audio_file, parse_xspf, scan_directory};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "wav", "aiff", "aif", "m4a", "aac", "wma", "opus", "ape", "wv", "mpc",
];

#[derive(Clone, Debug, PartialEq)]
pub enum PlayMode {
    Sequential,
    Shuffle,
    RepeatOne,
    RepeatAll,
}

impl PlayMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Sequential => Self::Shuffle,
            Self::Shuffle => Self::RepeatOne,
            Self::RepeatOne => Self::RepeatAll,
            Self::RepeatAll => Self::Sequential,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Option<Duration>,
    pub format: String,
    pub file_size: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bitrate: Option<u32>,
}

impl Track {
    pub fn from_path(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("?")
            .to_uppercase();
        let file_size = std::fs::metadata(&path).ok().map(|m| m.len());
        Self {
            path,
            title,
            artist: String::new(),
            album: String::new(),
            duration: None,
            format,
            file_size,
            sample_rate: None,
            channels: None,
            bitrate: None,
        }
    }

    pub fn display_title(&self) -> String {
        if self.artist.is_empty() {
            self.title.clone()
        } else {
            format!("{} - {}", self.artist, self.title)
        }
    }

    pub fn file_stem(&self) -> String {
        self.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    pub fn format_duration(&self) -> String {
        self.duration
            .map(|d| {
                let total = d.as_secs();
                format!("{:02}:{:02}", total / 60, total % 60)
            })
            .unwrap_or_else(|| "--:--".to_string())
    }

    pub fn format_size(&self) -> String {
        self.file_size.map(|s| {
            if s < 1024 { format!("{s} B") }
            else if s < 1024 * 1024 { format!("{:.1} KB", s as f64 / 1024.0) }
            else { format!("{:.1} MB", s as f64 / (1024.0 * 1024.0)) }
        }).unwrap_or_else(|| "?".to_string())
    }

    pub fn format_channels(&self) -> String {
        match self.channels {
            Some(2) => "Stereo".into(),
            Some(1) => "Mono".into(),
            Some(n) => format!("{n} ch"),
            None => "?".into(),
        }
    }
}

// ── Audio metadata from decoder ──

pub struct AudioMeta {
    pub duration: Option<Duration>,
    pub sample_rate: u32,
    pub channels: u16,
}

// ── Player Engine ──

pub struct PlayerEngine {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
}

impl PlayerEngine {
    pub fn new() -> anyhow::Result<Self> {
        let (stream, handle) = OutputStream::try_default()
            .context("Failed to create audio output stream (no audio device?)")?;
        let sink = Sink::try_new(&handle)
            .context("Failed to create audio sink")?;
        Ok(Self { _stream: stream, _handle: handle, sink })
    }

    pub fn load(&self, path: &Path, skip: Duration) -> anyhow::Result<AudioMeta> {
        debug!("Loading audio: {} (skip={}s)", path.display(), skip.as_secs_f64());
        let file = File::open(path)
            .with_context(|| format!("Cannot open file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)
            .with_context(|| format!("Unsupported audio format: {}", path.display()))?;
        let meta = AudioMeta {
            duration: source.total_duration(),
            sample_rate: source.sample_rate(),
            channels: source.channels(),
        };
        info!("Loaded: {} ({}, {:?}, {}Hz, {}ch)",
            path.display(),
            meta.duration.map(|d| format!("{:02}:{:02}", d.as_secs()/60, d.as_secs()%60)).unwrap_or_else(|| "?".into()),
            path.extension().and_then(|e| e.to_str()).unwrap_or("?").to_uppercase(),
            meta.sample_rate,
            meta.channels,
        );
        let source = source.skip_duration(skip);
        self.sink.append(source);
        Ok(meta)
    }

    pub fn play(&self) {
        debug!("Playback: play");
        self.sink.play();
    }
    pub fn pause(&self) {
        debug!("Playback: pause");
        self.sink.pause();
    }
    pub fn stop(&self) {
        debug!("Playback: stop");
        self.sink.stop();
    }
    pub fn is_empty(&self) -> bool { self.sink.empty() }
    pub fn set_volume(&self, vol: f32) {
        debug!("Volume: {:.0}%", vol * 100.0);
        self.sink.set_volume(vol.clamp(0.0, 1.0));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerStatus {
    Playing,
    Paused,
    Stopped,
}
