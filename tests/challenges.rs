use quickfox::{
    app::App,
    content::{daily_prompt, focus_patterns, focused_prompt},
    game::Session,
    persistence::{Profile, Settings},
};
use std::{collections::BTreeMap, time::Duration};
#[test]
fn daily_prompt_is_reproducible_changes_by_day_and_ignores_profile_errors() {
    assert_eq!(daily_prompt(20000), daily_prompt(20000));
    assert_ne!(daily_prompt(20000), daily_prompt(20001));
    let mut profile = Profile::default();
    profile.errors.insert("th".into(), 1000);
    let mut a = App::new(Settings::default(), profile, 42);
    a.start_daily(20000);
    assert_eq!(
        a.session
            .as_ref()
            .unwrap()
            .prompt
            .iter()
            .collect::<String>(),
        daily_prompt(20000)
    );
    assert_eq!(
        a.session.as_ref().unwrap().duration,
        Some(Duration::from_secs(60))
    );
}
#[test]
fn daily_best_persists_separately_and_never_unlocks_campaign() {
    let mut profile = Profile::default();
    let mut session = Session::new("a".repeat(30), Some(60), 2, None);
    session.daily_day = Some(20000);
    for i in 0..30 {
        session.input('a', Duration::from_secs(i));
    }
    profile.record(&session, 20000 * 86400);
    assert_eq!(profile.completed_missions, 0);
    assert!(profile.daily_best[&20000] > 0.0);
    let saved = serde_json::to_string(&profile).unwrap();
    let loaded: Profile = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.daily_best, profile.daily_best);
    let best = profile.daily_best[&20000];
    let mut slow = Session::new("aaaa".into(), Some(60), 2, None);
    slow.daily_day = Some(20000);
    slow.input('a', Duration::ZERO);
    slow.tick(Duration::from_secs(60));
    profile.record(&slow, 0);
    assert_eq!(profile.daily_best[&20000], best);
}
#[test]
fn focused_words_contain_the_selected_pattern_and_defaults_work() {
    for pattern in ["th", "ou", "x"] {
        let prompt = focused_prompt(pattern, 42);
        assert_eq!(prompt.split_whitespace().count(), 50);
        assert!(prompt.split_whitespace().all(|w| w.contains(pattern)));
    }
    assert_eq!(focus_patterns(&BTreeMap::new())[0].0, "th");
    let patterns = focus_patterns(&BTreeMap::from([
        ("th".into(), 8),
        ("a".into(), 2),
        (" ".into(), 50),
    ]));
    assert_eq!(patterns[0].0, "th");
    assert_eq!(patterns.len(), 2);
}
#[test]
fn legacy_profile_history_loads_with_new_optional_fields() {
    let legacy = r#"{"xp":10,"completed_missions":1,"history":[{"timestamp":1,"level":0,"campaign":true,"passed":true,"xp":10,"metrics":{"net_wpm":10.0,"gross_wpm":10.0,"accuracy":100.0,"consistency":0.0,"seconds":30.0,"correct":25,"attempts":25,"corrections":0}}]}"#;
    let profile: Profile = serde_json::from_str(legacy).unwrap();
    assert_eq!(profile.completed_missions, 1);
    assert!(profile.daily_best.is_empty());
    assert_eq!(profile.history[0].daily_day, None);
    assert!(!profile.history[0].focused);
}
