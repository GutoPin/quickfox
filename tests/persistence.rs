use quickfox::persistence::{Profile, Settings, Storage, ThemeName};
use std::fs;
#[test]
fn preferences_and_progress_survive_reopening() {
    let root = tempfile::tempdir().unwrap();
    {
        let store = Storage::open(Some(root.path())).unwrap();
        store
            .save_settings(&Settings {
                theme: ThemeName::Paperwave,
                ascii: true,
                ..Settings::default()
            })
            .unwrap();
        store
            .save_profile(&Profile {
                xp: 120,
                completed_missions: 3,
                ..Profile::default()
            })
            .unwrap();
    }
    let store = Storage::open(Some(root.path())).unwrap();
    let (settings, profile, warnings) = store.load().unwrap();
    assert_eq!(settings.theme, ThemeName::Paperwave);
    assert!(settings.ascii);
    assert_eq!(profile.xp, 120);
    assert_eq!(profile.completed_missions, 3);
    assert!(warnings.is_empty());
}
#[test]
fn damaged_json_is_preserved_before_recovery() {
    let root = tempfile::tempdir().unwrap();
    let store = Storage::open(Some(root.path())).unwrap();
    fs::write(store.data_dir.join("progress.json"), "{damaged save").unwrap();
    let (_, profile, warnings) = store.load().unwrap();
    assert_eq!(profile.xp, 0);
    assert_eq!(warnings.len(), 1);
    let backup = fs::read_dir(&store.data_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("recovered-")
        })
        .unwrap();
    assert_eq!(fs::read_to_string(backup).unwrap(), "{damaged save");
    store.save_profile(&profile).unwrap();
    assert!(store.load().unwrap().2.is_empty());
}
#[test]
fn future_schema_and_invalid_progress_are_never_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let store = Storage::open(Some(root.path())).unwrap();
    for contents in [r#"{"schema_version":99}"#, r#"{"completed_missions":999}"#] {
        let path = store.data_dir.join("progress.json");
        fs::write(&path, contents).unwrap();
        assert!(store.load().is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }
}
#[test]
fn simultaneous_writers_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let first = Storage::open(Some(root.path())).unwrap();
    assert!(Storage::open(Some(root.path())).is_err());
    drop(first);
    assert!(Storage::open(Some(root.path())).is_ok());
}
#[test]
fn failed_atomic_write_keeps_existing_destination_intact() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("progress.json");
    fs::create_dir(&path).unwrap();
    assert!(quickfox::persistence::atomic_json(&path, &Profile::default()).is_err());
    assert!(path.is_dir());
}
