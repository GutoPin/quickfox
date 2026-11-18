use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    Nightfall,
    Orchid,
    Paperwave,
    Ember,
    Nord,
    Dracula,
    Catppuccin,
    TokyoNight,
    Gruvbox,
    RosePine,
    SolarizedLight,
    Forest,
    Custom,
    Mono,
}
impl ThemeName {
    pub const ALL: [Self; 14] = [
        Self::Nightfall,
        Self::Orchid,
        Self::Paperwave,
        Self::Ember,
        Self::Nord,
        Self::Dracula,
        Self::Catppuccin,
        Self::TokyoNight,
        Self::Gruvbox,
        Self::RosePine,
        Self::SolarizedLight,
        Self::Forest,
        Self::Custom,
        Self::Mono,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Nightfall => "Nightfall",
            Self::Orchid => "Orchid Circuit",
            Self::Paperwave => "Paperwave",
            Self::Ember => "Ember",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::Catppuccin => "Catppuccin",
            Self::TokyoNight => "Tokyo Night",
            Self::Gruvbox => "Gruvbox",
            Self::RosePine => "Rose Pine",
            Self::SolarizedLight => "Solarized Light",
            Self::Forest => "Forest",
            Self::Custom => "Custom",
            Self::Mono => "Mono",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ColorPreference {
    #[default]
    Auto,
    TrueColor,
    Ansi256,
    Ansi16,
    Mono,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SoundProfile {
    #[default]
    Creamy,
    Soft,
    Typewriter,
    EgOreo,
    NkCream,
}
impl SoundProfile {
    pub const ALL: [Self; 5] = [
        Self::Creamy,
        Self::Soft,
        Self::Typewriter,
        Self::EgOreo,
        Self::NkCream,
    ];
    pub const SYNTHESIZED: [Self; 3] = [Self::Creamy, Self::Soft, Self::Typewriter];
    pub fn label(self) -> &'static str {
        match self {
            Self::Creamy => "Creamy",
            Self::Soft => "Soft",
            Self::Typewriter => "Typewriter",
            Self::EgOreo => "EG Oreo",
            Self::NkCream => "NK Cream",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SoundSettings {
    pub enabled: bool,
    pub profile: SoundProfile,
    pub volume: u8,
    pub directory: Option<std::path::PathBuf>,
}
pub const MAX_SOUND_VOLUME: u8 = 200;

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: SoundProfile::Creamy,
            volume: 35,
            directory: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Keybindings {
    pub menu: String,
    pub commands: String,
    pub restart: String,
    pub reload_config: String,
}
impl Default for Keybindings {
    fn default() -> Self {
        Self {
            menu: "ctrl+g".into(),
            commands: "ctrl+p".into(),
            restart: "ctrl+r".into(),
            reload_config: "f5".into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutSettings {
    pub sidebar: bool,
    pub prompt_width: u16,
    pub line_spacing: u16,
}
impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            sidebar: false,
            prompt_width: 76,
            line_spacing: 1,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PracticeSettings {
    pub mode: String,
    pub amount: u16,
    pub level: usize,
    pub adaptive: bool,
}
impl Default for PracticeSettings {
    fn default() -> Self {
        Self {
            mode: "time".into(),
            amount: 60,
            level: 2,
            adaptive: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub theme: ThemeName,
    pub color_mode: ColorPreference,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub ascii: bool,
    pub underline_caret: bool,
    pub onboarded: bool,
    pub keybindings: Keybindings,
    pub layout: LayoutSettings,
    pub practice: PracticeSettings,
    pub sound: SoundSettings,
    pub custom_theme: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: None,
            schema_version: 1,
            theme: ThemeName::Nightfall,
            color_mode: ColorPreference::Auto,
            reduced_motion: false,
            high_contrast: false,
            ascii: false,
            underline_caret: false,
            onboarded: false,
            keybindings: Keybindings::default(),
            layout: LayoutSettings::default(),
            practice: PracticeSettings::default(),
            sound: SoundSettings::default(),
            custom_theme: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.sound.volume > MAX_SOUND_VOLUME {
            bail!("sound.volume must be between 0 and {MAX_SOUND_VOLUME}");
        }
        if self.schema_version != 1 {
            bail!("schema_version must be 1; this configuration may require a newer Quickfox");
        }
        if !(32..=100).contains(&self.layout.prompt_width) {
            bail!("layout.prompt_width must be between 32 and 100");
        }
        if self.layout.line_spacing > 2 {
            bail!("layout.line_spacing must be 0, 1, or 2");
        }
        if !(1..=5).contains(&self.practice.level) {
            bail!("practice.level must be between 1 and 5");
        }
        match self.practice.mode.as_str() {
            "time" if [30, 60, 120].contains(&self.practice.amount) => {}
            "words" if [25, 50, 100].contains(&self.practice.amount) => {}
            _ => bail!(
                "practice needs mode=time with amount=30/60/120, or mode=words with amount=25/50/100"
            ),
        }
        let keys = [
            &self.keybindings.menu,
            &self.keybindings.commands,
            &self.keybindings.restart,
            &self.keybindings.reload_config,
        ];
        let mut seen = Vec::new();
        for key in keys {
            let parsed = parse_binding(key)?;
            if seen.contains(&parsed) {
                bail!("Duplicate global shortcut: {key}");
            }
            seen.push(parsed);
        }
        for (name, value) in &self.custom_theme {
            if ![
                "bg",
                "panel",
                "fg",
                "muted",
                "accent",
                "secondary",
                "tertiary",
                "success",
                "error",
                "gold",
                "selection",
                "border",
            ]
            .contains(&name.as_str())
            {
                bail!("Unknown custom_theme token '{name}'");
            }
            if parse_hex(value).is_none() {
                bail!("custom_theme.{name} must be a six-digit hex color such as #82b8ff");
            }
        }
        Ok(())
    }
    pub fn quick_mode(&self) -> usize {
        let choices = if self.practice.mode == "words" {
            [25, 50, 100]
        } else {
            [30, 60, 120]
        };
        choices
            .iter()
            .position(|n| *n == self.practice.amount)
            .unwrap_or(1)
            + if self.practice.mode == "words" { 3 } else { 0 }
    }
}
/// Restrict globals to unambiguous terminal chords that cannot eat typing or standard controls.
pub fn parse_binding(value: &str) -> Result<(KeyCode, KeyModifiers)> {
    let value = value.to_ascii_lowercase();
    if let Some(number) = value.strip_prefix('f').and_then(|s| s.parse::<u8>().ok())
        && (1..=12).contains(&number)
    {
        return Ok((KeyCode::F(number), KeyModifiers::NONE));
    }
    if let Some(letter) = value.strip_prefix("ctrl+")
        && letter.len() == 1
        && letter.as_bytes()[0].is_ascii_lowercase()
        && !["c", "h", "i", "j", "m", "s", "q", "z"].contains(&letter)
    {
        return Ok((
            KeyCode::Char(letter.chars().next().unwrap()),
            KeyModifiers::CONTROL,
        ));
    }
    bail!(
        "Invalid shortcut '{value}'. Use F1-F12 or ctrl+letter; Ctrl+C/H/I/J/M/S/Q/Z are reserved."
    )
}
pub fn binding_matches(value: &str, event: KeyEvent) -> bool {
    parse_binding(value)
        .is_ok_and(|(code, modifiers)| event.code == code && event.modifiers == modifiers)
}
pub fn parse_hex(value: &str) -> Option<(u8, u8, u8)> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value[1..].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    Some((
        u8::from_str_radix(&value[1..3], 16).ok()?,
        u8::from_str_radix(&value[3..5], 16).ok()?,
        u8::from_str_radix(&value[5..7], 16).ok()?,
    ))
}
pub fn read_config(path: &std::path::Path) -> Result<Settings> {
    use anyhow::Context;
    let bytes = std::fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?;
    let settings: Settings = serde_json::from_slice(&bytes)
        .with_context(|| format!("Invalid JSON configuration at {}", path.display()))?;
    settings
        .validate()
        .with_context(|| format!("Invalid configuration at {}", path.display()))?;
    Ok(settings)
}
