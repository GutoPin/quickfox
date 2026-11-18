use std::process::Command;
fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quickfox"))
}
#[test]
fn help_version_and_doctor_work_without_a_terminal() {
    for args in [vec!["--help"], vec!["--version"], vec!["doctor"]] {
        let output = binary().args(args).output().unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}
#[test]
fn noninteractive_start_has_an_actionable_error() {
    let output = binary().output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("interactive terminal"));
}
#[test]
fn invalid_modes_are_rejected() {
    for args in [
        vec!["play", "--seconds", "5"],
        vec!["play", "--level", "6"],
        vec!["play", "--seconds", "30", "--words", "25"],
    ] {
        assert!(!binary().args(args).output().unwrap().status.success());
    }
}
#[test]
fn export_is_parseable_and_diagnostics_do_not_create_directories() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("profile");
    let output = binary()
        .arg("--data-dir")
        .arg(&data)
        .arg("doctor")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!data.exists());
    let output = binary()
        .arg("--data-dir")
        .arg(&data)
        .arg("export")
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["sessions"], 0);
}

#[test]
fn config_commands_create_validate_and_refuse_to_overwrite() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("personal.json");
    let run = |action: &str| {
        binary()
            .arg("--config")
            .arg(&path)
            .args(["config", action])
            .output()
            .unwrap()
    };
    assert!(run("path").status.success());
    assert!(!path.exists());
    assert!(run("init").status.success());
    assert!(root.path().join("quickfox.schema.json").exists());
    assert!(run("validate").status.success());
    let original = std::fs::read(&path).unwrap();
    assert!(!run("init").status.success());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::write(&path, r#"{"layout":{"prompt_width":5}}"#).unwrap();
    let output = run("validate");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("prompt_width"));
}
#[test]
fn config_show_expands_defaults_without_creating_a_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.json");
    let output = binary()
        .arg("--config")
        .arg(&path)
        .args(["config", "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!path.exists());
    let settings: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(settings["keybindings"]["menu"], "ctrl+g");
}
