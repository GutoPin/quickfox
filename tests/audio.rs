use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use quickfox::{
    app::App,
    audio::{SoundEngine, synthesize},
    config::{Settings, SoundProfile},
    persistence::Profile,
};
use std::time::Duration;

#[test]
fn sounds_are_distinct_bounded_click_free_and_short() {
    let sounds = SoundProfile::SYNTHESIZED.map(synthesize);
    for sound in &sounds {
        assert!((1000..5000).contains(&sound.len()));
        assert_eq!(sound[0], 0.0);
        assert_eq!(*sound.last().unwrap(), 0.0);
        assert!(
            sound
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
        );
        assert!(sound.iter().any(|sample| sample.abs() > 0.02));
    }
    assert_ne!(sounds[0], sounds[1]);
    assert_ne!(sounds[1], sounds[2]);
    let energy = |samples: &[f32]| samples.iter().map(|s| s * s).sum::<f32>();
    assert!(energy(&sounds[1]) < energy(&sounds[0]));
}
#[test]
fn audio_is_off_by_default_and_legacy_settings_stay_silent() {
    let settings: Settings = serde_json::from_str("{}").unwrap();
    assert!(!settings.sound.enabled);
    let mut engine = SoundEngine::default();
    engine.configure(&settings.sound);
    engine.play();
    assert_eq!(engine.status(), "Sound is off");
    assert!(!engine.is_starting());
    assert!(
        serde_json::from_str::<Settings>(r#"{"sound":{"volume":200}}"#)
            .unwrap()
            .validate()
            .is_ok()
    );
    assert!(
        serde_json::from_str::<Settings>(r#"{"sound":{"volume":201}}"#)
            .unwrap()
            .validate()
            .is_err()
    );
    assert!(serde_json::from_str::<Settings>(r#"{"sound":{"profile":"unknown"}}"#).is_err());
}
#[test]
fn only_accepted_typing_and_corrections_request_a_key_sound() {
    let mut app = App::new(Settings::default(), Profile::default(), 42);
    app.start_campaign();
    let press = |code| KeyEvent::new(code, KeyModifiers::NONE);
    for (key, expected) in [
        (press(KeyCode::Backspace), false),
        (press(KeyCode::Char('a')), true),
        (
            KeyEvent::new_with_kind(KeyCode::Char('a'), KeyModifiers::NONE, KeyEventKind::Repeat),
            false,
        ),
        (
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            false,
        ),
        (press(KeyCode::Backspace), true),
        (press(KeyCode::Tab), false),
        (press(KeyCode::Down), false),
    ] {
        app.handle(key, Duration::ZERO, 0);
        assert_eq!(app.sound_requested, expected, "{key:?}");
    }
}

#[test]
fn recorded_wav_is_downmixed_resampled_and_faded() {
    let dir = tempfile::tempdir().unwrap();
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 24_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(dir.path().join("nk-cream.wav"), spec).unwrap();
    for _ in 0..4800 {
        writer.write_sample(16_384_i16).unwrap();
        writer.write_sample(0_i16).unwrap();
    }
    writer.finalize().unwrap();
    let sound = quickfox::audio::load_sound(SoundProfile::NkCream, Some(dir.path())).unwrap();
    assert_eq!(sound.len(), 8880);
    assert_eq!(sound[0], 0.0);
    assert_eq!(*sound.last().unwrap(), 0.0);
    assert!((sound[100] - 0.25).abs() < 0.001);
}

#[test]
fn absent_corrupt_and_incomplete_recordings_fail_without_a_device() {
    let dir = tempfile::tempdir().unwrap();
    for profile in [SoundProfile::EgOreo, SoundProfile::NkCream] {
        assert!(quickfox::audio::load_sound(profile, Some(dir.path())).is_err());
    }
    for filename in ["oreo.ogg", "nk-cream.wav"] {
        std::fs::write(dir.path().join(filename), b"invalid audio").unwrap();
    }
    for profile in [SoundProfile::EgOreo, SoundProfile::NkCream] {
        assert!(quickfox::audio::load_sound(profile, Some(dir.path())).is_err());
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(dir.path().join("nk-cream.wav"), spec).unwrap();
    writer.write_sample(0.5_f32).unwrap();
    writer.finalize().unwrap();
    assert!(quickfox::audio::load_sound(SoundProfile::NkCream, Some(dir.path())).is_err());
    let mut writer = hound::WavWriter::create(dir.path().join("nk-cream.wav"), spec).unwrap();
    for _ in 0..8880 {
        writer.write_sample(f32::NAN).unwrap();
    }
    writer.finalize().unwrap();
    assert!(quickfox::audio::load_sound(SoundProfile::NkCream, Some(dir.path())).is_err());
}
