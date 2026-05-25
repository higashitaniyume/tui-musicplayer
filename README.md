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

| Key          | Action                                                       |
| ------------ | ------------------------------------------------------------ |
| `Space`      | Play / Pause                                                 |
| `n` / `p`    | Next / Previous track                                        |
| `s`          | Stop                                                         |
| `←` / `→`    | Seek -5s / +5s                                               |
| `+` / `-`    | Volume up / down                                             |
| `m`          | Cycle play mode (Seq → Shuffle → Repeat1 → RepeatAll)        |
| `Tab`        | Cycle focus (Playlist → Lyrics → Library)                     |
| `l`          | Toggle Library panel                                         |
| `w`          | Switch named playlist (XSPF imports)                         |
| `Enter`      | Play selected / Add to playlist / Seek lyric                 |
| `a`          | (Library) Add track to playlist                              |
| `d`          | (Playlist) Remove track from playlist                        |
| `F2`         | Open Settings                                                |
| `q` / `Esc`  | Quit                                                         |

### Settings Mode

| Key       | Action                                                    |
| --------- | --------------------------------------------------------- |
| `Tab`     | Switch section (Folders / Playlists)                       |
| `a`       | Add music folder (type path → Enter)                       |
| `i`       | Import .xspf file (type path → Enter)                      |
| `d`       | Remove selected folder/playlist                            |
| `l`       | Toggle language (English / 中文)                            |
| `Enter`   | Rescan all configured sources                              |
| `Esc`/`F2`| Back                                                      |

## File Locations

| File   | Path                                                              |
| ------ | ----------------------------------------------------------------- |
| Config | `%AppData%/tui-musicplayer/config.json`                           |
| Log    | `%AppData%/tui-musicplayer/logs/player_2026-05-25_08-30-00.log`   |

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

## Known Issues

_Audited 2026-05-25 — 14 source files, ~3500 lines of Rust._

### Functional bugs

- **High — Shuffle mode is not implemented.** `PlayMode::Shuffle` falls through to the same branch as `Sequential`, so tracks always play in insertion order. The `rand` crate is declared in `Cargo.toml` but never used. ([`playback.rs:163`](src/app/playback.rs#L163))
- **Medium — LRC timestamps without milliseconds are silently dropped.** `parse_timestamp` requires a decimal point (`.`) in the bracket tag, but the LRC spec allows `[mm:ss]` without fractional seconds. LRC files using that format will never display lyrics. ([`lyrics.rs:54-65`](src/audio/lyrics.rs#L54-L65))
- **Medium — `truncate` / `wrap_text` use char count, not terminal display width.** CJK characters are double-width in terminals but are counted as 1, causing text overflow in library, playlist, and lyrics panels. ([`ui/mod.rs:33`](src/ui/mod.rs#L33), [`panels.rs:253`](src/ui/panels.rs#L253))
- **Low — Input buffer byte-index safety.** `String::remove()` is called with a char-position cursor — panics possible with multi-byte UTF-8 in path entry. ([`input.rs:53-63`](src/app/input.rs#L53-L63))
- **Low — Windows XSPF path normalization is fragile.** Assumes specific byte layout (`/C:/…`) without validating the drive letter. ([`scan.rs:56-60`](src/audio/scan.rs#L56-L60))
- **Low — `lyrics_state.select(Some(0))` called unconditionally** even when no LRC file was loaded. ([`playback.rs:89`](src/app/playback.rs#L89))
- **Low — Audio metadata not populated until first play.** Bitrate, sample rate, and duration show `?` for unplayed tracks. ([`mod.rs:55-78`](src/audio/mod.rs#L55-L78))

### Security notes

- **Medium — XSPF URL decoding only handles `%20`.** Paths with other percent-encoded characters (`%23`, `%26`, etc.) fail to resolve. Not exploitable (paths only go to `rodio::Decoder`), but breaks library loading for files with special characters. ([`scan.rs:62`](src/audio/scan.rs#L62))
- **Low — Config save errors silently discarded** in 7 locations. The user may believe settings were persisted when I/O actually failed. ([`mod.rs:390-604`](src/app/mod.rs))
- **Low — Log file `File::create().unwrap()` can panic** on startup if the logs directory is unwritable. ([`main.rs:104`](src/main.rs#L104))
- **Low — No base-directory confinement for XSPF paths.** A crafted XSPF could reference arbitrary filesystem paths (mitigated by extension filter and `rodio` decode failure). ([`scan.rs:43-71`](src/audio/scan.rs#L43-L71))

### Dependency hygiene

- `rand = "0.8"` declared but never imported (dead weight — tied to the missing shuffle implementation).
- `roxmltree` is safe against XXE (no DTD/entity resolution).
- The app makes no network requests and executes no shell commands — minimal attack surface.

## License

MIT
