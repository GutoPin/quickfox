use quickfox::{
    app::{App, Screen},
    persistence::{Profile, Settings, ThemeName},
    ui::{
        self,
        theme::{ColorMode, Theme},
    },
};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use std::time::Duration;
fn render(app: &App, width: u16, height: u16, mode: ColorMode) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, mode, Duration::from_secs(31)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                + "\n"
        })
        .collect()
}
#[test]
fn every_screen_renders_at_supported_sizes_and_themes() {
    for theme in ThemeName::ALL {
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
                let mut app = App::new(
                    Settings {
                        theme,
                        ..Settings::default()
                    },
                    Profile::default(),
                    42,
                );
                app.start_campaign();
                app.session.as_mut().unwrap().input('a', Duration::ZERO);
                app.tick(Duration::from_secs(30), 0);
                app.screen = screen;
                let output = render(&app, width, height, ColorMode::TrueColor);
                assert!(output.contains("QUICKFOX"));
                assert!(
                    output.contains(match screen {
                        Screen::Home => "Continue campaign",
                        Screen::Menu => "Focused practice",
                        Screen::Onboarding => "Follow the words",
                        Screen::Campaign => "Mastery",
                        Screen::QuickPlay => "Content",
                        Screen::Playing => "accuracy",
                        Screen::Paused => "Menu (keep session)",
                        Screen::Results => "NET WPM",
                        Screen::Statistics => "RECENT SESSIONS",
                        Screen::Settings => "Typing caret",
                        Screen::Help => "Ctrl+C",
                        Screen::Focused => "focused practice",
                        Screen::Daily => "No attempt yet today",
                    }),
                    "{screen:?} at {width}x{height}"
                );
            }
        }
    }
}
#[test]
fn tiny_terminals_have_a_recoverable_message() {
    let app = App::new(Settings::default(), Profile::default(), 42);
    for (w, h) in [(1, 1), (20, 10), (59, 19)] {
        let output = render(&app, w, h, ColorMode::Mono);
        assert!(!output.is_empty());
    }
    assert!(render(&app, 59, 19, ColorMode::Mono).contains("Resize"));
}
#[test]
fn prompt_wrapping_preserves_every_character_and_keeps_rows_in_bounds() {
    let prompt: Vec<char> = "a longwordthatexceedsthewidth and more words "
        .chars()
        .collect();
    for width in 1..25 {
        let rows = ui::prompt_rows(&prompt, width);
        let restored: String = rows.iter().flat_map(|r| prompt[r.clone()].iter()).collect();
        assert_eq!(restored, prompt.iter().collect::<String>());
        assert!(rows.iter().all(|r| r.len() <= width));
    }
}
#[test]
fn ascii_mode_uses_no_unicode_interface_symbols() {
    let mut app = App::new(
        Settings {
            ascii: true,
            ..Settings::default()
        },
        Profile::default(),
        42,
    );
    for screen in [
        Screen::Home,
        Screen::Campaign,
        Screen::Settings,
        Screen::Playing,
    ] {
        app.start_campaign();
        app.screen = screen;
        assert!(
            render(&app, 100, 30, ColorMode::TrueColor).is_ascii(),
            "{screen:?}"
        );
    }
}
fn luminance(color: Color) -> f64 {
    let Color::Rgb(r, g, b) = color else {
        panic!("expected RGB");
    };
    let f = |n: u8| {
        let x = f64::from(n) / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}
#[test]
fn truecolor_text_tokens_meet_aa_contrast() {
    for name in ThemeName::ALL
        .into_iter()
        .filter(|name| *name != ThemeName::Mono)
    {
        let theme = Theme::new(
            &Settings {
                theme: name,
                ..Settings::default()
            },
            ColorMode::TrueColor,
        );
        for bg in [theme.bg, theme.panel] {
            for fg in [
                theme.fg,
                theme.muted,
                theme.accent,
                theme.success,
                theme.error,
                theme.gold,
                theme.secondary,
                theme.tertiary,
            ] {
                let (a, b) = (luminance(fg), luminance(bg));
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                    "{name:?} {fg:?} on {bg:?}"
                );
            }
        }
    }
}

#[test]
fn caret_stays_visible_after_multiple_row_crossings_in_compact_mode() {
    use ratatui::style::Modifier;
    for (width, height) in [(60, 20), (80, 24), (100, 30)] {
        let mut app = App::new(Settings::default(), Profile::default(), 42);
        app.start_campaign();
        for position in 0..240 {
            let character = app.session.as_ref().unwrap().prompt[position];
            app.session
                .as_mut()
                .unwrap()
                .input(character, Duration::from_millis(position as u64 * 50));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| {
                    ui::render(frame, &app, ColorMode::TrueColor, Duration::from_secs(15))
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let caret_count = buffer
                .content()
                .iter()
                .filter(|cell| cell.modifier.contains(Modifier::REVERSED))
                .count();
            assert_eq!(
                caret_count, 1,
                "missing caret at position {position}, {width}x{height}"
            );
        }
    }
}

#[test]
fn reviewed_home_and_paperwave_snapshots_match() {
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    assert_eq!(
        render(&app, 80, 24, ColorMode::TrueColor),
        include_str!("fixtures/home-80x24.txt")
    );
    app.settings.theme = ThemeName::Paperwave;
    app.screen = Screen::Settings;
    assert_eq!(
        render(&app, 100, 30, ColorMode::TrueColor),
        include_str!("fixtures/settings-paperwave-100x30.txt")
    );
}

#[test]
fn configured_spacing_and_prompt_width_keep_the_caret_visible() {
    use ratatui::style::Modifier;
    for spacing in 0..=2 {
        let mut settings = Settings::default();
        settings.layout.line_spacing = spacing;
        settings.layout.prompt_width = 32;
        let mut app = App::new(settings, Profile::default(), 42);
        app.start_campaign();
        for i in 0..110 {
            let c = app.session.as_ref().unwrap().prompt[i];
            app.session
                .as_mut()
                .unwrap()
                .input(c, Duration::from_millis(i as u64));
        }
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal
            .draw(|frame| ui::render(frame, &app, ColorMode::TrueColor, Duration::ZERO))
            .unwrap();
        assert_eq!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .filter(|c| c.modifier.contains(Modifier::REVERSED))
                .count(),
            1
        );
    }
}
#[test]
fn command_menu_keeps_selected_last_item_visible_at_minimum_size() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.handle(
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
        Duration::ZERO,
        0,
    );
    for _ in 0..20 {
        app.handle(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Duration::ZERO,
            0,
        );
    }
    let output = render(&app, 60, 20, ColorMode::TrueColor);
    assert!(output.contains("> Quit Quickfox"));
    assert!(output.contains("Esc close"));
}

#[test]
fn settings_scroll_exposes_every_action_and_configured_help_matches_bindings() {
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.screen = Screen::Settings;
    for (selected, label) in [
        "Theme",
        "High contrast",
        "Reduced motion",
        "ASCII symbols",
        "Typing caret",
        "Sidebar",
        "Key sounds",
        "Sound profile",
        "Volume",
        "Preview sound",
        "Adaptive prompts",
        "Reload JSON",
    ]
    .iter()
    .enumerate()
    {
        app.selection = selected;
        assert!(render(&app, 60, 20, ColorMode::TrueColor).contains(&format!("> {label}")));
    }
    app.settings.keybindings.menu = "f2".into();
    app.settings.keybindings.commands = "f3".into();
    app.settings.keybindings.restart = "f4".into();
    app.settings.keybindings.reload_config = "f6".into();
    app.screen = Screen::Help;
    let output = render(&app, 60, 20, ColorMode::TrueColor);
    for key in ["f2", "f3", "f4", "f6", "Tab", "Ctrl+C"] {
        assert!(output.contains(key), "{key}");
    }
    assert!(!output.contains("ctrl+p"));
}

#[test]
fn compact_lists_disclose_ranges_and_typing_shows_only_practice_controls() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.screen = Screen::Settings;
    assert!(render(&app, 60, 20, ColorMode::TrueColor).contains("1 Appearance"));
    app.selection = 11;
    assert!(render(&app, 60, 20, ColorMode::TrueColor).contains("Reload JSON"));
    app.handle(
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
        Duration::ZERO,
        0,
    );
    assert!(render(&app, 60, 20, ColorMode::TrueColor).contains("1-8 / 11"));
    app.palette.as_mut().unwrap().selection = app.commands().len() - 1;
    assert!(render(&app, 60, 20, ColorMode::TrueColor).contains("4-11 / 11"));
    app.palette = None;
    app.start_campaign();
    let output = render(&app, 60, 20, ColorMode::TrueColor);
    assert!(output.contains("Esc pause   Backspace correct   Tab menu"));
    assert!(!output.contains("commands"));
    assert!(!output.contains("reload"));
}

#[test]
fn nightfall_is_neutral_with_distinct_gray_surfaces() {
    for mode in [ColorMode::TrueColor, ColorMode::Ansi256, ColorMode::Ansi16] {
        let theme = Theme::new(&Settings::default(), mode);
        for color in [
            theme.bg,
            theme.panel,
            theme.fg,
            theme.muted,
            theme.accent,
            theme.success,
            theme.error,
            theme.gold,
            theme.secondary,
            theme.tertiary,
            theme.selection,
            theme.border,
        ] {
            assert!(
                match color {
                    Color::Rgb(r, g, b) => r == g && g == b,
                    Color::Indexed(index) => index == 16 || index >= 231,
                    Color::Black | Color::White | Color::Gray | Color::DarkGray => true,
                    _ => false,
                },
                "chromatic Nightfall token: {color:?}"
            );
        }
        assert_ne!(theme.bg, theme.selection);
        if mode != ColorMode::Ansi16 {
            assert_ne!(theme.bg, theme.panel);
        }
    }
}

#[test]
fn sidebar_shows_progress_only_on_wide_browsing_screens() {
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    for enabled in [false, true] {
        app.settings.layout.sidebar = enabled;
        for width in [111, 112] {
            app.go(Screen::Menu);
            let menu = render(&app, width, 24, ColorMode::Mono);
            assert_eq!(menu.matches("Quick play").count(), 1);
            assert!(!menu.contains("Campaign progress"));
            app.go(Screen::Onboarding);
            assert!(!render(&app, width, 24, ColorMode::Mono).contains("Campaign progress"));
            app.go(Screen::Settings);
            app.selection = 5;
            let settings = render(&app, width, 24, ColorMode::Mono);
            assert_eq!(
                settings.contains("Campaign progress"),
                enabled && width >= 112
            );
            assert!(settings.contains("112+ columns"));
        }
    }
}

#[test]
fn confirmation_replaces_underlying_controls() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.start_campaign();
    app.pause(Duration::ZERO);
    app.handle(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        Duration::ZERO,
        0,
    );
    for (width, height) in [(60, 20), (120, 40)] {
        let output = render(&app, width, height, ColorMode::Mono);
        assert!(output.contains("Enter replace   Esc keep session"));
        assert!(!output.contains("Enter resume"));
        assert!(!output.contains("Keep going"));
    }
}

#[test]
fn underline_caret_remains_distinct_after_an_error() {
    use ratatui::style::Modifier;
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.settings.underline_caret = true;
    app.start_campaign();
    let session = app.session.as_mut().unwrap();
    session.prompt = "a b".chars().collect();
    session.input('x', Duration::ZERO);
    for mode in [ColorMode::TrueColor, ColorMode::Mono] {
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal
            .draw(|frame| ui::render(frame, &app, mode, Duration::ZERO))
            .unwrap();
        let cells = terminal.backend().buffer().content();
        let caret = cells.iter().position(|c| c.symbol() == "·").unwrap();
        assert!(cells[caret].modifier.contains(Modifier::UNDERLINED));
        assert!(!cells[caret].modifier.contains(Modifier::REVERSED));
        let mistake = &cells[caret - 1];
        assert!(mistake.modifier.contains(Modifier::UNDERLINED));
        // Colored themes fill a mistake with the error color; Mono reverses it.
        if mode == ColorMode::Mono {
            assert!(mistake.modifier.contains(Modifier::REVERSED));
        } else {
            assert_eq!(mistake.bg, Theme::new(&app.settings, mode).error);
            assert_ne!(mistake.bg, cells[caret].bg);
        }
    }
}

#[test]
fn wide_keyboard_guide_tracks_input_and_resize() {
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.start_campaign();
    app.session.as_mut().unwrap().prompt = "a b".chars().collect();
    for (size, expected, typed) in [
        ((240, 70), "Next key   a", 'a'),
        ((160, 50), "Next key   Space", ' '),
    ] {
        let output = render(&app, size.0, size.1, ColorMode::Mono);
        assert!(output.contains(expected));
        app.session.as_mut().unwrap().input(typed, Duration::ZERO);
    }
    assert!(!render(&app, 80, 24, ColorMode::Mono).contains("Next key"));
    assert!(render(&app, 240, 70, ColorMode::Mono).contains("Next key   b"));
}
