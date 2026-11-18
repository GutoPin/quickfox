//! Deterministic previews rendered through the real UI and Ratatui TestBackend.
use quickfox::{
    app::{App, Screen},
    persistence::{Profile, Settings, ThemeName},
    ui::{self, theme::ColorMode},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use serde_json::json;
use std::{fs, path::PathBuf, time::Duration};
fn color(color: Color, foreground: bool) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(index @ 232..=255) => {
            let gray = 8 + (index - 232) * 10;
            format!("#{gray:02x}{gray:02x}{gray:02x}")
        }
        Color::Indexed(index @ 16..=231) => {
            let steps = [0, 95, 135, 175, 215, 255];
            let n = usize::from(index - 16);
            format!(
                "#{:02x}{:02x}{:02x}",
                steps[n / 36],
                steps[n / 6 % 6],
                steps[n % 6]
            )
        }
        _ => if foreground { "#edf4ff" } else { "#0b1020" }.into(),
    }
}
fn main() -> anyhow::Result<()> {
    let directory = PathBuf::from(std::env::args().nth(1).unwrap_or("target/previews".into()));
    fs::create_dir_all(&directory)?;
    for (width, height) in [
        (60, 20),
        (80, 24),
        (100, 30),
        (120, 40),
        (160, 50),
        (240, 70),
    ] {
        for screen in [
            Screen::Home,
            Screen::Menu,
            Screen::Onboarding,
            Screen::Campaign,
            Screen::QuickPlay,
            Screen::Focused,
            Screen::Daily,
            Screen::Playing,
            Screen::Paused,
            Screen::Results,
            Screen::Statistics,
            Screen::Settings,
            Screen::Help,
        ] {
            let mut app = App::new(Settings::default(), Profile::default(), 42);
            app.start_campaign();
            let prompt = app.session.as_ref().unwrap().prompt.clone();
            for (i, c) in prompt.into_iter().take(42).enumerate() {
                app.session.as_mut().unwrap().input(
                    if i == 23 { 'x' } else { c },
                    Duration::from_millis(i as u64 * 140),
                );
            }
            if matches!(screen, Screen::Results | Screen::Statistics) {
                app.tick(Duration::from_secs(30), 0);
            }
            app.screen = screen;
            if screen == Screen::Home {
                app.profile = Profile::default();
                app.session = None;
            }
            capture(
                &directory,
                &format!("{screen:?}-{width}x{height}").to_lowercase(),
                &app,
                width,
                height,
            )?;
        }
    }
    let mut scrolling = App::new(Settings::default(), Profile::default(), 42);
    scrolling.start_campaign();
    let prompt = scrolling.session.as_ref().unwrap().prompt.clone();
    for (i, c) in prompt.into_iter().take(120).enumerate() {
        scrolling
            .session
            .as_mut()
            .unwrap()
            .input(c, Duration::from_millis(i as u64 * 100));
    }
    capture(&directory, "playing-scroll-60x20", &scrolling, 60, 20)?;
    for (width, height) in [(60, 20), (80, 24), (100, 30), (120, 40)] {
        let mut app = App::new(Settings::default(), Profile::default(), 42);
        for (pattern, count) in [("t", 9), ("th", 7), ("r", 4), ("ou", 3)] {
            app.profile.errors.insert(pattern.into(), count);
        }
        app.screen = Screen::Focused;
        capture(
            &directory,
            &format!("heatmap-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.handle(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('p'),
                crossterm::event::KeyModifiers::CONTROL,
            ),
            Duration::ZERO,
            0,
        );
        capture(
            &directory,
            &format!("commands-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.palette.as_mut().unwrap().selection = app.commands().len() - 1;
        capture(
            &directory,
            &format!("commands-scroll-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.palette.as_mut().unwrap().query = "no matches".into();
        capture(
            &directory,
            &format!("commands-empty-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.palette = None;
        app.start_campaign();
        app.handle(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('r'),
                crossterm::event::KeyModifiers::CONTROL,
            ),
            Duration::ZERO,
            0,
        );
        capture(
            &directory,
            &format!("confirm-{width}x{height}"),
            &app,
            width,
            height,
        )?;
    }
    for (width, height) in [(60, 20), (80, 24), (100, 30), (120, 40)] {
        let mut app = App::new(Settings::default(), Profile::default(), 42);
        app.screen = Screen::Settings;
        app.selection = 6;
        capture(
            &directory,
            &format!("settings-sound-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.selection = 7;
        app.settings.sound.profile = quickfox::config::SoundProfile::EgOreo;
        capture(
            &directory,
            &format!("settings-recorded-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.selection = 11;
        capture(
            &directory,
            &format!("settings-scroll-{width}x{height}"),
            &app,
            width,
            height,
        )?;
        app.start_campaign();
        app.session.as_mut().unwrap().input('a', Duration::ZERO);
        app.menu(Duration::from_secs(1));
        capture(
            &directory,
            &format!("menu-resume-{width}x{height}"),
            &app,
            width,
            height,
        )?;
    }
    for theme in ThemeName::ALL {
        let mut app = App::new(
            Settings {
                theme,
                ..Settings::default()
            },
            Profile::default(),
            42,
        );
        app.screen = Screen::Settings;
        capture(
            &directory,
            &format!("theme-{theme:?}").to_lowercase(),
            &app,
            100,
            30,
        )?;
    }
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.screen = Screen::Menu;
    capture_mode(
        &directory,
        "menu-ansi256-80x24",
        &app,
        80,
        24,
        ColorMode::Ansi256,
    )?;
    Ok(())
}
fn capture(
    directory: &std::path::Path,
    name: &str,
    app: &App,
    width: u16,
    height: u16,
) -> anyhow::Result<()> {
    capture_mode(directory, name, app, width, height, ColorMode::TrueColor)
}
fn capture_mode(
    directory: &std::path::Path,
    name: &str,
    app: &App,
    width: u16,
    height: u16,
    mode: ColorMode,
) -> anyhow::Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| ui::render(frame, app, mode, Duration::from_secs(31)))?;
    let buffer = terminal.backend().buffer();
    let mut rows = Vec::new();
    let mut plain = String::new();
    for y in 0..height {
        let mut cells = Vec::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let reverse = cell.modifier.contains(Modifier::REVERSED);
            let fg = if reverse { cell.bg } else { cell.fg };
            let bg = if reverse { cell.fg } else { cell.bg };
            cells.push(json!({"text":cell.symbol(),"fg":color(fg, !reverse),"bg":color(bg, reverse),"bold":cell.modifier.contains(Modifier::BOLD),"underline":cell.modifier.contains(Modifier::UNDERLINED)}));
            plain.push_str(cell.symbol());
        }
        rows.push(cells);
        plain.push('\n');
    }
    fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec(&json!({"width":width,"height":height,"cells":rows}))?,
    )?;
    fs::write(directory.join(format!("{name}.txt")), plain)?;
    Ok(())
}
