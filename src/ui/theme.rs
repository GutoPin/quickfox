use crate::config::{ColorPreference, Settings, ThemeName, parse_hex};
use ratatui::style::Color;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Ansi16,
    Mono,
}
impl ColorMode {
    pub fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::Mono;
        }
        let c = std::env::var("COLORTERM").unwrap_or_default();
        let t = std::env::var("TERM").unwrap_or_default();
        if c.contains("truecolor") || c.contains("24bit") {
            Self::TrueColor
        } else if t.contains("256color") {
            Self::Ansi256
        } else {
            Self::Ansi16
        }
    }
    pub fn configured(settings: &Settings) -> Self {
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::Mono;
        }
        match settings.color_mode {
            ColorPreference::Auto => Self::detect(),
            ColorPreference::TrueColor => Self::TrueColor,
            ColorPreference::Ansi256 => Self::Ansi256,
            ColorPreference::Ansi16 => Self::Ansi16,
            ColorPreference::Mono => Self::Mono,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Theme {
    pub brand: Color,
    pub bg: Color,
    pub panel: Color,
    pub fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub gold: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub selection: Color,
    pub border: Color,
}
impl Theme {
    pub fn new(settings: &Settings, mode: ColorMode) -> Self {
        use Color::*;
        let values = match settings.theme {
            ThemeName::Nightfall | ThemeName::Custom => [
                0x0a0a0a, 0x181818, 0xeeeeee, 0xa3a3a3, 0xeeeeee, 0xeeeeee, 0x8f8f8f, 0xeeeeee,
                0xa3a3a3, 0xa3a3a3, 0x292929, 0x555555,
            ],
            ThemeName::Orchid => [
                0x160f24, 0x251b37, 0xf6f0ff, 0xbaabce, 0xc6a1ff, 0x93e3bf, 0xff879c, 0xffd28a,
                0x87ccff, 0xf6a5d2, 0x362744, 0x71617e,
            ],
            ThemeName::Paperwave => [
                0xf6f4ee, 0xe9e4db, 0x25324a, 0x505e72, 0x315cbd, 0x126c4d, 0xa92e3f, 0x8a5700,
                0x275c91, 0x78449a, 0xd7deea, 0x8490a0,
            ],
            ThemeName::Ember => [
                0x171210, 0x251c19, 0xfff2e8, 0xcbb7aa, 0xffb38a, 0xa0d8a0, 0xff8a9a, 0xf5d47c,
                0x91c7e8, 0xd3a9f0, 0x3c2c26, 0x806653,
            ],
            ThemeName::Nord => [
                0x2e3440, 0x3b4252, 0xeceff4, 0xb9c4d3, 0x88c0d0, 0xa3be8c, 0xff9cab, 0xebcb8b,
                0x9db7d8, 0xd5b4ce, 0x4c566a, 0x67758b,
            ],
            ThemeName::Dracula => [
                0x282a36, 0x343746, 0xf8f8f2, 0xbfc4de, 0xbd93f9, 0x50fa7b, 0xff8a8a, 0xf1fa8c,
                0x8be9fd, 0xff79c6, 0x44475a, 0x6272a4,
            ],
            ThemeName::Catppuccin => [
                0x1e1e2e, 0x313244, 0xcdd6f4, 0xbac2de, 0xcba6f7, 0xa6e3a1, 0xf38ba8, 0xf9e2af,
                0x89dceb, 0xf5c2e7, 0x45475a, 0x6c7086,
            ],
            ThemeName::TokyoNight => [
                0x1a1b26, 0x24283b, 0xc0caf5, 0xa9b1d6, 0x7aa2f7, 0x9ece6a, 0xf7768e, 0xe0af68,
                0x7dcfff, 0xbb9af7, 0x343b58, 0x565f89,
            ],
            ThemeName::Gruvbox => [
                0x282828, 0x3c3836, 0xebdbb2, 0xbdae93, 0xfabd2f, 0xb8bb26, 0xfb8075, 0xfea04b,
                0xa5c8ae, 0xdba2b2, 0x504945, 0x7c6f64,
            ],
            ThemeName::RosePine => [
                0x191724, 0x26233a, 0xe0def4, 0xb0adc7, 0xc4a7e7, 0x9ccfd8, 0xeb6f92, 0xf6c177,
                0x9ccfd8, 0xebbcba, 0x403d52, 0x6e6a86,
            ],
            ThemeName::SolarizedLight => [
                0xfdf6e3, 0xeee8d5, 0x073642, 0x4b6269, 0x006c87, 0x466c00, 0xb52b28, 0x805b00,
                0x006f69, 0x80578d, 0xddd6c1, 0x839496,
            ],
            ThemeName::Forest => [
                0x101b16, 0x1b2b22, 0xe2f0dc, 0xaac1ab, 0xa8d982, 0x98d4ac, 0xf29b8f, 0xe8c47e,
                0x8cd1c8, 0xccb0d8, 0x344b3a, 0x637b68,
            ],
            ThemeName::Mono => [0; 12],
        };
        let names = [
            "bg",
            "panel",
            "fg",
            "muted",
            "accent",
            "success",
            "error",
            "gold",
            "secondary",
            "tertiary",
            "selection",
            "border",
        ];
        let mut colors = [Reset; 12];
        for (i, value) in values.iter().enumerate() {
            colors[i] = if settings.theme == ThemeName::Mono || mode == ColorMode::Mono {
                Reset
            } else {
                Rgb((value >> 16) as u8, (value >> 8) as u8, *value as u8)
            };
            if settings.theme == ThemeName::Custom
                && mode != ColorMode::Mono
                && let Some((r, g, b)) = settings
                    .custom_theme
                    .get(names[i])
                    .and_then(|v| parse_hex(v))
            {
                colors[i] = Rgb(r, g, b);
            }
        }
        if mode == ColorMode::Ansi256 && settings.theme != ThemeName::Mono {
            colors = colors.map(indexed);
        }
        if mode == ColorMode::Ansi16 && settings.theme != ThemeName::Mono {
            colors = if matches!(
                settings.theme,
                ThemeName::Paperwave | ThemeName::SolarizedLight
            ) {
                [
                    White, White, Black, Black, Blue, Green, Red, Black, Blue, Magenta, Gray,
                    DarkGray,
                ]
            } else {
                [
                    Black,
                    Black,
                    White,
                    Gray,
                    LightCyan,
                    LightGreen,
                    LightRed,
                    LightYellow,
                    LightBlue,
                    LightMagenta,
                    DarkGray,
                    Gray,
                ]
            };
        }
        if mode == ColorMode::Ansi16 && settings.theme == ThemeName::Nightfall {
            colors = [
                Black, Black, White, Gray, White, White, Gray, White, Gray, Gray, DarkGray,
                DarkGray,
            ];
        }
        if settings.high_contrast {
            colors[3] = colors[2];
        }
        let brand = if settings.theme == ThemeName::Nightfall {
            match mode {
                ColorMode::TrueColor => Rgb(255, 133, 77),
                ColorMode::Ansi256 => indexed(Rgb(255, 133, 77)),
                ColorMode::Ansi16 => LightYellow,
                ColorMode::Mono => Reset,
            }
        } else {
            colors[4]
        };
        Self {
            brand,
            bg: colors[0],
            panel: colors[1],
            fg: colors[2],
            muted: colors[3],
            accent: colors[4],
            success: colors[5],
            error: colors[6],
            gold: colors[7],
            secondary: colors[8],
            tertiary: colors[9],
            selection: colors[10],
            border: colors[11],
        }
    }
}
fn indexed(color: Color) -> Color {
    if let Color::Rgb(r, g, b) = color {
        if r == g && g == b {
            return Color::Indexed(match r {
                0..=4 => 16,
                249..=255 => 231,
                _ => 232 + ((r.saturating_sub(8) + 5) / 10).min(23),
            });
        }
        let cube = |c: u8| -> u8 {
            if c < 48 {
                0
            } else if c < 115 {
                1
            } else {
                ((u16::from(c) - 35) / 40).min(5) as u8
            }
        };
        Color::Indexed(16 + 36 * cube(r) + 6 * cube(g) + cube(b))
    } else {
        color
    }
}
