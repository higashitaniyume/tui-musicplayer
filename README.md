# tui-musicplayer

A cross-platform terminal music player built with [ratatui](https://github.com/ratatui/ratatui) + [rodio](https://github.com/RustAudio/rodio).

![screenshot](https://via.placeholder.com/800x400/1a1a2e/cyan?text=tui-musicplayer)

## Features

- **Terminal UI** — clean TUI with library, playlist, lyrics, and now-playing panels
- **Audio playback** — play/pause/stop, seek, volume control, play modes (sequential, shuffle, repeat)
- **Music library** — scan folders recursively for audio files (supports 13 formats: mp3, flac, ogg, wav, aiff, m4a, aac, wma, opus, ape, wv, mpc)
- **XSPF playlists** — import `.xspf` playlist files as named playlists, switch between them with `w`
- **Lyrics** — auto-load `.lrc` files, auto-scroll with playback, click/Enter to seek
- **Fade transitions** — smooth fade-in/out on play, pause, stop, and track changes
- **Settings page** — add/remove music folders and XSPF playlists, toggle language, persistent config
- **Bilingual** — English / 中文 (default), toggle in settings
- **Mouse support** — click to select, play, seek, adjust volume; scroll to navigate lists
- **Logging** — detailed debug logs written to disk

## Installation

```bash
git clone https://github.com/user/tui-musicplayer.git
cd tui-musicplayer
cargo build --release
```

Binary will be at `target/release/tui-musicplayer` (or `target\release\tui-musicplayer.exe` on Windows).

## Usage

```bash
# Start with configured music folders
tui-musicplayer

# Add a folder to the music library (persisted to config)
tui-musicplayer --add-folder ~/Music
tui-musicplayer --add-folder "D:\My Music"

# Import an .xspf playlist (persisted to config)
tui-musicplayer --add-xspf ~/playlists/rock.xspf

# One-time play (not saved)
tui-musicplayer ~/Downloads/song.flac album/

# Combine: add folder AND play a file
tui-musicplayer --add-folder ~/Music song.flac

# Show current config
tui-musicplayer --list

# Set language
tui-musicplayer --lang zh
tui-musicplayer --lang en
```

All paths added via `--add-folder` / `--add-xspf` are canonicalized (resolved to absolute paths) before saving, so they work regardless of where you launch the app from.

## Key Bindings

### Normal Mode

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous track |
| `s` | Stop |
| `←` / `→` | Seek -5s / +5s |
| `+` / `-` | Volume up / down |
| `m` | Cycle play mode (Seq → Shuffle → Repeat1 → RepeatAll) |
| `Tab` | Cycle focus (Playlist → Lyrics → Library) |
| `l` | Toggle Library panel |
| `w` | Switch named playlist (XSPF imports) |
| `Enter` | Play selected / Add to playlist / Seek lyric |
| `a` | (Library) Add track to playlist |
| `d` | (Playlist) Remove track from playlist |
| `F2` | Open Settings |
| `q` / `Esc` | Quit |

### Settings Mode

| Key | Action |
|-----|--------|
| `Tab` | Switch section (Folders / Playlists) |
| `a` | Add music folder (type path → Enter) |
| `i` | Import .xspf file (type path → Enter) |
| `d` | Remove selected folder/playlist |
| `l` | Toggle language (English / 中文) |
| `Enter` | Rescan all configured sources |
| `Esc` / `F2` | Back |

## File Locations

| File | Path |
|------|------|
| Config | `%AppData%/tui-musicplayer/config.json` |
| Log | `%AppData%/tui-musicplayer/logs/player_2026-05-25_08-30-00.log` |

On Linux/macOS, `%AppData%` maps to `~/.local/share/` or `~/.config/`.

## Supported Audio Formats

mp3, flac, ogg, wav, aiff, aif, m4a, aac, wma, opus, ape, wv, mpc

## Dependencies

- [ratatui](https://crates.io/crates/ratatui) — Terminal UI framework
- [crossterm](https://crates.io/crates/crossterm) — Terminal backend
- [rodio](https://crates.io/crates/rodio) — Audio playback engine (with Symphonia backend)
- [clap](https://crates.io/crates/clap) — CLI argument parsing
- [walkdir](https://crates.io/crates/walkdir) — Recursive directory scanning
- [roxmltree](https://crates.io/crates/roxmltree) — XSPF XML parsing
- [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) — Config persistence
- [log](https://crates.io/crates/log) + [simplelog](https://crates.io/crates/simplelog) — Logging

## License

MIT
