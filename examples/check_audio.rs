//! Optional audible smoke check. Never run by cargo test; requires system audio.
use quickfox::{
    audio::SoundEngine,
    config::{SoundProfile, SoundSettings},
};
use std::time::{Duration, Instant};
fn main() -> anyhow::Result<()> {
    let mut engine = SoundEngine::default();
    let mut settings = SoundSettings {
        enabled: true,
        ..Default::default()
    };
    engine.configure(&settings);
    let start = Instant::now();
    while engine.is_starting() && start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(10));
    }
    anyhow::ensure!(
        engine.status() == "System audio ready",
        "{}",
        engine.status()
    );
    for profile in SoundProfile::ALL {
        settings.profile = profile;
        engine.configure(&settings);
        let start = Instant::now();
        while engine.is_starting() && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        engine.preview();
        std::thread::sleep(Duration::from_millis(250));
        anyhow::ensure!(
            engine.status() == "System audio ready",
            "{}",
            engine.status()
        );
        println!("{}: submitted to system audio", profile.label());
    }
    settings.enabled = false;
    engine.configure(&settings);
    anyhow::ensure!(engine.status() == "Sound is off", "Mute did not apply");
    Ok(())
}
