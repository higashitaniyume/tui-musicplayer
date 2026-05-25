use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, LevelFilter};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use simplelog::{CombinedLogger, ConfigBuilder, WriteLogger};

mod app;
mod app_input;
mod app_playback;
mod config;
mod locale;
mod player;
mod ui;
mod ui_panels;
mod ui_settings;

use config::Config;
use locale::{detect_locale, Lang, Locale};

#[derive(ValueEnum, Clone, Copy)]
enum LangArg {
    En,
    Zh,
}

impl From<LangArg> for Lang {
    fn from(arg: LangArg) -> Self {
        match arg {
            LangArg::En => Lang::En,
            LangArg::Zh => Lang::ZhCn,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "tui-musicplayer",
    version,
    about = "A cross-platform terminal music player"
)]
struct Args {
    /// Paths to audio files or directories to scan (default: configured folders or system music folder)
    paths: Vec<PathBuf>,

    /// Display language
    #[arg(long, value_enum)]
    lang: Option<LangArg>,
}

fn setup_logging() {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tui-musicplayer");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("player.log");

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();

    let _ = CombinedLogger::init(vec![
        WriteLogger::new(LevelFilter::Debug, config, std::fs::File::create(&log_path).unwrap()),
    ]);
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let args = Args::parse();

    let config = Config::load();

    // Language priority: CLI --lang > config file > auto-detect (default Chinese)
    let locale = args
        .lang
        .map(|l| Locale::from_lang(l.into()))
        .or_else(|| {
            config.language.as_ref().and_then(|s| match s.as_str() {
                "en" => Some(Locale::en()),
                "zh" => Some(Locale::zh_cn()),
                _ => None,
            })
        })
        .unwrap_or_else(detect_locale);

    info!("tui-musicplayer v{} starting", env!("CARGO_PKG_VERSION"));
    info!("Locale: {:?}", locale.lang());
    info!("CLI paths: {:?}", args.paths);
    info!("Config loaded: {} folders, {} xspf playlists",
        config.music_folders.len(), config.xspf_playlists.len());
    for folder in &config.music_folders {
        info!("  music folder: {}", folder.display());
    }
    for xspf in &config.xspf_playlists {
        info!("  xspf playlist: {}", xspf.display());
    }

    let search_paths: Vec<PathBuf> = if args.paths.is_empty() {
        let mut p = Vec::new();
        for folder in &config.music_folders {
            p.push(folder.clone());
        }
        for xspf in &config.xspf_playlists {
            p.push(xspf.clone());
        }
        if p.is_empty() {
            let default = system_music_dir();
            info!("No paths configured, using system music dir: {}", default.display());
            vec![default]
        } else {
            p
        }
    } else {
        args.paths
    };

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &search_paths, config, locale);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    match &result {
        Ok(()) => info!("tui-musicplayer exiting normally"),
        Err(e) => error!("tui-musicplayer exiting with error: {e:?}"),
    }

    result
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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    paths: &[PathBuf],
    config: Config,
    locale: Locale,
) -> anyhow::Result<()> {
    let mut app = app::App::new(paths.to_vec(), config, locale)?;
    info!("App initialized: {} tracks in library, {} in playlist, {} saved playlists",
        app.library_count(), app.playlist_count(), app.saved_playlists.len());
    for (i, (name, indices)) in app.saved_playlists.iter().enumerate() {
        info!("  saved playlist [{i}]: '{name}' — {len} tracks", len = indices.len());
    }
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                Event::Resize(cols, rows) => {
                    info!("Terminal resize: {cols}x{rows}");
                    let _ = terminal.clear();
                }
                _ => {}
            }
        }

        app.update();

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
