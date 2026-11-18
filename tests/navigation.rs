use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use quickfox::{
    app::{App, Screen},
    persistence::{Profile, Settings},
};
use std::time::Duration;
fn at(n: u64) -> Duration {
    Duration::from_secs(n)
}
fn key(app: &mut App, code: KeyCode, time: u64) {
    app.handle(KeyEvent::new(code, KeyModifiers::NONE), at(time), time);
}
fn ctrl(app: &mut App, c: char, time: u64) {
    app.handle(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        at(time),
        time,
    );
}
fn app() -> App {
    App::new(Settings::default(), Profile::default(), 42)
}
#[test]
fn menu_and_resume_keep_prompt_input_and_active_time() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Char('a'), 0);
    let prompt = a.session.as_ref().unwrap().prompt.clone();
    ctrl(&mut a, 'g', 5);
    assert_eq!(a.screen, Screen::Menu);
    assert!(a.session.as_ref().unwrap().paused);
    key(&mut a, KeyCode::Enter, 100);
    a.tick(at(105), 105);
    let session = a.session.as_ref().unwrap();
    assert_eq!(session.prompt, prompt);
    assert_eq!(session.typed, vec!['a']);
    assert_eq!(session.elapsed(), at(10));
}
#[test]
fn escape_from_pause_returns_menu_then_home_without_exiting() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Esc, 0);
    key(&mut a, KeyCode::Esc, 1);
    assert_eq!(a.screen, Screen::Menu);
    assert!(a.has_session());
    key(&mut a, KeyCode::Esc, 2);
    assert!(!a.quit);
}
#[test]
fn command_search_routes_to_settings_and_keeps_session_paused() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Char('a'), 0);
    ctrl(&mut a, 'p', 3);
    assert!(a.session.as_ref().unwrap().paused);
    for c in "settings".chars() {
        key(&mut a, KeyCode::Char(c), 5);
    }
    assert_eq!(a.commands().len(), 1);
    key(&mut a, KeyCode::Enter, 6);
    assert_eq!(a.screen, Screen::Settings);
    assert!(a.session.as_ref().unwrap().paused);
    ctrl(&mut a, 'g', 10);
    key(&mut a, KeyCode::Enter, 20);
    a.tick(at(22), 22);
    assert_eq!(a.session.as_ref().unwrap().elapsed(), at(5));
}
#[test]
fn closing_palette_resumes_only_if_opened_during_typing() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Char('a'), 0);
    ctrl(&mut a, 'p', 2);
    ctrl(&mut a, 'p', 20);
    assert_eq!(a.screen, Screen::Playing);
    a.tick(at(23), 23);
    assert_eq!(a.session.as_ref().unwrap().elapsed(), at(5));
    ctrl(&mut a, 'g', 24);
    ctrl(&mut a, 'p', 25);
    assert!(a.palette.is_some());
    key(&mut a, KeyCode::Esc, 26);
    assert_eq!(a.screen, Screen::Menu);
    assert!(a.session.as_ref().unwrap().paused);
}
#[test]
fn replacement_requires_confirmation_and_cancel_keeps_input() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Char('a'), 0);
    ctrl(&mut a, 'g', 1);
    a.go(Screen::QuickPlay);
    key(&mut a, KeyCode::Enter, 2);
    assert!(a.pending.is_some());
    key(&mut a, KeyCode::Esc, 3);
    assert_eq!(a.session.as_ref().unwrap().typed, vec!['a']);
    assert!(a.has_session());
    key(&mut a, KeyCode::Enter, 4);
    key(&mut a, KeyCode::Enter, 5);
    assert!(a.pending.is_none());
    assert_eq!(a.screen, Screen::Playing);
    assert!(a.session.as_ref().unwrap().typed.is_empty());
    assert_eq!(a.profile.sessions, 0);
}
#[test]
fn menu_shortcut_closes_palette_and_cancels_replacement_safely() {
    let mut a = app();
    a.start_campaign();
    ctrl(&mut a, 'p', 0);
    ctrl(&mut a, 'g', 1);
    assert!(a.palette.is_none());
    assert_eq!(a.screen, Screen::Menu);
    ctrl(&mut a, 'r', 2);
    assert!(a.pending.is_some());
    ctrl(&mut a, 'g', 3);
    assert!(a.pending.is_none());
    assert!(a.has_session());
}
#[test]
fn empty_search_and_custom_shortcuts_are_safe() {
    let mut a = app();
    a.settings.keybindings.commands = "f2".into();
    a.settings.keybindings.menu = "f3".into();
    key(&mut a, KeyCode::F(2), 0);
    assert!(a.palette.is_some());
    for c in "not a command".chars() {
        key(&mut a, KeyCode::Char(c), 0);
    }
    assert!(a.commands().is_empty());
    key(&mut a, KeyCode::Down, 0);
    key(&mut a, KeyCode::Enter, 0);
    assert!(a.palette.is_some());
    key(&mut a, KeyCode::F(3), 0);
    assert!(a.palette.is_none());
    assert_eq!(a.screen, Screen::Menu);
}

#[test]
fn retry_preserves_the_original_mode_after_browsing_other_options() {
    let mut a = app();
    a.quick_mode = 3;
    a.start(1, None);
    let length = a
        .session
        .as_ref()
        .unwrap()
        .prompt
        .iter()
        .filter(|c| **c == ' ')
        .count();
    ctrl(&mut a, 'g', 0);
    a.quick_mode = 2;
    ctrl(&mut a, 'r', 1);
    key(&mut a, KeyCode::Enter, 2);
    assert_eq!(a.session.as_ref().unwrap().duration, None);
    assert_eq!(
        a.session
            .as_ref()
            .unwrap()
            .prompt
            .iter()
            .filter(|c| **c == ' ')
            .count(),
        length
    );
}

#[test]
fn tab_menu_and_command_palette_are_distinct_on_every_screen() {
    for screen in [
        Screen::Home,
        Screen::Menu,
        Screen::Playing,
        Screen::Paused,
        Screen::QuickPlay,
        Screen::Campaign,
        Screen::Focused,
        Screen::Daily,
        Screen::Onboarding,
        Screen::Settings,
        Screen::Statistics,
        Screen::Help,
        Screen::Results,
    ] {
        let mut a = app();
        a.start_campaign();
        a.session.as_mut().unwrap().input('a', at(0));
        if screen != Screen::Playing {
            a.pause(at(0));
        }
        a.screen = screen;
        key(&mut a, KeyCode::Tab, 1);
        assert_eq!(a.screen, Screen::Menu, "Tab from {screen:?}");
        assert!(a.palette.is_none());
        assert!(a.session.as_ref().unwrap().paused);
        ctrl(&mut a, 'p', 2);
        assert!(a.palette.is_some());
        assert_eq!(a.screen, Screen::Menu);
        key(&mut a, KeyCode::Esc, 3);
        assert!(a.palette.is_none());
        assert_eq!(a.screen, Screen::Menu);
        assert_eq!(a.session.as_ref().unwrap().typed, vec!['a']);
    }
}

#[test]
fn all_menu_destinations_and_home_escape_match_their_labels() {
    for (index, expected) in [
        (1, Screen::QuickPlay),
        (2, Screen::Campaign),
        (3, Screen::Focused),
        (4, Screen::Daily),
        (5, Screen::Statistics),
        (6, Screen::Settings),
        (7, Screen::Help),
    ] {
        let mut a = app();
        key(&mut a, KeyCode::Tab, 0);
        for _ in 0..index {
            key(&mut a, KeyCode::Down, 0);
        }
        key(&mut a, KeyCode::Enter, 0);
        assert_eq!(a.screen, expected);
        key(&mut a, KeyCode::Esc, 0);
        assert_eq!(a.screen, Screen::Menu);
        key(&mut a, KeyCode::Esc, 0);
        assert_eq!(a.screen, Screen::Home);
    }
}

#[test]
fn global_reload_and_restart_work_from_commands_and_modifiers_do_not_type() {
    let mut a = app();
    a.start_campaign();
    key(&mut a, KeyCode::Char('a'), 0);
    ctrl(&mut a, 'p', 1);
    key(&mut a, KeyCode::F(5), 2);
    assert!(a.reload_requested);
    assert!(a.palette.is_none());
    assert!(a.session.as_ref().unwrap().paused);
    ctrl(&mut a, 'p', 3);
    ctrl(&mut a, 'r', 4);
    assert!(a.pending.is_some());
    assert!(a.palette.is_none());
    key(&mut a, KeyCode::Esc, 5);
    a.home(at(5));
    ctrl(&mut a, 'q', 5);
    assert!(!a.quit);
    ctrl(&mut a, 'p', 6);
    ctrl(&mut a, ' ', 6);
    assert!(a.palette.as_ref().unwrap().query.is_empty());
}

#[test]
fn settings_controls_wrap_scroll_change_volume_and_do_not_save_preview_actions() {
    let mut a = app();
    a.go(Screen::Settings);
    key(&mut a, KeyCode::Up, 0);
    assert_eq!(a.selection, 11);
    key(&mut a, KeyCode::Enter, 0);
    assert!(a.reload_requested);
    assert!(!a.settings_dirty);
    a.selection = 8;
    for _ in 0..30 {
        key(&mut a, KeyCode::Left, 0);
    }
    assert_eq!(a.settings.sound.volume, 0);
    for _ in 0..45 {
        key(&mut a, KeyCode::Right, 0);
    }
    assert_eq!(a.settings.sound.volume, 200);
    a.selection = 9;
    a.settings_dirty = false;
    key(&mut a, KeyCode::Enter, 0);
    assert!(!a.settings_dirty);
    assert!(a.notice.as_ref().unwrap().contains("Enable"));
}

#[test]
fn settings_sections_and_recorded_profiles_are_keyboard_accessible() {
    use quickfox::{app::settings_section, config::SoundProfile};
    let mut a = app();
    a.go(Screen::Settings);
    for (digit, section, row) in [('4', 3, 11), ('3', 2, 10), ('2', 1, 6), ('1', 0, 0)] {
        key(&mut a, KeyCode::Char(digit), 0);
        assert_eq!(a.selection, row);
        assert_eq!(settings_section(a.selection), section);
    }
    key(&mut a, KeyCode::Char('2'), 0);
    key(&mut a, KeyCode::Down, 0);
    for profile in [
        SoundProfile::Soft,
        SoundProfile::Typewriter,
        SoundProfile::EgOreo,
        SoundProfile::NkCream,
        SoundProfile::Creamy,
    ] {
        key(&mut a, KeyCode::Right, 0);
        assert_eq!(a.settings.sound.profile, profile);
    }
    key(&mut a, KeyCode::Tab, 0);
    assert_eq!(a.screen, Screen::Menu);
    assert!(a.palette.is_none());
    ctrl(&mut a, 'p', 0);
    assert!(a.palette.is_some());
}

#[test]
fn home_starts_the_first_practice_and_resumes_after_browsing() {
    let mut a = app();
    key(&mut a, KeyCode::Down, 0);
    key(&mut a, KeyCode::Enter, 0);
    assert_eq!(a.screen, Screen::Playing);
    let session = a.session.as_ref().unwrap();
    assert_eq!(session.remaining(), Some(at(30)));
    assert!(!session.has_started());
    key(&mut a, KeyCode::Char('a'), 1);
    key(&mut a, KeyCode::Tab, 2);
    key(&mut a, KeyCode::Esc, 3);
    key(&mut a, KeyCode::Enter, 4);
    assert_eq!(a.screen, Screen::Playing);
    assert_eq!(a.session.as_ref().unwrap().typed, vec!['a']);
}

#[test]
fn enter_retries_the_failed_mission_even_when_replaying_an_earlier_level() {
    let mut a = app();
    a.profile.completed_missions = 6;
    a.start(0, Some(0));
    key(&mut a, KeyCode::Char('x'), 0);
    a.tick(at(30), 30);
    assert_eq!(a.screen, Screen::Results);
    key(&mut a, KeyCode::Enter, 31);
    assert_eq!(a.session.as_ref().unwrap().mission, Some(0));
}
