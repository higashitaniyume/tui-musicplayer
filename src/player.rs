use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

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

// ── Directory scanning ──

pub fn scan_directory(dir: &Path) -> anyhow::Result<Vec<Track>> {
    let mut tracks = Vec::new();
    for entry in walkdir::WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    tracks.push(Track::from_path(path.to_path_buf()));
                }
            }
        }
    }
    tracks.sort_by(|a, b| a.file_stem().to_lowercase().cmp(&b.file_stem().to_lowercase()));
    Ok(tracks)
}

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

// ── XSPF Playlist import ──

pub fn parse_xspf(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read XSPF file: {}", path.display()))?;
    let doc = roxmltree::Document::parse(&content)
        .with_context(|| format!("Invalid XML in: {}", path.display()))?;

    let mut files = Vec::new();
    for track_node in doc.descendants().filter(|n| n.has_tag_name("track")) {
        if let Some(loc) = track_node
            .descendants()
            .find(|n| n.has_tag_name("location"))
            .and_then(|n| n.text())
        {
            let loc = loc.trim();
            // Strip file:// prefix
            let path_str = loc
                .strip_prefix("file://")
                .or_else(|| loc.strip_prefix("file:"))
                .unwrap_or(loc);
            // URL-decode (basic: just %20 → space)
            let path_str = path_str.replace("%20", " ");
            let p = PathBuf::from(path_str);
            if p.exists() {
                files.push(p);
            }
        }
    }

    Ok(files)
}

// ── LRC Lyrics ──

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

pub fn parse_lrc(path: &Path) -> Option<Vec<LyricLine>> {
    let lrc_path = path.with_extension("lrc");
    let content = std::fs::read_to_string(&lrc_path).ok()?;
    let mut lines: Vec<LyricLine> = Vec::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut timestamps = Vec::new();
        let mut search_start = 0;
        while let Some(start) = line[search_start..].find('[') {
            let abs_start = search_start + start;
            if let Some(end) = line[abs_start..].find(']') {
                let abs_end = abs_start + end;
                let tag = &line[abs_start + 1..abs_end];
                if let Some(ts) = parse_timestamp(tag) {
                    timestamps.push(ts);
                    search_start = abs_end + 1;
                } else {
                    search_start = abs_end + 1;
                }
            } else {
                break;
            }
        }
        let text_start = line.rfind(']').map(|i| i + 1).unwrap_or(0);
        let text = line[text_start..].trim().to_string();
        if text.is_empty() {
            continue;
        }
        for ts in timestamps {
            if lines.iter().any(|l| l.timestamp == ts) {
                continue;
            }
            lines.push(LyricLine { timestamp: ts, text: text.clone() });
        }
    }

    lines.sort_by_key(|l| l.timestamp);
    if lines.is_empty() { None } else { Some(lines) }
}

fn parse_timestamp(s: &str) -> Option<Duration> {
    let colon = s.find(':')?;
    let minutes: u64 = s[..colon].parse().ok()?;
    let rest = &s[colon + 1..];
    let dot = rest.find('.')?;
    let seconds: u64 = rest[..dot].parse().ok()?;
    let frac_str = &rest[dot + 1..];
    let frac_len = frac_str.len().min(3);
    let frac: u64 = frac_str[..frac_len].parse().ok()?;
    let millis = frac * 10u64.pow(3 - frac_len as u32);
    Some(Duration::new(minutes * 60 + seconds, millis as u32 * 1_000_000))
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
        let source = source.skip_duration(skip);
        self.sink.append(source);
        Ok(meta)
    }

    pub fn play(&self) { self.sink.play(); }
    pub fn pause(&self) { self.sink.pause(); }
    pub fn stop(&self) { self.sink.stop(); }
    pub fn is_empty(&self) -> bool { self.sink.empty() }
    pub fn set_volume(&self, vol: f32) { self.sink.set_volume(vol.clamp(0.0, 1.0)); }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerStatus {
    Playing,
    Paused,
    Stopped,
}
