mod session;
pub use session::{Metrics, Session};

#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub name: &'static str,
    pub focus: &'static str,
}

pub const LEVELS: [Level; 5] = [
    Level {
        name: "First Keys",
        focus: "Short, familiar words",
    },
    Level {
        name: "Flow",
        focus: "Common English words",
    },
    Level {
        name: "Momentum",
        focus: "Longer words and new patterns",
    },
    Level {
        name: "Precision",
        focus: "Capitals and punctuation",
    },
    Level {
        name: "Mastery",
        focus: "Sentences, numbers, and symbols",
    },
];
pub const MISSION_NAMES: [&str; 3] = ["Warm-up", "Pace", "Endurance"];
pub const MISSION_SECONDS: [u64; 3] = [30, 45, 60];
pub const TARGET_ACCURACY: f64 = 94.0;
pub const MINIMUM_CHARACTERS: usize = 20;
