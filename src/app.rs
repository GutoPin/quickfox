use crate::{
    config::{MAX_SOUND_VOLUME, SoundProfile, binding_matches},
    content::{
        LocalAdaptiveProvider, PromptProvider, daily_prompt, focus_patterns, focused_prompt,
    },
    game::{MISSION_SECONDS, Session},
    persistence::{Profile, Record, Settings, ThemeName},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Menu,
    Onboarding,
    Campaign,
    QuickPlay,
    Focused,
    Daily,
    Playing,
    Paused,
    Results,
    Statistics,
    Settings,
    Help,
}
impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Menu => "Menu",
            Self::Onboarding => "Welcome",
            Self::Campaign => "Campaign",
            Self::QuickPlay => "Quick play",
            Self::Focused => "Focused practice",
            Self::Daily => "Daily challenge",
            Self::Playing => "Typing",
            Self::Paused => "Paused",
            Self::Results => "Results",
            Self::Statistics => "Statistics",
            Self::Settings => "Settings",
            Self::Help => "Help",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickMode {
    Seconds(u64),
    Words(usize),
}
impl QuickMode {
    pub const ALL: [Self; 6] = [
        Self::Seconds(30),
        Self::Seconds(60),
        Self::Seconds(120),
        Self::Words(25),
        Self::Words(50),
        Self::Words(100),
    ];
    pub fn label(self) -> String {
        match self {
            Self::Seconds(n) => format!("{n}s"),
            Self::Words(n) => format!("{n} words"),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum SessionRequest {
    Standard(usize, Option<usize>),
    Daily(u64),
    Focused(usize),
    Retry,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Home,
    Menu,
    Resume,
    Campaign,
    QuickPlay,
    Focused,
    Daily,
    Statistics,
    Settings,
    Help,
    Reload,
    Restart,
    Quit,
}
impl Action {
    pub const ALL: [Self; 13] = [
        Self::Home,
        Self::Menu,
        Self::Resume,
        Self::Campaign,
        Self::QuickPlay,
        Self::Focused,
        Self::Daily,
        Self::Statistics,
        Self::Settings,
        Self::Help,
        Self::Reload,
        Self::Restart,
        Self::Quit,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Go to home",
            Self::Menu => "Open navigation menu",
            Self::Resume => "Resume paused session",
            Self::Campaign => "Open campaign",
            Self::QuickPlay => "Open quick play",
            Self::Focused => "Practice difficult keys",
            Self::Daily => "Open daily challenge",
            Self::Statistics => "View statistics",
            Self::Settings => "Change theme and settings",
            Self::Help => "Show help and shortcuts",
            Self::Reload => "Reload JSON configuration",
            Self::Restart => "Restart current session",
            Self::Quit => "Quit Quickfox",
        }
    }
}
pub const SETTINGS_SECTIONS: [(&str, usize, usize); 4] = [
    ("Appearance", 0, 6),
    ("Sound", 6, 10),
    ("Practice", 10, 11),
    ("Config", 11, 12),
];
pub fn settings_section(selection: usize) -> usize {
    SETTINGS_SECTIONS
        .iter()
        .position(|(_, start, end)| (*start..*end).contains(&selection))
        .unwrap_or(0)
}

pub struct CommandPalette {
    pub query: String,
    pub selection: usize,
    previous: Screen,
    resume_on_close: bool,
}
pub struct App {
    pub screen: Screen,
    pub selection: usize,
    pub settings: Settings,
    pub profile: Profile,
    pub session: Option<Session>,
    pub result: Option<Record>,
    pub previous_best: f64,
    pub quick_mode: usize,
    pub quick_level: usize,
    pub notice: Option<String>,
    pub quit: bool,
    pub settings_dirty: bool,
    pub profile_dirty: bool,
    pub reload_requested: bool,
    pub result_at: Duration,
    pub palette: Option<CommandPalette>,
    pub pending: Option<SessionRequest>,
    pub today: u64,
    pub history_offset: usize,
    pub config_path: String,
    pub focus_pattern: String,
    pub sound_requested: bool,
    pub audio_status: &'static str,
    provider: LocalAdaptiveProvider,
}
impl App {
    pub fn new(settings: Settings, profile: Profile, seed: u64) -> Self {
        Self {
            screen: Screen::Home,
            selection: 0,
            quick_mode: settings.quick_mode(),
            quick_level: settings.practice.level.saturating_sub(1).min(4),
            settings,
            profile,
            session: None,
            result: None,
            previous_best: 0.0,
            notice: None,
            quit: false,
            settings_dirty: false,
            profile_dirty: false,
            reload_requested: false,
            result_at: Duration::ZERO,
            palette: None,
            pending: None,
            today: seed / 86400,
            history_offset: 0,
            config_path: "config.json".into(),
            focus_pattern: String::new(),
            sound_requested: false,
            audio_status: "Sound is off",
            provider: LocalAdaptiveProvider::new(seed),
        }
    }
    pub fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.selection = 0;
    }
    pub fn has_session(&self) -> bool {
        self.session.as_ref().is_some_and(|s| !s.finished)
    }
    pub fn home(&mut self, now: Duration) {
        self.pause(now);
        self.go(Screen::Home);
    }
    pub fn menu(&mut self, now: Duration) {
        self.pause(now);
        self.go(Screen::Menu);
    }
    pub fn resume(&mut self, now: Duration) {
        if self.has_session() {
            self.session.as_mut().unwrap().resume(now);
            self.go(Screen::Playing);
        } else {
            self.notice = Some("No paused session. Start a new practice from Home.".into());
        }
    }
    pub fn apply_settings(&mut self, settings: Settings) {
        self.quick_mode = settings.quick_mode();
        self.quick_level = settings.practice.level - 1;
        self.settings = settings;
    }
    fn start_guided_practice(&mut self) {
        if !self.settings.onboarded {
            self.settings.onboarded = true;
            self.settings_dirty = true;
        }
        self.start_campaign();
    }
    pub fn start_campaign(&mut self) {
        let mission = self.profile.completed_missions.min(14);
        self.start(mission / 3, Some(mission));
    }
    pub fn start(&mut self, level: usize, mission: Option<usize>) {
        let (seconds, words) = match mission {
            Some(i) => (Some(MISSION_SECONDS[i % 3]), 600),
            None => match QuickMode::ALL[self.quick_mode] {
                QuickMode::Seconds(n) => (Some(n), 600),
                QuickMode::Words(n) => (None, n),
            },
        };
        let errors = if self.settings.practice.adaptive {
            self.profile.errors.clone()
        } else {
            Default::default()
        };
        let prompt = self.provider.prompt(level, words, &errors);
        self.session = Some(Session::new(prompt, seconds, level, mission));
        self.result = None;
        self.go(Screen::Playing);
    }
    pub fn start_daily(&mut self, day: u64) {
        let mut session = Session::new(daily_prompt(day), Some(60), 2, None);
        session.daily_day = Some(day);
        self.session = Some(session);
        self.result = None;
        self.go(Screen::Playing);
    }
    pub fn start_focused(&mut self, index: usize) {
        let patterns = focus_patterns(&self.profile.errors);
        self.focus_pattern = patterns[index.min(patterns.len() - 1)].0.clone();
        let mut session = Session::new(
            focused_prompt(&self.focus_pattern, self.profile.sessions.wrapping_add(42)),
            None,
            1,
            None,
        );
        session.focused = true;
        self.session = Some(session);
        self.result = None;
        self.go(Screen::Playing);
    }
    fn request_start(&mut self, request: SessionRequest, now: Duration) {
        if self.has_session() {
            self.pause(now);
            self.pending = Some(request);
        } else {
            self.begin(request);
        }
    }
    fn begin(&mut self, request: SessionRequest) {
        match request {
            SessionRequest::Standard(level, mission) => self.start(level, mission),
            SessionRequest::Daily(day) => self.start_daily(day),
            SessionRequest::Focused(index) => self.start_focused(index),
            SessionRequest::Retry => self.restart_session(),
        }
    }
    pub fn tick(&mut self, now: Duration, timestamp: u64) {
        self.today = timestamp / 86400;
        if self.screen == Screen::Playing
            && let Some(session) = self.session.as_mut()
        {
            session.tick(now);
            if session.finished {
                self.previous_best = self.profile.best_wpm;
                self.result = Some(self.profile.record(session, timestamp));
                self.profile_dirty = true;
                self.result_at = now;
                self.go(Screen::Results);
            }
        }
    }
    pub fn pause(&mut self, now: Duration) {
        if self.screen == Screen::Playing
            && let Some(session) = self.session.as_mut()
        {
            session.pause(now);
            if !session.finished {
                self.go(Screen::Paused);
            }
        }
    }
    pub fn needs_tick(&self, now: Duration) -> bool {
        (self.screen == Screen::Playing && self.session.as_ref().is_some_and(Session::has_started))
            || (self.screen == Screen::Results
                && !self.settings.reduced_motion
                && now.saturating_sub(self.result_at) < Duration::from_millis(500))
    }
    pub fn commands(&self) -> Vec<Action> {
        let query = self
            .palette
            .as_ref()
            .map(|p| p.query.to_ascii_lowercase())
            .unwrap_or_default();
        Action::ALL
            .into_iter()
            .filter(|action| {
                (!matches!(action, Action::Resume | Action::Restart) || self.has_session())
                    && action.label().to_ascii_lowercase().contains(&query)
            })
            .collect()
    }
    fn open_palette(&mut self, now: Duration) {
        let previous = self.screen;
        let resume_on_close = previous == Screen::Playing;
        self.pause(now);
        self.palette = Some(CommandPalette {
            query: String::new(),
            selection: 0,
            previous,
            resume_on_close,
        });
    }
    fn close_palette(&mut self, now: Duration) {
        if let Some(palette) = self.palette.take() {
            self.screen = palette.previous;
            if palette.resume_on_close {
                self.resume(now);
            }
        }
    }
    pub fn execute(&mut self, action: Action, now: Duration) {
        match action {
            Action::Home => self.home(now),
            Action::Menu => self.menu(now),
            Action::Resume => self.resume(now),
            Action::Campaign => self.go(Screen::Campaign),
            Action::QuickPlay => self.go(Screen::QuickPlay),
            Action::Focused => self.go(Screen::Focused),
            Action::Daily => self.go(Screen::Daily),
            Action::Statistics => {
                self.history_offset = 0;
                self.go(Screen::Statistics)
            }
            Action::Settings => self.go(Screen::Settings),
            Action::Help => self.go(Screen::Help),
            Action::Reload => {
                self.pause(now);
                self.reload_requested = true;
            }
            Action::Restart => self.retry(now),
            Action::Quit => self.quit = true,
        }
    }
    pub fn handle(&mut self, key: KeyEvent, now: Duration, timestamp: u64) {
        self.sound_requested = false;
        if key.kind != KeyEventKind::Press {
            return;
        }
        self.today = timestamp / 86400;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.screen == Screen::Playing {
            self.tick(now, timestamp);
            if self.screen != Screen::Playing {
                return;
            }
        }
        if binding_matches(&self.settings.keybindings.menu, key)
            || (key.code == KeyCode::Tab && key.modifiers.is_empty())
        {
            self.pending = None;
            self.palette = None;
            self.menu(now);
            return;
        }
        if self.palette.is_some() && binding_matches(&self.settings.keybindings.commands, key) {
            self.close_palette(now);
            return;
        }
        if self.pending.is_some() {
            match key.code {
                KeyCode::Enter => {
                    let request = self.pending.take().unwrap();
                    self.begin(request);
                }
                KeyCode::Esc => {
                    self.pending = None;
                }
                _ => {}
            }
            return;
        }
        if binding_matches(&self.settings.keybindings.reload_config, key) {
            self.palette = None;
            self.pause(now);
            self.reload_requested = true;
            return;
        }
        if binding_matches(&self.settings.keybindings.restart, key) {
            self.palette = None;
            self.retry(now);
            return;
        }
        if binding_matches(&self.settings.keybindings.commands, key) {
            self.open_palette(now);
            return;
        }
        if self.palette.is_some() {
            let count = self.commands().len();
            match key.code {
                KeyCode::Esc => self.close_palette(now),
                KeyCode::Enter => {
                    let selected = self.palette.as_ref().unwrap().selection;
                    if let Some(action) = self.commands().get(selected).copied() {
                        self.palette = None;
                        self.execute(action, now);
                    }
                }
                KeyCode::Up => {
                    let p = self.palette.as_mut().unwrap();
                    p.selection = p.selection.saturating_sub(1);
                }
                KeyCode::Down => {
                    let p = self.palette.as_mut().unwrap();
                    p.selection = (p.selection + 1).min(count.saturating_sub(1));
                }
                KeyCode::Backspace => {
                    let p = self.palette.as_mut().unwrap();
                    p.query.pop();
                    p.selection = 0;
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) && (c.is_ascii_graphic() || c == ' ') =>
                {
                    let p = self.palette.as_mut().unwrap();
                    if p.query.len() < 48
                        && let KeyCode::Char(c) = key.code
                    {
                        p.query.push(c);
                    }
                    p.selection = 0;
                }
                _ => {}
            }
            return;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return;
        }
        match self.screen {
            Screen::Playing => match key.code {
                KeyCode::Esc => self.pause(now),
                KeyCode::Backspace => {
                    if let Some(s) = self.session.as_mut() {
                        let previous = s.typed.len();
                        s.backspace(now);
                        self.sound_requested = s.typed.len() != previous;
                    }
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    if let Some(s) = self.session.as_mut() {
                        let previous = s.typed.len();
                        s.input(c, now);
                        self.sound_requested = s.typed.len() != previous;
                    }
                }
                _ => {}
            },
            Screen::Paused => match key.code {
                KeyCode::Enter => self.resume(now),
                KeyCode::Esc | KeyCode::Char('m') => self.menu(now),
                KeyCode::Char('r') => self.retry(now),
                KeyCode::Char('q') => {
                    self.session = None;
                    self.go(Screen::Menu)
                }
                _ => {}
            },
            Screen::Home => match key.code {
                KeyCode::Enter if self.has_session() => self.resume(now),
                KeyCode::Enter => self.start_guided_practice(),
                KeyCode::Char('q') => self.quit = true,
                KeyCode::Char('?') => self.go(Screen::Help),
                _ => {}
            },
            Screen::Menu => match key.code {
                KeyCode::Right if self.selection < 5 => self.selection = 5 + self.selection.min(2),
                KeyCode::Left if self.selection >= 5 => self.selection -= 5,
                KeyCode::Esc => self.home(now),
                KeyCode::Up | KeyCode::Char('k') => self.selection = (self.selection + 7) % 8,
                KeyCode::Down | KeyCode::Char('j') => self.selection = (self.selection + 1) % 8,
                KeyCode::Enter => match self.selection {
                    0 if self.has_session() => self.resume(now),
                    0 if !self.settings.onboarded => self.go(Screen::Onboarding),
                    0 => self.start_campaign(),
                    1 => self.go(Screen::QuickPlay),
                    2 => self.go(Screen::Campaign),
                    3 => self.go(Screen::Focused),
                    4 => self.go(Screen::Daily),
                    5 => {
                        self.history_offset = 0;
                        self.go(Screen::Statistics)
                    }
                    6 => self.go(Screen::Settings),
                    _ => self.go(Screen::Help),
                },
                KeyCode::Char('q') => self.quit = true,
                KeyCode::Char('?') => self.go(Screen::Help),
                _ => {}
            },
            Screen::Onboarding => match key.code {
                KeyCode::Enter => self.start_guided_practice(),
                KeyCode::Esc => self.menu(now),
                _ => {}
            },
            Screen::Campaign => match key.code {
                KeyCode::Esc => self.menu(now),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selection = self.selection.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => self.selection = (self.selection + 1).min(4),
                KeyCode::Enter if self.selection <= self.profile.level() => {
                    let mission = if self.selection == self.profile.level() {
                        self.profile.completed_missions.min(14)
                    } else {
                        self.selection * 3
                    };
                    self.request_start(
                        SessionRequest::Standard(self.selection, Some(mission)),
                        now,
                    );
                }
                _ => {}
            },
            Screen::QuickPlay => match key.code {
                KeyCode::Esc => self.menu(now),
                KeyCode::Up | KeyCode::Down => self.selection = 1 - self.selection,
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selection == 0 {
                        self.quick_mode = (self.quick_mode + 5) % 6
                    } else {
                        self.quick_level = (self.quick_level + 4) % 5
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.selection == 0 {
                        self.quick_mode = (self.quick_mode + 1) % 6
                    } else {
                        self.quick_level = (self.quick_level + 1) % 5
                    }
                }
                KeyCode::Enter => {
                    self.request_start(SessionRequest::Standard(self.quick_level, None), now)
                }
                _ => {}
            },
            Screen::Focused => match key.code {
                KeyCode::Esc => self.menu(now),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selection = self.selection.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selection =
                        (self.selection + 1).min(focus_patterns(&self.profile.errors).len() - 1)
                }
                KeyCode::Enter => self.request_start(SessionRequest::Focused(self.selection), now),
                _ => {}
            },
            Screen::Daily => match key.code {
                KeyCode::Esc => self.menu(now),
                KeyCode::Enter => self.request_start(SessionRequest::Daily(self.today), now),
                _ => {}
            },
            Screen::Results => match key.code {
                KeyCode::Enter => {
                    if self.session.as_ref().is_some_and(|s| s.daily_day.is_some()) {
                        self.start_daily(self.today)
                    } else if self.session.as_ref().is_some_and(|s| s.focused) {
                        self.start_focused(0)
                    } else if self.session.as_ref().is_some_and(|s| s.mission.is_some()) {
                        if self.result.as_ref().is_some_and(|r| !r.passed) {
                            self.restart_session();
                        } else {
                            self.start_campaign();
                        }
                    } else {
                        self.start(self.quick_level, None)
                    }
                }
                KeyCode::Char('r') => self.retry(now),
                KeyCode::Esc => self.menu(now),
                _ => {}
            },
            Screen::Statistics => match key.code {
                KeyCode::Esc | KeyCode::Enter => self.menu(now),
                KeyCode::Down => {
                    self.history_offset =
                        (self.history_offset + 1).min(self.profile.history.len().saturating_sub(1))
                }
                KeyCode::Up => self.history_offset = self.history_offset.saturating_sub(1),
                _ => {}
            },
            Screen::Settings => match key.code {
                KeyCode::Esc => self.menu(now),
                KeyCode::Char(c @ '1'..='4') => {
                    self.selection = SETTINGS_SECTIONS[(c as u8 - b'1') as usize].1;
                }
                KeyCode::Up | KeyCode::Char('k') => self.selection = (self.selection + 11) % 12,
                KeyCode::Down | KeyCode::Char('j') => self.selection = (self.selection + 1) % 12,
                KeyCode::Left | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                    let backwards = key.code == KeyCode::Left;
                    match self.selection {
                        0 => {
                            let i = ThemeName::ALL
                                .iter()
                                .position(|t| *t == self.settings.theme)
                                .unwrap_or(0);
                            let n = ThemeName::ALL.len();
                            self.settings.theme =
                                ThemeName::ALL[(i + if backwards { n - 1 } else { 1 }) % n];
                        }
                        6 => {
                            self.settings.sound.enabled = !self.settings.sound.enabled;
                            self.sound_requested = self.settings.sound.enabled;
                        }
                        7 => {
                            let i = SoundProfile::ALL
                                .iter()
                                .position(|s| *s == self.settings.sound.profile)
                                .unwrap_or(0);
                            self.settings.sound.profile = SoundProfile::ALL[(i + if backwards {
                                SoundProfile::ALL.len() - 1
                            } else {
                                1
                            }) % SoundProfile::ALL
                                .len()];
                            self.sound_requested = true;
                        }
                        8 => {
                            let volume = self.settings.sound.volume;
                            self.settings.sound.volume = if backwards {
                                volume.saturating_sub(5)
                            } else {
                                volume.saturating_add(5).min(MAX_SOUND_VOLUME)
                            };
                            self.sound_requested = true;
                        }
                        9 => {
                            self.sound_requested = true;
                            if !self.settings.sound.enabled {
                                self.notice = Some("Enable Key sounds to preview a sound.".into());
                            }
                        }
                        1 => self.settings.high_contrast = !self.settings.high_contrast,
                        2 => self.settings.reduced_motion = !self.settings.reduced_motion,
                        3 => self.settings.ascii = !self.settings.ascii,
                        4 => self.settings.underline_caret = !self.settings.underline_caret,
                        5 => self.settings.layout.sidebar = !self.settings.layout.sidebar,
                        10 => self.settings.practice.adaptive = !self.settings.practice.adaptive,
                        _ => self.reload_requested = true,
                    }
                    if !matches!(self.selection, 9 | 11) {
                        self.settings_dirty = true;
                    }
                }
                _ => {}
            },
            Screen::Help => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    self.menu(now)
                }
            }
        }
        self.tick(now, timestamp);
    }
    fn retry(&mut self, now: Duration) {
        if self.session.is_some() {
            self.request_start(SessionRequest::Retry, now);
        }
    }
    fn restart_session(&mut self) {
        let Some(previous) = self.session.as_ref() else {
            return;
        };
        if let Some(day) = previous.daily_day {
            self.start_daily(day);
            return;
        }
        let focused = previous.focused;
        let level = previous.level;
        let mission = previous.mission;
        let seconds = previous.duration.map(|d| d.as_secs());
        let words = previous.prompt.iter().filter(|c| **c == ' ').count() + 1;
        let errors = if self.settings.practice.adaptive {
            self.profile.errors.clone()
        } else {
            Default::default()
        };
        let prompt = if focused {
            focused_prompt(&self.focus_pattern, self.profile.sessions.wrapping_add(42))
        } else {
            self.provider.prompt(level, words, &errors)
        };
        let mut session = Session::new(prompt, seconds, level, mission);
        session.focused = focused;
        self.session = Some(session);
        self.result = None;
        self.go(Screen::Playing);
    }
}
