use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use quickfox::{
    app::{App, Screen},
    content::{LocalAdaptiveProvider, PromptProvider},
    game::Session,
    persistence::{Profile, Settings},
};
use std::{collections::BTreeMap, time::Duration};
fn at(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn app() -> App {
    App::new(Settings::default(), Profile::default(), 42)
}

#[test]
fn corrections_do_not_erase_accuracy_or_inflate_net_speed() {
    let mut session = Session::new("abc".into(), None, 0, None);
    session.input('a', at(0));
    session.input('x', at(1));
    session.backspace(at(2));
    session.input('b', at(3));
    session.input('c', at(60));
    let m = session.metrics();
    assert_eq!(m.correct, 3);
    assert_eq!(m.attempts, 4);
    assert_eq!(m.corrections, 1);
    assert_eq!(m.accuracy, 75.0);
    assert!((m.net_wpm - 0.6).abs() < 0.001);
    assert!((m.gross_wpm - 0.8).abs() < 0.001);
    assert_eq!(session.errors["b"], 1);
    assert_eq!(session.errors["ab"], 1);
}
#[test]
fn clock_waits_for_first_key_and_excludes_pauses() {
    let mut s = Session::new("hello world".into(), Some(30), 0, None);
    s.tick(at(60));
    assert_eq!(s.elapsed(), Duration::ZERO);
    s.pause(at(60));
    s.resume(at(120));
    s.input('h', at(150));
    s.tick(at(155));
    assert_eq!(s.elapsed(), at(5));
    s.pause(at(155));
    s.tick(at(999));
    assert_eq!(s.elapsed(), at(5));
    s.input('x', at(180));
    assert_eq!(s.typed.len(), 1);
    s.resume(at(200));
    s.tick(at(225));
    assert!(s.finished);
    assert_eq!(s.elapsed(), at(30));
}
#[test]
fn deadline_prevents_late_keys_and_backspaces() {
    let mut s = Session::new("abc".into(), Some(30), 0, None);
    s.input('a', at(0));
    s.input('b', at(30));
    s.backspace(at(31));
    assert_eq!(s.typed, vec!['a']);
    assert_eq!(s.attempts, 1);
    assert_eq!(s.elapsed(), at(30));
}
#[test]
fn invalid_input_and_early_spaces_follow_explicit_rules() {
    let mut s = Session::new("a b".into(), None, 0, None);
    s.input('é', at(1));
    s.input('\n', at(1));
    s.input('\t', at(1));
    assert!(!s.has_started());
    s.backspace(at(1));
    assert_eq!(s.corrections, 0);
    s.input(' ', at(2));
    assert_eq!(s.typed.len(), 1);
    assert_eq!(s.errors["a"], 1);
    assert_eq!(s.metrics().accuracy, 0.0);
}
#[test]
fn consistency_includes_empty_full_windows() {
    let mut s = Session::new("a".repeat(100), Some(30), 0, None);
    for _ in 0..10 {
        s.input('a', at(0));
    }
    s.tick(at(10));
    assert_eq!(s.metrics().consistency, 0.0);
    let mut s = Session::new("a".repeat(100), Some(30), 0, None);
    for _ in 0..10 {
        s.input('a', at(0));
    }
    for _ in 0..10 {
        s.input('a', at(5));
    }
    s.tick(at(10));
    assert_eq!(s.metrics().consistency, 100.0);
}
#[test]
fn menu_shortcuts_are_characters_while_typing_and_repeats_are_ignored() {
    let mut app = app();
    app.start(1, None);
    app.handle(key(KeyCode::Char('q')), at(0), 0);
    app.handle(key(KeyCode::Char('r')), at(1), 0);
    let mut repeat = key(KeyCode::Char('x'));
    repeat.kind = KeyEventKind::Repeat;
    app.handle(repeat, at(2), 0);
    repeat.kind = KeyEventKind::Release;
    app.handle(repeat, at(2), 0);
    assert_eq!(app.session.as_ref().unwrap().typed, vec!['q', 'r']);
    assert!(!app.quit);
    assert_eq!(app.screen, Screen::Playing);
    app.handle(key(KeyCode::Esc), at(2), 0);
    assert_eq!(app.screen, Screen::Paused);
    app.handle(key(KeyCode::Enter), at(20), 0);
    assert_eq!(app.screen, Screen::Playing);
}
#[test]
fn all_fifteen_missions_unlock_in_order_and_results_record_once() {
    let mut app = app();
    for mission in 0..15 {
        app.start_campaign();
        assert_eq!(app.session.as_ref().unwrap().mission, Some(mission));
        let target: Vec<char> = app
            .session
            .as_ref()
            .unwrap()
            .prompt
            .iter()
            .take(25)
            .copied()
            .collect();
        for c in target {
            app.handle(key(KeyCode::Char(c)), at(0), 100);
        }
        app.tick(at(120), 100);
        assert_eq!(app.screen, Screen::Results);
        assert_eq!(app.profile.completed_missions, mission + 1);
        app.tick(at(130), 100);
        assert_eq!(app.profile.sessions as usize, mission + 1);
    }
    assert_eq!(app.profile.level(), 4);
    app.start_campaign();
    assert_eq!(app.session.as_ref().unwrap().mission, Some(14));
}
#[test]
fn failed_and_abandoned_sessions_cannot_unlock_a_level() {
    let mut app = app();
    app.start_campaign();
    app.handle(key(KeyCode::Char('x')), at(0), 0);
    app.tick(at(60), 0);
    assert_eq!(app.profile.completed_missions, 0);
    app.start_campaign();
    app.handle(key(KeyCode::Esc), at(0), 0);
    app.handle(key(KeyCode::Char('q')), at(1), 0);
    assert_eq!(app.profile.sessions, 1);
    assert_eq!(app.screen, Screen::Menu);
    app.go(Screen::Campaign);
    app.selection = 4;
    app.handle(key(KeyCode::Enter), at(1), 0);
    assert_eq!(app.screen, Screen::Campaign);
    app.start(4, None);
    assert_eq!(app.screen, Screen::Playing);
}
#[test]
fn late_enter_cannot_skip_the_result_screen() {
    let mut app = app();
    app.start_campaign();
    app.handle(key(KeyCode::Char('a')), at(0), 0);
    app.handle(key(KeyCode::Enter), at(40), 40);
    assert_eq!(app.screen, Screen::Results);
}
#[test]
fn deterministic_prompts_cover_every_level_and_exact_word_counts() {
    for level in 0..5 {
        let a = LocalAdaptiveProvider::new(19).prompt(level, 100, &BTreeMap::new());
        let b = LocalAdaptiveProvider::new(19).prompt(level, 100, &BTreeMap::new());
        assert_eq!(a, b);
        assert!(a.is_ascii());
        assert_eq!(a.split_whitespace().count(), 100);
        if level == 0 {
            assert!(a.split_whitespace().all(|w| w.len() <= 4));
        }
        if level == 3 {
            assert!(!a.chars().any(|c| c.is_ascii_digit()));
        }
        if level == 4 {
            assert!(a.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
#[test]
fn adaptive_weighting_increases_target_pattern_exposure() {
    let baseline = LocalAdaptiveProvider::new(123).prompt(1, 10000, &BTreeMap::new());
    let errors = BTreeMap::from([("th".into(), 10), ("t".into(), 10), ("h".into(), 10)]);
    let targeted = LocalAdaptiveProvider::new(123).prompt(1, 10000, &errors);
    assert!(targeted.matches("th").count() > baseline.matches("th").count());
    assert!(
        targeted
            .split_whitespace()
            .zip(targeted.split_whitespace().skip(1))
            .all(|(a, b)| a != b)
    );
}
#[test]
fn arbitrary_edit_sequences_keep_metrics_finite_and_indices_safe() {
    let mut s = Session::new("the quick brown fox ".repeat(1000), None, 0, None);
    let mut seed = 77u64;
    for i in 0..20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let now = Duration::from_millis(i * 30);
        if seed.is_multiple_of(3) {
            s.backspace(now);
        } else {
            s.input((b'a' + (seed % 26) as u8) as char, now);
        }
        let m = s.metrics();
        assert!(m.net_wpm.is_finite());
        assert!(m.gross_wpm.is_finite());
        assert!((0.0..=100.0).contains(&m.accuracy));
        assert!(s.typed.len() <= s.prompt.len());
    }
}
