use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub net_wpm: f64,
    pub gross_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub seconds: f64,
    pub correct: usize,
    pub attempts: usize,
    pub corrections: usize,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub prompt: Vec<char>,
    pub typed: Vec<char>,
    pub duration: Option<Duration>,
    pub level: usize,
    pub mission: Option<usize>,
    pub daily_day: Option<u64>,
    pub focused: bool,
    pub errors: BTreeMap<String, u32>,
    pub attempts: usize,
    pub correct_attempts: usize,
    pub corrections: usize,
    pub finished: bool,
    pub paused: bool,
    started_at: Option<Duration>,
    paused_at: Option<Duration>,
    paused_for: Duration,
    elapsed: Duration,
    correct_by_window: Vec<usize>,
}

impl Session {
    pub fn new(prompt: String, seconds: Option<u64>, level: usize, mission: Option<usize>) -> Self {
        Self {
            prompt: prompt.chars().collect(),
            typed: Vec::new(),
            duration: seconds.map(Duration::from_secs),
            level,
            mission,
            daily_day: None,
            focused: false,
            errors: BTreeMap::new(),
            attempts: 0,
            correct_attempts: 0,
            corrections: 0,
            finished: false,
            paused: false,
            started_at: None,
            paused_at: None,
            paused_for: Duration::ZERO,
            elapsed: Duration::ZERO,
            correct_by_window: Vec::new(),
        }
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
    pub fn remaining(&self) -> Option<Duration> {
        self.duration
            .map(|limit| limit.saturating_sub(self.elapsed))
    }

    pub fn tick(&mut self, now: Duration) {
        if self.paused || self.finished {
            return;
        }
        if let Some(start) = self.started_at {
            self.elapsed = now.saturating_sub(start).saturating_sub(self.paused_for);
            if let Some(limit) = self.duration
                && self.elapsed >= limit
            {
                self.elapsed = limit;
                self.finished = true;
            }
        }
    }

    pub fn input(&mut self, character: char, now: Duration) {
        self.tick(now);
        if self.paused || self.finished || !(character.is_ascii_graphic() || character == ' ') {
            return;
        }
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
        let position = self.typed.len();
        let Some(&expected) = self.prompt.get(position) else {
            return;
        };
        self.attempts += 1;
        if character == expected {
            self.correct_attempts += 1;
            let bucket = (self.elapsed.as_secs() / 5) as usize;
            self.correct_by_window.resize(bucket + 1, 0);
            self.correct_by_window[bucket] += 1;
        } else {
            *self.errors.entry(expected.to_string()).or_default() += 1;
            if position > 0 && self.prompt[position - 1] != ' ' && expected != ' ' {
                *self
                    .errors
                    .entry(format!("{}{expected}", self.prompt[position - 1]))
                    .or_default() += 1;
            }
        }
        self.typed.push(character);
        if self.typed.len() == self.prompt.len() {
            self.finished = true;
        }
    }

    pub fn backspace(&mut self, now: Duration) {
        self.tick(now);
        if !self.paused && !self.finished && self.typed.pop().is_some() {
            self.corrections += 1;
        }
    }

    pub fn pause(&mut self, now: Duration) {
        self.tick(now);
        if !self.paused && !self.finished {
            self.paused = true;
            self.paused_at = Some(now);
        }
    }

    pub fn resume(&mut self, now: Duration) {
        if self.paused {
            if let Some(at) = self.paused_at.take()
                && self.has_started()
            {
                self.paused_for += now.saturating_sub(at);
            }
            self.paused = false;
        }
    }

    pub fn progress(&self) -> f64 {
        if let Some(limit) = self.duration {
            (self.elapsed.as_secs_f64() / limit.as_secs_f64()).clamp(0.0, 1.0)
        } else {
            self.typed.len() as f64 / self.prompt.len().max(1) as f64
        }
    }

    pub fn metrics(&self) -> Metrics {
        let seconds = self.elapsed.as_secs_f64();
        let correct = self
            .typed
            .iter()
            .zip(&self.prompt)
            .filter(|(a, b)| a == b)
            .count();
        let rate = if seconds >= 1.0 { 12.0 / seconds } else { 0.0 };
        // Only complete five-second windows participate; the last partial window is excluded.
        let windows = (self.elapsed.as_secs() / 5) as usize;
        let counts: Vec<f64> = (0..windows)
            .map(|i| *self.correct_by_window.get(i).unwrap_or(&0) as f64)
            .collect();
        let mean = counts.iter().sum::<f64>() / windows.max(1) as f64;
        let consistency = if windows >= 2 && mean > 0.0 {
            let variance = counts.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / windows as f64;
            (100.0 * (1.0 - variance.sqrt() / mean)).clamp(0.0, 100.0)
        } else {
            0.0
        };
        Metrics {
            net_wpm: correct as f64 * rate,
            gross_wpm: self.attempts as f64 * rate,
            accuracy: if self.attempts == 0 {
                100.0
            } else {
                self.correct_attempts as f64 * 100.0 / self.attempts as f64
            },
            consistency,
            seconds,
            correct,
            attempts: self.attempts,
            corrections: self.corrections,
        }
    }
}
