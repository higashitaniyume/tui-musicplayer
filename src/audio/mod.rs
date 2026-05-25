use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait};
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
    pub fn new(host_name: Option<&str>) -> anyhow::Result<Self> {
        let (stream, handle) = Self::create_output(host_name)?;
        let sink = Sink::try_new(&handle)
            .context("Failed to create audio sink")?;
        Ok(Self { _stream: stream, _handle: handle, sink })
    }

    fn find_device_by_name(name: &str) -> Option<cpal::Device> {
        for host_id in cpal::available_hosts() {
            if let Ok(host) = cpal::host_from_id(host_id) {
                if let Ok(devices) = host.output_devices() {
                    for device in devices {
                        if device.name().map(|n| n == name).unwrap_or(false) {
                            return Some(device);
                        }
                    }
                }
            }
        }
        None
    }

    fn create_output(device_name: Option<&str>) -> anyhow::Result<(OutputStream, OutputStreamHandle)> {
        let name = match device_name {
            Some(n) if !n.is_empty() => n,
            _ => {
                return OutputStream::try_default()
                    .context("Failed to create default audio output stream (no audio device?)")
                    .map_err(Into::into);
            }
        };
        let device = Self::find_device_by_name(name)
            .with_context(|| format!("Audio device not found: {name}"))?;
        OutputStream::try_from_device(&device)
            .with_context(|| format!("Failed to create output stream for: {name}"))
            .map_err(Into::into)
    }

    /// Restart the audio engine with a new device
    pub(crate) fn restart_with_device(&mut self, device_name: Option<&str>) -> anyhow::Result<()> {
        let (stream, handle) = Self::create_output(device_name)?;
        let sink = Sink::try_new(&handle)
            .context("Failed to create audio sink")?;
        self._stream = stream;
        self._handle = handle;
        self.sink = sink;
        Ok(())
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

/// List available audio host names
pub fn available_hosts() -> Vec<String> {
    cpal::available_hosts().iter().map(|id| id.name().to_string()).collect()
}

/// Enumerate all output devices across all hosts.
/// Returns (host_name, device_name, is_current) tuples.
pub fn available_devices(current_device: Option<&str>) -> Vec<(String, String, bool)> {
    let mut devices = Vec::new();
    let hosts = cpal::available_hosts();
    info!("Found {} audio host(s)", hosts.len());

    // Also try the default host explicitly if not in the list
    let default_id = cpal::default_host().id();
    let mut all_host_ids: Vec<cpal::HostId> = hosts.clone();
    if !all_host_ids.iter().any(|h| h.name() == default_id.name()) {
        all_host_ids.push(default_id);
    }

    for host_id in &all_host_ids {
        info!("  Host: {}", host_id.name());
        match cpal::host_from_id(*host_id) {
            Ok(host) => {
                // Try enumerating output devices
                match host.output_devices() {
                    Ok(outputs) => {
                        let mut count = 0;
                        for device in outputs {
                            if let Ok(name) = device.name() {
                                info!("    Device: {name}");
                                let is_current = current_device == Some(&name);
                                devices.push((host_id.name().to_string(), name, is_current));
                                count += 1;
                            }
                        }
                        // If device iterator was empty, try the default output device
                        if count == 0 {
                            if let Some(def_dev) = host.default_output_device() {
                                if let Ok(name) = def_dev.name() {
                                    info!("    Default device: {name}");
                                    let is_current = current_device == Some(&name);
                                    devices.push((host_id.name().to_string(), name, is_current));
                                }
                            }
                        }
                        info!("    {} usable device(s)", count);
                    }
                    Err(e) => info!("    output_devices error: {e}"),
                }
            }
            Err(e) => info!("  host_from_id error: {e}"),
        }
    }
    info!("Total: {} audio devices enumerated", devices.len());
    devices
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerStatus {
    Playing,
    Paused,
    Stopped,
}
