use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
        Event, KeyCode, KeyModifiers,
    },
    execute,
};
use quickfox::{
    app::{App, Screen},
    audio::SoundEngine,
    persistence::{Storage, ThemeName},
    ui::{self, theme::ColorMode},
};
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Parser)]
#[command(
    version,
    about = "Touch typing practice for the terminal, in English.",
    after_help = "Runs offline. Progress is stored on this device."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_enum,
        help = "Choose and save a color theme"
    )]
    theme: Option<ThemeName>,
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Use an isolated config/ and data/ directory"
    )]
    data_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Use a custom JSON configuration file"
    )]
    config: Option<PathBuf>,
    #[arg(long, global = true, help = "Use ASCII interface symbols")]
    ascii: bool,
    #[arg(long, global = true, help = "Disable the short results animation")]
    reduced_motion: bool,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Start a quick practice session, with every level available.
    Play {
        #[arg(long, value_parser = ["30", "60", "120"], conflicts_with = "words")]
        seconds: Option<String>,
        #[arg(long, value_parser = ["25", "50", "100"])]
        words: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        level: Option<u8>,
    },
    /// Start your next campaign mission.
    Campaign,
    /// Start today's deterministic 60-second challenge (UTC).
    Daily,
    /// Practice your most difficult letter pattern.
    Practice,
    /// Inspect, initialize, or validate your editable JSON configuration.
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },
    /// Print local terminal capabilities and save paths without opening the UI.
    Doctor,
    /// Write your progress as JSON to standard output.
    Export,
}
#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the resolved configuration path without creating files.
    Path,
    /// Create a complete editable config and its JSON schema; never overwrite a file.
    Init,
    /// Validate the configuration without starting the terminal UI.
    Validate,
    /// Print the effective JSON with all defaults filled in.
    Show,
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableFocusChange);
        ratatui::restore();
    }
}
fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = resolve_config_path(cli.data_dir.as_deref(), cli.config.as_deref())?;
    if let Some(Command::Config { ref action }) = cli.command {
        return configuration(action, &config_path);
    }
    if matches!(cli.command, Some(Command::Doctor)) {
        return doctor(cli.data_dir, config_path);
    }

    let interactive = !matches!(cli.command, Some(Command::Export));
    if interactive && (!io::stdin().is_terminal() || !io::stdout().is_terminal()) {
        bail!(
            "Quickfox needs an interactive terminal. Run it directly in your terminal, or use `quickfox doctor` / `quickfox export` for plain output."
        );
    }
    if interactive && std::env::var("TERM").is_ok_and(|term| term == "dumb") {
        bail!(
            "This terminal reports TERM=dumb. Open a terminal with cursor control, then run Quickfox again."
        );
    }
    let mut storage = Storage::open(cli.data_dir.as_deref())?;
    storage.config_path = config_path;
    if cli.config.is_some() && !storage.config_path.exists() {
        bail!(
            "Configuration file does not exist. Run with the same --config FILE and `config init` first."
        );
    }

    let (mut settings, profile, warnings) = storage.load()?;
    for warning in &warnings {
        eprintln!("{warning}");
    }
    if matches!(cli.command, Some(Command::Export)) {
        serde_json::to_writer_pretty(io::stdout().lock(), &profile)?;
        println!();
        return Ok(());
    }
    if let Some(theme) = cli.theme {
        settings.theme = theme;
    }
    if cli.ascii {
        settings.ascii = true;
    }
    if cli.reduced_motion {
        settings.reduced_motion = true;
    }
    let mut app = App::new(settings, profile, timestamp());
    app.config_path = storage.config_path.display().to_string();
    app.settings_dirty =
        !storage.config_path.exists() || cli.theme.is_some() || cli.ascii || cli.reduced_motion;
    app.notice = if warnings.is_empty() {
        None
    } else {
        Some("Damaged save backed up. Recovery details were printed before startup.".into())
    };
    match cli.command {
        Some(Command::Play {
            seconds,
            words,
            level,
        }) => {
            if let Some(level) = level {
                app.quick_level = usize::from(level - 1);
            }
            if words.is_some() || seconds.is_some() {
                app.quick_mode = if let Some(words) = words {
                    match words.as_str() {
                        "25" => 3,
                        "50" => 4,
                        _ => 5,
                    }
                } else {
                    match seconds.as_deref() {
                        Some("30") => 0,
                        Some("120") => 2,
                        _ => 1,
                    }
                };
            }
            app.start(app.quick_level, None);
        }
        Some(Command::Campaign) => app.start_campaign(),
        Some(Command::Daily) => app.start_daily(timestamp() / 86400),
        Some(Command::Practice) => app.start_focused(0),
        _ => {}
    }
    let result = run(&mut app, &storage);
    // Errors are printed only after the guard has restored the normal terminal.
    result
}
fn run(app: &mut App, storage: &Storage) -> Result<()> {
    let mut terminal = ratatui::init();
    let _guard = TerminalGuard;
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableFocusChange);
        previous_hook(info);
    }));
    execute!(io::stdout(), EnableBracketedPaste, EnableFocusChange)?;
    let origin = Instant::now();
    let mut audio = SoundEngine::default();
    let mut unsaved_settings = false;
    let mut unsaved_profile = false;
    loop {
        let now = origin.elapsed();
        app.tick(now, timestamp());
        if app.reload_requested {
            app.reload_requested = false;
            match storage.reload_settings() {
                Ok(settings) => {
                    app.apply_settings(settings);
                    app.settings_dirty = false;
                    unsaved_settings = false;
                    app.notice = Some("JSON configuration reloaded. Session kept paused.".into());
                }
                Err(error) => app.notice = Some(format!("Config kept unchanged: {error}")),
            }
        }
        audio.configure(&app.settings.sound);
        app.audio_status = audio.status();
        if app.settings_dirty {
            app.settings_dirty = false;
            unsaved_settings = match storage.save_settings(&app.settings) {
                Ok(()) => false,
                Err(error) => {
                    app.notice = Some(format!("Settings could not be saved: {error}"));
                    true
                }
            };
        }
        if app.profile_dirty {
            app.profile_dirty = false;
            unsaved_profile = match storage.save_profile(&app.profile) {
                Ok(()) => false,
                Err(error) => {
                    app.notice = Some(format!("Progress could not be saved: {error}"));
                    true
                }
            };
        }
        if app.quit {
            break;
        }
        let size = terminal.size()?;
        let too_small = size.width < ui::MIN_WIDTH || size.height < ui::MIN_HEIGHT;
        if too_small {
            app.pause(now);
        }
        terminal.draw(|frame| ui::render(frame, app, ColorMode::configured(&app.settings), now))?;
        let mut input = if app.needs_tick(now) || audio.is_starting() {
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            event::read()?
        } else {
            event::read()?
        };
        // Drain a bounded burst without discarding a key or redrawing for each queued byte.
        for index in 0..64 {
            let boundary = !matches!(&input, Event::Key(_));
            let previous = (app.screen, app.palette.is_some(), app.pending.is_some());
            match input {
                Event::Key(key) => {
                    if !too_small
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        app.handle(key, origin.elapsed(), timestamp());
                        audio.configure(&app.settings.sound);
                        if std::mem::take(&mut app.sound_requested) {
                            if app.screen == Screen::Settings {
                                audio.preview();
                            } else {
                                audio.play();
                            }
                        }
                    }
                }
                Event::FocusLost => app.pause(origin.elapsed()),
                Event::Paste(_) if app.screen == Screen::Playing => {
                    app.notice = Some("Paste ignored. Type the words to practice.".into())
                }
                Event::Resize(_, _) | Event::FocusGained => {}
                _ => {}
            }
            if app.quit
                || boundary
                || app.reload_requested
                || index == 63
                || previous != (app.screen, app.palette.is_some(), app.pending.is_some())
                || !event::poll(Duration::ZERO)?
            {
                break;
            }
            input = event::read()?;
        }
    }
    if unsaved_settings {
        storage
            .save_settings(&app.settings)
            .context("Settings remain unsaved")?;
    }
    if unsaved_profile {
        storage
            .save_profile(&app.profile)
            .context("Progress remains unsaved")?;
    }
    Ok(())
}
fn doctor(root: Option<PathBuf>, config_path: PathBuf) -> Result<()> {
    let mode = ColorMode::detect();
    let (_config, data) = if let Some(root) = root {
        (root.join("config"), root.join("data"))
    } else {
        let dirs = quickfox::persistence::project_dirs()
            .context("Cannot locate your home directory; use --data-dir PATH")?;
        (
            dirs.config_dir().to_path_buf(),
            dirs.data_dir().to_path_buf(),
        )
    };
    println!("Quickfox {}\n", env!("CARGO_PKG_VERSION"));
    println!("Input terminal:  {}", io::stdin().is_terminal());
    println!("Output terminal: {}", io::stdout().is_terminal());
    println!("Color mode:      {mode:?}");
    println!(
        "Terminal size:   {}",
        crossterm::terminal::size()
            .map(|(w, h)| format!("{w} x {h}"))
            .unwrap_or_else(|_| "unavailable".into())
    );
    println!("Config:          {}", config_path.display());
    println!("Progress:        {}", data.join("progress.json").display());
    println!("Config editing:  quickfox config show / config validate; F5 reloads in the UI");
    println!("Minimum size:    {} x {}", ui::MIN_WIDTH, ui::MIN_HEIGHT);
    println!(
        "Network:         never used\nContent:         embedded English words and original sentences"
    );
    Ok(())
}

fn resolve_config_path(
    root: Option<&std::path::Path>,
    custom: Option<&std::path::Path>,
) -> Result<PathBuf> {
    if let Some(path) = custom {
        return Ok(if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()?.join(path)
        });
    }
    if let Some(root) = root {
        return Ok(root.join("config/config.json"));
    }
    Ok(quickfox::persistence::project_dirs()
        .context("Cannot locate home; use --config FILE")?
        .config_dir()
        .join("config.json"))
}
fn configuration(action: &ConfigCommand, path: &std::path::Path) -> Result<()> {
    use quickfox::config::{Settings, read_config};
    use std::io::Write;
    match action {
        ConfigCommand::Path => println!("{}", path.display()),
        ConfigCommand::Show => {
            let settings = if path.exists() {
                read_config(path)?
            } else {
                Settings::default()
            };
            serde_json::to_writer_pretty(io::stdout().lock(), &settings)?;
            println!();
        }
        ConfigCommand::Validate => {
            read_config(path)?;
            println!("Valid configuration: {}", path.display());
        }
        ConfigCommand::Init => {
            if path.exists() {
                bail!(
                    "Configuration already exists at {}. Use `config show` or edit that file.",
                    path.display()
                );
            }
            let parent = path.parent().context("Config path has no parent")?;
            std::fs::create_dir_all(parent)?;
            let schema_path = parent.join("quickfox.schema.json");
            if !schema_path.exists() {
                let mut schema = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(schema_path)?;
                schema.write_all(include_bytes!("../config/quickfox.schema.json"))?;
            }
            let settings = Settings {
                schema: Some("./quickfox.schema.json".into()),
                ..Settings::default()
            };
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            serde_json::to_writer_pretty(&mut file, &settings)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            println!(
                "Created {}. Edit it, then run `config validate` or press F5 in the game.",
                path.display()
            );
        }
    }
    Ok(())
}
