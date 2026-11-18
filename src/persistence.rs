use crate::game::{MINIMUM_CHARACTERS, Metrics, TARGET_ACCURACY};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub use crate::config::{Settings, ThemeName};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: u64,
    #[serde(default)]
    pub daily_day: Option<u64>,
    #[serde(default)]
    pub focused: bool,
    pub level: usize,
    pub campaign: bool,
    pub passed: bool,
    pub xp: u64,
    pub metrics: Metrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub schema_version: u32,
    pub xp: u64,
    pub completed_missions: usize,
    pub sessions: u64,
    pub best_wpm: f64,
    pub errors: BTreeMap<String, u32>,
    pub history: Vec<Record>,
    pub daily_best: BTreeMap<u64, f64>,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            xp: 0,
            completed_missions: 0,
            sessions: 0,
            best_wpm: 0.0,
            errors: BTreeMap::new(),
            history: Vec::new(),
            daily_best: BTreeMap::new(),
        }
    }
}
impl Profile {
    pub fn level(&self) -> usize {
        (self.completed_missions / 3).min(4)
    }
    pub fn record(&mut self, session: &crate::game::Session, timestamp: u64) -> Record {
        let metrics = session.metrics();
        let passed = session.finished
            && metrics.accuracy >= TARGET_ACCURACY
            && metrics.correct >= MINIMUM_CHARACTERS;
        if passed
            && session.mission == Some(self.completed_missions)
            && self.completed_missions < 15
        {
            self.completed_missions += 1;
        }
        let xp = if metrics.correct == 0 {
            0
        } else {
            (metrics.correct as u64 / 2).min(300) + if passed { 50 } else { 10 }
        };
        self.xp = self.xp.saturating_add(xp);
        self.sessions = self.sessions.saturating_add(1);
        self.best_wpm = self.best_wpm.max(metrics.net_wpm);
        for count in self.errors.values_mut() {
            *count = (*count * 3) / 4;
        }
        for (pattern, count) in &session.errors {
            let value = self.errors.entry(pattern.to_ascii_lowercase()).or_default();
            *value = value.saturating_add(*count).min(1000);
        }
        self.errors.retain(|_, count| *count > 0);
        if let Some(day) = session.daily_day {
            let best = self.daily_best.entry(day).or_default();
            *best = best.max(metrics.net_wpm);
            while self.daily_best.len() > 366 {
                self.daily_best.pop_first();
            }
        }
        let record = Record {
            timestamp,
            daily_day: session.daily_day,
            focused: session.focused,
            level: session.level,
            campaign: session.mission.is_some(),
            passed,
            xp,
            metrics,
        };
        self.history.push(record.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        record
    }
}

/// Standard per-user config and data directories.
pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "quickfox")
}
pub struct Storage {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    config_revision: std::cell::RefCell<Option<Vec<u8>>>,
    _lock: File,
}
impl Storage {
    pub fn open(root: Option<&Path>) -> Result<Self> {
        let (config_dir, data_dir) = if let Some(root) = root {
            (root.join("config"), root.join("data"))
        } else {
            let dirs = project_dirs()
                .context("Cannot locate your home directory. Use --data-dir PATH.")?;
            (dirs.config_dir().to_owned(), dirs.data_dir().to_owned())
        };
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&data_dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data_dir.join(".lock"))?;
        lock.try_lock_exclusive().context(
            "Another Quickfox process is using this profile. Close it or use --data-dir PATH.",
        )?;
        Ok(Self {
            config_revision: std::cell::RefCell::new(None),
            config_path: config_dir.join("config.json"),
            config_dir,
            data_dir,
            _lock: lock,
        })
    }
    pub fn load(&self) -> Result<(Settings, Profile, Vec<String>)> {
        let mut warnings = Vec::new();
        let settings = if self.config_path.exists() {
            self.reload_settings()?
        } else {
            Settings::default()
        };
        settings.validate()?;
        let mut profile: Profile =
            read_or_recover(&self.data_dir.join("progress.json"), &mut warnings)?;
        if settings.schema_version != 1 || profile.schema_version != 1 {
            bail!(
                "This save uses an unsupported schema. Your files are unchanged; use a compatible Quickfox version or a separate --data-dir."
            );
        }
        if profile.completed_missions > 15
            || !profile.best_wpm.is_finite()
            || profile.best_wpm < 0.0
            || profile.errors.len() > 10000
            || profile
                .errors
                .keys()
                .any(|key| key.is_empty() || key.len() > 2 || !key.is_ascii())
        {
            bail!(
                "Progress contains invalid values. Your file is unchanged. Inspect progress.json or use a separate --data-dir."
            );
        }
        for count in profile.errors.values_mut() {
            *count = (*count).min(1000);
        }
        if profile.history.len() > 100 {
            profile.history.drain(..profile.history.len() - 100);
        }
        Ok((settings, profile, warnings))
    }
    pub fn reload_settings(&self) -> Result<Settings> {
        let bytes = fs::read(&self.config_path)
            .with_context(|| format!("Cannot read {}", self.config_path.display()))?;
        let settings: Settings =
            serde_json::from_slice(&bytes).context("Invalid configuration JSON")?;
        settings.validate()?;
        *self.config_revision.borrow_mut() = Some(bytes);
        Ok(settings)
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        settings.validate()?;
        let current = match fs::read(&self.config_path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if current != *self.config_revision.borrow() {
            bail!("Configuration changed on disk. Reload JSON before saving settings.");
        }
        atomic_json(&self.config_path, settings)?;
        *self.config_revision.borrow_mut() = Some(fs::read(&self.config_path)?);
        Ok(())
    }
    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        atomic_json(&self.data_dir.join("progress.json"), profile)
    }
}
fn read_or_recover<T: serde::de::DeserializeOwned + Default>(
    path: &Path,
    warnings: &mut Vec<String>,
) -> Result<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => return Err(error).with_context(|| format!("Cannot read {}", path.display())),
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => Ok(value),
        Err(_) => {
            let backup = tempfile::Builder::new()
                .prefix("recovered-")
                .tempfile_in(path.parent().unwrap())?;
            let (_, backup_path) = backup.keep()?;
            fs::rename(path, &backup_path)
                .context("Cannot preserve the damaged save; refusing to overwrite it")?;
            warnings.push(format!(
                "Damaged save preserved at {}. A fresh profile is ready.",
                backup_path.display()
            ));
            Ok(T::default())
        }
    }
}
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("Save path has no parent")?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, value)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .with_context(|| format!("Cannot save {}", path.display()))?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}
