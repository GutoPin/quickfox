use std::collections::BTreeMap;

pub const COMMON: &str = include_str!("../assets/words/en/common.txt");
pub const ADVANCED: &str = include_str!("../assets/words/en/advanced.txt");
pub const PHRASES: &str = include_str!("../assets/words/en/phrases.txt");

pub trait PromptProvider {
    fn prompt(&mut self, level: usize, words: usize, errors: &BTreeMap<String, u32>) -> String;
}

pub struct LocalAdaptiveProvider {
    seed: u64,
}
impl LocalAdaptiveProvider {
    pub fn new(seed: u64) -> Self {
        Self { seed: seed.max(1) }
    }
    fn next(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }
}
impl PromptProvider for LocalAdaptiveProvider {
    fn prompt(&mut self, level: usize, words: usize, errors: &BTreeMap<String, u32>) -> String {
        let mut pool: Vec<&str> = COMMON
            .split_whitespace()
            .filter(|word| match level {
                0 => word.len() <= 4,
                1 => word.len() <= 7,
                _ => true,
            })
            .collect();
        if level == 2 {
            pool.extend(ADVANCED.split_whitespace());
        }
        if level >= 3 {
            pool = PHRASES
                .lines()
                .filter(|line| {
                    !line.is_empty() && (level >= 4 || !line.chars().any(|c| c.is_ascii_digit()))
                })
                .collect();
        }
        let weights: Vec<u64> = pool
            .iter()
            .map(|word| {
                let lower = word.to_ascii_lowercase();
                10 + errors
                    .iter()
                    .filter(|(pattern, _)| lower.contains(pattern.as_str()))
                    .map(|(_, count)| u64::from((*count).min(10)))
                    .sum::<u64>()
                    .min(30)
            })
            .collect();
        let total: u64 = weights.iter().sum();
        let mut selected = Vec::new();
        let mut count = 0;
        let mut previous = None;
        while count < words {
            let mut roll = self.next() % total;
            let mut index = 0;
            for (i, weight) in weights.iter().enumerate() {
                if roll < *weight {
                    index = i;
                    break;
                }
                roll -= *weight;
            }
            if previous == Some(index) {
                continue;
            }
            let word = pool[index];
            count += word.split_whitespace().count();
            selected.push(word);
            previous = Some(index);
        }
        // Word mode is exactly the requested length, including sentence-based levels.
        selected
            .join(" ")
            .split_whitespace()
            .take(words)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn daily_prompt(day: u64) -> String {
    // UTC day and a fixed content version define the same non-adaptive challenge for everyone.
    LocalAdaptiveProvider::new(day.wrapping_add(0x0058_d411_2026)).prompt(2, 600, &BTreeMap::new())
}
pub fn focus_patterns(errors: &BTreeMap<String, u32>) -> Vec<(String, u32)> {
    let mut patterns: Vec<_> = errors
        .iter()
        .filter(|(key, _)| key.chars().all(|c| c.is_ascii_alphabetic()))
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    patterns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    patterns.truncate(8);
    if patterns.is_empty() {
        patterns = vec![("th".into(), 0), ("st".into(), 0), ("ou".into(), 0)];
    }
    patterns
}
pub fn focused_prompt(pattern: &str, seed: u64) -> String {
    let pool: Vec<_> = COMMON
        .split_whitespace()
        .chain(ADVANCED.split_whitespace())
        .filter(|word| word.contains(pattern))
        .collect();
    if pool.is_empty() {
        return LocalAdaptiveProvider::new(seed).prompt(1, 50, &BTreeMap::new());
    }
    let mut rng = LocalAdaptiveProvider::new(seed);
    let mut result = Vec::new();
    for _ in 0..50 {
        let mut index = (rng.next() as usize) % pool.len();
        if pool.len() > 1 && result.last() == Some(&pool[index]) {
            index = (index + 1) % pool.len();
        }
        result.push(pool[index]);
    }
    result.join(" ")
}
