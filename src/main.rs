use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod app;
mod app_input;
mod app_playback;
mod locale;
mod player;
mod ui;
mod ui_panels;

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
    /// Paths to audio files or directories to scan (default: system music folder)
    paths: Vec<PathBuf>,

    /// Display language
    #[arg(long, value_enum)]
    lang: Option<LangArg>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let locale = args
        .lang
        .map(|l| Locale::from_lang(l.into()))
        .unwrap_or_else(detect_locale);

    let search_paths = if args.paths.is_empty() {
        vec![system_music_dir()]
    } else {
        args.paths
    };

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &search_paths, locale);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

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
    locale: Locale,
) -> anyhow::Result<()> {
    let mut app = app::App::new(paths.to_vec(), locale)?;
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                Event::Resize(_, _) => {
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
