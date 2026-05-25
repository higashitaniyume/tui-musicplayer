use std::path::{Path, PathBuf};

use anyhow::Context;
use log::{debug, info};

use super::{Track, AUDIO_EXTENSIONS};

pub fn scan_directory(dir: &Path) -> anyhow::Result<Vec<Track>> {
    debug!("Scanning directory: {}", dir.display());
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
    debug!("Scan complete {}: {} audio files", dir.display(), tracks.len());
    Ok(tracks)
}

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn parse_xspf(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    debug!("Parsing XSPF: {}", path.display());
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read XSPF file: {}", path.display()))?;
    let doc = roxmltree::Document::parse(&content)
        .with_context(|| format!("Invalid XML in: {}", path.display()))?;

    let mut files = Vec::new();
    let mut missing = 0u32;
    for track_node in doc.descendants().filter(|n| n.has_tag_name("track")) {
        if let Some(loc) = track_node
            .descendants()
            .find(|n| n.has_tag_name("location"))
            .and_then(|n| n.text())
        {
            let loc = loc.trim();
            let mut path_str = loc
                .strip_prefix("file:///")
                .or_else(|| loc.strip_prefix("file://"))
                .or_else(|| loc.strip_prefix("file:"))
                .unwrap_or(loc)
                .to_string();
            if cfg!(windows) && path_str.len() > 3 {
                let bytes = path_str.as_bytes();
                if bytes[0] == b'/' && bytes[2] == b':' {
                    path_str.remove(0);
                }
            }
            let path_str = path_str.replace("%20", " ");
            let p = PathBuf::from(&path_str);
            if p.exists() {
                files.push(p);
            } else {
                missing += 1;
                debug!("  XSPF track not found: {}", p.display());
            }
        }
    }

    if missing > 0 {
        info!("XSPF '{}': {} tracks found, {} missing", path.display(), files.len(), missing);
    }
    Ok(files)
}
