use quickfox::{
    config::{Settings, ThemeName, parse_binding, read_config},
    persistence::Storage,
    ui::theme::{ColorMode, Theme},
};
use ratatui::style::Color;
use std::fs;
#[test]
fn old_preferences_gain_new_defaults_without_losing_values() {
    let settings: Settings = serde_json::from_str(
        r#"{"schema_version":1,"theme":"paperwave","ascii":true,"onboarded":true}"#,
    )
    .unwrap();
    settings.validate().unwrap();
    assert_eq!(settings.theme, ThemeName::Paperwave);
    assert!(settings.ascii);
    assert!(settings.onboarded);
    assert_eq!(settings.keybindings.menu, "ctrl+g");
}
#[test]
fn example_and_roundtrip_are_valid() {
    let settings: Settings = serde_json::from_str(include_str!("../config/example.json")).unwrap();
    settings.validate().unwrap();
    let other: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(other.custom_theme, settings.custom_theme);
    let t = Theme::new(&settings, ColorMode::TrueColor);
    assert_eq!(t.secondary, Color::Rgb(163, 163, 163));
}
#[test]
fn invalid_colors_ranges_unknown_fields_and_conflicting_bindings_are_rejected() {
    for input in [
        r##"{"custom_theme":{"accent":"#oops"}}"##,
        r#"{"keybindings":{"menu":"q"}}"#,
        r#"{"keybindings":{"menu":"ctrl+p"}}"#,
        r#"{"layout":{"line_spacing":5}}"#,
        r#"{"practice":{"level":0}}"#,
        r#"{"practice":{"mode":"words","amount":60}}"#,
        r#"{"layout":{"prompt_width":500}}"#,
    ] {
        let settings: Settings = serde_json::from_str(input).unwrap();
        assert!(settings.validate().is_err(), "{input}");
    }
    assert!(serde_json::from_str::<Settings>(r#"{"layuot":{}}"#).is_err());
    assert!(serde_json::from_str::<Settings>(r#"{"layout":{"sidebr":true}}"#).is_err());
    for key in ["ctrl+c", "ctrl+h", "ctrl+m", "ctrl+i", "f13"] {
        assert!(parse_binding(key).is_err());
    }
}
#[test]
fn configuration_errors_never_rename_or_replace_the_users_file() {
    let root = tempfile::tempdir().unwrap();
    let store = Storage::open(Some(root.path())).unwrap();
    for text in [
        "{unfinished edit",
        r#"{"theme":"unknown"}"#,
        r#"{"schema_version":2}"#,
    ] {
        fs::write(&store.config_path, text).unwrap();
        assert!(store.load().is_err());
        assert_eq!(fs::read_to_string(&store.config_path).unwrap(), text);
    }
}
#[test]
fn reload_validates_atomically_and_external_edits_are_not_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let store = Storage::open(Some(root.path())).unwrap();
    store.save_settings(&Settings::default()).unwrap();
    fs::write(&store.config_path, r#"{"theme":"ember"}"#).unwrap();
    assert!(store.save_settings(&Settings::default()).is_err());
    let settings = store.reload_settings().unwrap();
    assert_eq!(settings.theme, ThemeName::Ember);
    store.save_settings(&settings).unwrap();
    fs::write(&store.config_path, "{unfinished").unwrap();
    assert!(store.reload_settings().is_err());
    assert!(store.save_settings(&settings).is_err());
    assert_eq!(
        fs::read_to_string(&store.config_path).unwrap(),
        "{unfinished"
    );
}
#[test]
fn custom_colors_respect_monochrome_and_partial_overrides() {
    let mut settings = Settings {
        theme: ThemeName::Custom,
        ..Settings::default()
    };
    settings
        .custom_theme
        .insert("accent".into(), "#123456".into());
    settings.validate().unwrap();
    assert_eq!(
        Theme::new(&settings, ColorMode::TrueColor).accent,
        Color::Rgb(18, 52, 86)
    );
    assert_eq!(Theme::new(&settings, ColorMode::Mono).accent, Color::Reset);
    assert_ne!(Theme::new(&settings, ColorMode::TrueColor).bg, Color::Reset);
}
#[test]
fn read_config_rejects_invalid_json_with_a_path() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("personal.json");
    fs::write(&path, "bad json").unwrap();
    assert!(
        read_config(&path)
            .unwrap_err()
            .to_string()
            .contains("personal.json")
    );
}
