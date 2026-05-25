use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, warn, LevelFilter};
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
    about = "A cross-platform terminal music player",
    long_about = "A terminal-based music player with library management, XSPF playlist support, lyrics display, and bilingual interface.\n\n\
                  Start without arguments to load your configured music folders.\n\
                  Use --add-folder or --add-xspf to permanently add sources to your config."
)]
struct Args {
    /// Audio files or directories to play (one-time, not saved to config)
    paths: Vec<PathBuf>,

    /// Add a music folder to the library (persisted to config)
    #[arg(long = "add-folder", value_name = "DIR", verbatim_doc_comment)]
    add_folder: Vec<PathBuf>,

    /// Import an .xspf playlist file (persisted to config)
    #[arg(long = "add-xspf", value_name = "XSPF", verbatim_doc_comment)]
    add_xspf: Vec<PathBuf>,

    /// Display language (en / zh)
    #[arg(long, value_enum)]
    lang: Option<LangArg>,

    /// Show current config paths and exit
    #[arg(long)]
    list: bool,
}

fn canonicalize_path(path: &PathBuf) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(_) => {
            // If the path doesn't exist yet, try to make it absolute
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

fn setup_logging() {
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tui-musicplayer");
    let log_dir = base_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    let ts = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::parse("[year]-[month]-[day]_[hour]-[minute]-[second]").unwrap())
        .unwrap_or_else(|_| "unknown".to_string());
    let log_path = log_dir.join(format!("player_{ts}.log"));

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();

    let _ = CombinedLogger::init(vec![
        WriteLogger::new(LevelFilter::Debug, config, std::fs::File::create(&log_path).unwrap()),
    ]);
    eprintln!("Log: {}", log_path.display());
}

fn print_config(config: &Config) {
    println!("tui-musicplayer config");
    println!("  config path: {}", config_path_display());
    println!("  language: {}", config.language.as_deref().unwrap_or("auto"));
    println!("  music folders ({}):", config.music_folders.len());
    for (i, folder) in config.music_folders.iter().enumerate() {
        println!("    [{i}] {}", folder.display());
    }
    println!("  xspf playlists ({}):", config.xspf_playlists.len());
    for (i, xspf) in config.xspf_playlists.iter().enumerate() {
        println!("    [{i}] {}", xspf.display());
    }
}

fn config_path_display() -> String {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tui-musicplayer")
        .join("config.json")
        .display()
        .to_string()
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let args = Args::parse();

    // ── --list: print config and exit ──
    if args.list {
        let config = Config::load();
        print_config(&config);
        return Ok(());
    }

    let mut config = Config::load();

    // ── --add-folder: persist folder paths to config ──
    let mut config_changed = false;
    for folder in &args.add_folder {
        let canonical = canonicalize_path(folder);
        if config.music_folders.iter().any(|f| f == &canonical) {
            info!("Folder already in config: {}", canonical.display());
        } else {
            info!("Adding folder to config: {} (from {})", canonical.display(), folder.display());
            config.music_folders.push(canonical);
            config_changed = true;
        }
    }

    // ── --add-xspf: persist XSPF paths to config ──
    for xspf in &args.add_xspf {
        let canonical = canonicalize_path(xspf);
        if config.xspf_playlists.iter().any(|p| p == &canonical) {
            info!("XSPF already in config: {}", canonical.display());
        } else {
            info!("Adding XSPF to config: {} (from {})", canonical.display(), xspf.display());
            config.xspf_playlists.push(canonical);
            config_changed = true;
        }
    }

    if config_changed {
        if let Err(e) = config.save() {
            warn!("Failed to save config: {e}");
        } else {
            eprintln!("Config updated: {} folders, {} xspf playlists",
                config.music_folders.len(), config.xspf_playlists.len());
        }
    }

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
