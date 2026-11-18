pub mod theme;
use crate::{
    app::{Action, App, QuickMode, SETTINGS_SECTIONS, Screen, settings_section},
    config::MAX_SOUND_VOLUME,
    game::{LEVELS, MISSION_NAMES, MISSION_SECONDS},
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Gauge, Paragraph, Wrap},
};
use std::{ops::Range, time::Duration};
use theme::{ColorMode, Theme};

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 20;
const SIDEBAR_WIDTH: u16 = 112;

fn text(frame: &mut Frame, area: Rect, y: u16, line: impl Into<Line<'static>>) {
    if y < area.height {
        frame.render_widget(
            Paragraph::new(line.into()),
            Rect::new(area.x, area.y + y, area.width, 1),
        );
    }
}
fn span(value: impl Into<String>, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(color))
}
fn bold(value: impl Into<String>, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(color).bold())
}
fn heading(frame: &mut Frame, area: Rect, title: &str, subtitle: &str, t: Theme) {
    text(frame, area, 0, Line::from(bold(title, t.fg)));
    frame.render_widget(
        Paragraph::new(subtitle.to_owned())
            .style(Style::default().fg(t.muted))
            .wrap(Wrap { trim: true }),
        Rect::new(area.x, area.y + 2, area.width, 2),
    );
}
fn visible_range(
    frame: &mut Frame,
    area: Rect,
    top: usize,
    capacity: usize,
    total: usize,
    t: Theme,
) {
    let label = if total == 0 {
        "0 results".to_owned()
    } else {
        format!("{}-{} / {}", top + 1, (top + capacity).min(total), total)
    };
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Right)
            .style(Style::default().fg(t.muted)),
        Rect::new(
            area.x + area.width.saturating_sub(14),
            area.y,
            area.width.min(14),
            1,
        ),
    );
}
fn choice(
    frame: &mut Frame,
    area: Rect,
    y: u16,
    label: &str,
    value: &str,
    selected: bool,
    t: Theme,
) {
    if y >= area.height {
        return;
    }
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            bold(if selected { "> " } else { "  " }, t.brand),
            span(format!("{label:<21}"), t.fg),
            span(value.to_owned(), if selected { t.fg } else { t.muted }),
        ]))
        .style(Style::default().bg(if selected { t.selection } else { t.bg })),
        Rect::new(area.x, area.y + y, area.width, 1),
    );
}
fn track(
    frame: &mut Frame,
    area: Rect,
    y: u16,
    progress: f64,
    label: String,
    t: Theme,
    ascii: bool,
) {
    if y >= area.height {
        return;
    }
    let row = Rect::new(area.x, area.y + y, area.width, 1);
    if ascii {
        let width = area.width.saturating_sub(label.len() as u16 + 3) as usize;
        let filled = (width as f64 * progress.clamp(0.0, 1.0)).round() as usize;
        text(
            frame,
            area,
            y,
            Line::from(vec![
                span("=".repeat(filled), t.accent),
                span("-".repeat(width - filled), t.muted),
                span(format!("  {label}"), t.fg),
            ]),
        );
    } else {
        frame.render_widget(
            Gauge::default()
                .ratio(progress.clamp(0.0, 1.0))
                .label(label)
                .gauge_style(Style::default().fg(t.accent).bg(t.panel))
                .use_unicode(true),
            row,
        );
    }
}

pub fn render(frame: &mut Frame, app: &App, mode: ColorMode, now: Duration) {
    let area = frame.area();
    let t = Theme::new(&app.settings, mode);
    frame.render_widget(
        Block::default().style(Style::default().bg(t.bg).fg(t.fg)),
        area,
    );
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(bold(crate::NAME.to_uppercase(), t.accent)),
                Line::from(""),
                Line::from("This window is too small."),
                Line::from(format!("Resize to at least {MIN_WIDTH} x {MIN_HEIGHT}.")),
                Line::from("Your session is paused. Ctrl+C exits."),
            ])
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    if app.pending.is_some() {
        confirm_restart(frame, area, t);
        return;
    }
    if app.palette.is_some() {
        command_palette(frame, area, app, t);
        return;
    }
    if app.screen == Screen::Home {
        home(frame, area, app, t);
        return;
    }
    let sidebar = app.settings.layout.sidebar
        && area.width >= SIDEBAR_WIDTH
        && matches!(
            app.screen,
            Screen::QuickPlay
                | Screen::Focused
                | Screen::Campaign
                | Screen::Daily
                | Screen::Settings
                | Screen::Help
                | Screen::Statistics
        );
    let expanded = matches!(
        app.screen,
        Screen::Playing | Screen::Settings | Screen::Menu
    );
    let max_width = match (expanded, sidebar) {
        (true, true) => 172,
        (true, false) => 148,
        (false, true) => 132,
        (false, false) => 96,
    };
    let width = area.width.saturating_sub(4).min(max_width);
    let height = (area.height - 2).min(44);
    let shell = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let header = Rect::new(shell.x, shell.y, shell.width, 2);
    frame.render_widget(
        Block::default().style(Style::default().bg(t.panel)),
        Rect::new(header.x, header.y, width, 1),
    );
    let tab = format!("  {}  ", crate::NAME.to_uppercase());
    let tab_width = tab.len();
    text(
        frame,
        header,
        0,
        Line::from(vec![
            bold(tab, t.bg).bg(t.brand),
            bold(format!("   {}", app.screen.label()), t.fg),
        ]),
    );
    let rule = if app.settings.ascii { "-" } else { "─" };
    text(
        frame,
        header,
        1,
        Line::from(vec![
            span(rule.repeat(tab_width), t.brand),
            span(
                rule.repeat(usize::from(width).saturating_sub(tab_width)),
                t.border,
            ),
        ]),
    );
    let status = if app.screen == Screen::Playing {
        if app.settings.sound.enabled {
            format!(
                "{} / {}% sound ",
                app.settings.sound.profile.label(),
                app.settings.sound.volume
            )
        } else {
            "sound off ".into()
        }
    } else {
        format!("{} XP ", app.profile.xp)
    };
    if shell.width >= 64 {
        frame.render_widget(
            Paragraph::new(status)
                .alignment(Alignment::Right)
                .style(Style::default().fg(t.muted)),
            Rect::new(shell.x + shell.width - 26, shell.y, 26, 1),
        );
    }
    let content = Rect::new(shell.x, shell.y + 3, shell.width, shell.height - 7);
    let body = if sidebar {
        practice_summary(
            frame,
            Rect::new(content.x, content.y, 25, content.height),
            app,
            t,
        );
        Rect::new(
            content.x + 28,
            content.y,
            content.width - 28,
            content.height,
        )
    } else {
        Rect::new(content.x + 2, content.y, content.width - 4, content.height)
    };
    match app.screen {
        Screen::Home => unreachable!("home has its own centered composition"),
        Screen::Menu => menu(frame, body, app, t),
        Screen::Onboarding => onboarding(frame, body, t),
        Screen::Campaign => campaign(frame, body, app, t),
        Screen::QuickPlay => quick_play(frame, body, app, t),
        Screen::Focused => focused(frame, body, app, t),
        Screen::Daily => daily(frame, body, app, t),
        Screen::Playing => playing(frame, body, app, t),
        Screen::Paused => paused(frame, body, app, t),
        Screen::Results => results(frame, body, app, t, now),
        Screen::Statistics => statistics(frame, body, app, t),
        Screen::Settings => settings(frame, body, app, t),
        Screen::Help => help(frame, body, app, t),
    }
    let controls = match app.screen {
        Screen::Home => "Up/Down choose   Enter open   Q quit",
        Screen::Menu => "Arrows choose   Enter open   Esc home   Q quit",
        Screen::Playing => "Esc pause   Backspace correct   Tab menu",
        Screen::Paused => "Enter resume   Esc menu   R restart   Q discard",
        Screen::Campaign if app.selection > app.profile.level() => {
            "Locked: finish the previous level   Esc menu"
        }
        Screen::Results if app.result.as_ref().is_some_and(|r| r.campaign && !r.passed) => {
            "Enter retry mission   Esc menu"
        }
        Screen::Results if app.result.as_ref().is_some_and(|r| r.campaign) => {
            "Enter continue campaign   R retry   Esc menu"
        }
        Screen::Results => "Enter practice again   R retry   Esc menu",
        Screen::Settings => "1-4 section   Up/Down select   Left/Right change",
        Screen::QuickPlay => "Arrows adjust   Enter start   Esc menu",
        Screen::Daily => "Enter start challenge   Esc menu",
        Screen::Onboarding => "Enter start typing   Esc menu",
        Screen::Help => "Esc menu   Ctrl+C quit",
        Screen::Statistics => "Up/Down history   Esc menu",
        _ => "Up/Down choose   Enter start   Esc menu",
    };
    let foot = Rect::new(shell.x, shell.y + shell.height - 2, shell.width, 2);
    text(
        frame,
        foot,
        0,
        Line::from(span(format!(" {controls}"), t.fg)),
    );
    if app.screen != Screen::Playing {
        text(
            frame,
            foot,
            1,
            Line::from(vec![
                bold(" tab", t.fg),
                span(" / ", t.muted),
                bold(format!("{} ", app.settings.keybindings.menu), t.fg),
                span("menu   ", t.muted),
                bold(format!("{} ", app.settings.keybindings.commands), t.fg),
                span("commands   ", t.muted),
                bold(format!("{} ", app.settings.keybindings.reload_config), t.fg),
                span("reload", t.muted),
            ]),
        );
    }
    if let Some(notice) = &app.notice {
        text(
            frame,
            shell,
            shell.height - 3,
            Line::from(span(notice.clone(), t.gold)),
        );
    }
}
fn menu_items(app: &App) -> [(&'static str, &'static str); 8] {
    [
        (
            if app.has_session() {
                "Resume session"
            } else if app.profile.sessions == 0 {
                "Start typing"
            } else {
                "Continue campaign"
            },
            if app.has_session() {
                "Pick up where you left off"
            } else {
                "Your next mission"
            },
        ),
        ("Quick play", "Time or word count"),
        ("Campaign", "15 missions, 5 levels"),
        ("Focused practice", "Train your difficult keys"),
        ("Daily challenge", "60 seconds, new words each day"),
        ("Statistics", "Speed and accuracy over time"),
        ("Settings", "Colors, keys, and layout"),
        ("Help", "Keys and scoring rules"),
    ]
}
fn practice_summary(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    text(frame, area, 0, Line::from(bold("Your practice", t.fg)));
    text(
        frame,
        area,
        2,
        Line::from(span("Campaign progress", t.muted)),
    );
    text(
        frame,
        area,
        3,
        Line::from(bold(
            format!("{} / 15 missions", app.profile.completed_missions),
            t.fg,
        )),
    );
    text(
        frame,
        area,
        5,
        Line::from(span("Sessions completed", t.muted)),
    );
    text(
        frame,
        area,
        6,
        Line::from(bold(app.profile.sessions.to_string(), t.fg)),
    );
}
/// The Home masthead in two tones: the second word carries the theme's brand color.
const WORDMARK: [&str; 2] = ["QUICK", "FOX"];
// Pixel capitals with two-pixel stems: (width, rows), most significant bit on the left.
fn glyph(c: char) -> (usize, [u8; 7]) {
    match c {
        'K' => (6, [51, 54, 60, 56, 60, 54, 51]),
        'E' => (6, [63, 48, 48, 62, 48, 48, 63]),
        'Q' => (6, [30, 51, 51, 51, 51, 54, 27]),
        'C' => (6, [30, 51, 48, 48, 48, 51, 30]),
        'O' => (6, [30, 51, 51, 51, 51, 51, 30]),
        'X' => (6, [51, 51, 30, 12, 30, 51, 51]),
        'F' => (6, [63, 48, 48, 62, 48, 48, 48]),
        'M' => (7, [99, 119, 127, 107, 99, 99, 99]),
        'N' => (6, [51, 59, 63, 55, 51, 51, 51]),
        'U' => (6, [51, 51, 51, 51, 51, 51, 30]),
        'S' => (6, [31, 48, 48, 30, 3, 3, 62]),
        'T' => (6, [63, 12, 12, 12, 12, 12, 12]),
        'I' => (2, [3; 7]),
        'G' => (6, [30, 51, 48, 55, 51, 51, 30]),
        _ => (6, [0; 7]),
    }
}
/// Lettering width in pixels: glyphs with one-pixel gaps, the two-pixel lean and shadow.
fn title_pixels(words: &[(&str, Color)]) -> usize {
    let letters = words.iter().flat_map(|(word, _)| word.chars());
    letters.map(|c| glyph(c).0 + 1).sum::<usize>() + 2
}
/// Section titles use the masthead's two-cell pixels or fall back to a text heading.
fn title_fits(words: &[(&str, Color)], area: Rect) -> bool {
    title_pixels(words) * 2 <= usize::from(area.width)
}
/// Draws italic pixel capitals and returns the rows used. Rows lean one whole pixel every
/// two or three rows, so each two-pixel stem overlaps itself and strokes never break.
/// A pixel is `px` cells wide and `px` half-rows tall (about square). A one-pixel shadow
/// in the theme's `selection` tone sits below-right at two-cell pixels. ASCII uses `#`,
/// one row per pixel and no shadow.
fn display_title(
    frame: &mut Frame,
    area: Rect,
    words: &[(&str, Color)],
    px: u16,
    ascii: bool,
    t: Theme,
) -> u16 {
    let px = usize::from(px.max(1));
    let mut grid = vec![vec![None; title_pixels(words)]; 8];
    let mut ink = Vec::new();
    let mut left = 0;
    for (word, color) in words {
        for c in word.chars() {
            let (width, rows) = glyph(c);
            for (row, bits) in rows.iter().enumerate() {
                let lean = (7 - row) / 3;
                for i in (0..width).filter(|i| bits >> (width - 1 - i) & 1 == 1) {
                    ink.push((row, left + lean + i, *color));
                }
            }
            left += width + 1;
        }
    }
    if !ascii && px > 1 && t.selection != Color::Reset {
        for &(row, column, _) in &ink {
            grid[row + 1][column + 1] = Some(t.selection);
        }
    }
    for (row, column, color) in ink {
        grid[row][column] = Some(color);
    }
    let rows = if ascii { 7 } else { 4 * px as u16 };
    let pixel = |x: usize, s: usize| {
        grid.get(s / px)
            .and_then(|r| r.get(x / px))
            .copied()
            .flatten()
    };
    let buffer = frame.buffer_mut();
    for y in 0..rows.min(area.height) {
        for x in 0..area.width {
            let (cx, cy) = (usize::from(x), usize::from(y));
            let (top, bottom) = if ascii {
                (grid[cy].get(cx / px).copied().flatten(), None)
            } else {
                (pixel(cx, cy * 2), pixel(cx, cy * 2 + 1))
            };
            let Some(cell) = buffer.cell_mut((area.x + x, area.y + y)) else {
                continue;
            };
            match (top, bottom) {
                (None, None) => continue,
                _ if ascii => cell.set_char('#').set_fg(top.unwrap_or(t.fg)),
                (Some(a), Some(b)) if a == b => cell.set_char('█').set_fg(a),
                (Some(a), b) => cell.set_char('▀').set_fg(a).set_bg(b.unwrap_or(t.bg)),
                (None, Some(b)) => cell.set_char('▄').set_fg(b).set_bg(t.bg),
            };
        }
    }
    rows
}

fn home(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    let width = area.width.saturating_sub(6).min(122);
    let x = area.x + (area.width - width) / 2;
    let words = [(WORDMARK[0], t.fg), (WORDMARK[1], t.brand)];
    let px = (width / title_pixels(&words) as u16).clamp(1, 2);
    let height = if app.settings.ascii { 7 } else { 4 * px };
    let top = area.y + (area.height.saturating_sub(height + 10)) / 2;
    let hero = Rect::new(x, top, width, height);
    display_title(frame, hero, &words, px, app.settings.ascii, t);
    let mission = app.profile.completed_missions.min(14);
    let (title, detail, action) = if app.has_session() {
        (
            "Resume session".to_owned(),
            "Your words and time are saved here.".to_owned(),
            "resume",
        )
    } else if app.profile.sessions == 0 {
        (
            format!(
                "Your first {}-second practice",
                MISSION_SECONDS[mission % 3]
            ),
            "Copy the words. Your first letter starts the clock.".to_owned(),
            "start",
        )
    } else {
        (
            "Continue campaign".to_owned(),
            format!(
                "{}   /   {}   /   {} sec",
                LEVELS[mission / 3].name,
                MISSION_NAMES[mission % 3],
                MISSION_SECONDS[mission % 3]
            ),
            "continue",
        )
    };
    let practice = Rect::new(x, top + height + 1, width, 7);
    let rule = if app.settings.ascii { "-" } else { "─" };
    text(
        frame,
        practice,
        0,
        Line::from(span(rule.repeat(width.into()), t.border)),
    );
    text(frame, practice, 2, Line::from(bold(title, t.fg)));
    text(frame, practice, 3, Line::from(span(detail, t.muted)));
    text(
        frame,
        practice,
        5,
        Line::from(vec![
            bold(format!("  Enter  {action}  "), t.brand).reversed(),
            span("    Tab  all modes", t.muted),
        ]),
    );
    let footer = Rect::new(area.x + 2, area.y + area.height - 2, area.width - 4, 1);
    text(
        frame,
        footer,
        0,
        Line::from(span(
            format!("{}  /  English typing practice", crate::NAME.to_uppercase()),
            t.muted,
        )),
    );
    frame.render_widget(
        Paragraph::new(format!("v{}  /  q quit", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(Style::default().fg(t.muted)),
        footer,
    );
    if let Some(notice) = &app.notice {
        text(
            frame,
            Rect::new(x, area.y + area.height - 3, width, 1),
            0,
            Line::from(span(notice.clone(), t.gold)),
        );
    }
}
fn menu(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    let words = [("MENU", t.fg)];
    let first = if area.height >= 26 && !app.settings.ascii && title_fits(&words, area) {
        display_title(frame, area, &words, 2, false, t) + 2
    } else {
        text(frame, area, 0, Line::from(bold("Practice menu", t.fg)));
        2
    };
    if area.width >= 86 && area.height >= first + 15 {
        let width = (area.width - 4) / 2;
        for (i, (label, detail)) in menu_items(app).iter().enumerate() {
            let (column, index, group) = if i < 5 {
                (0, i, "Practice")
            } else {
                (1, i - 5, "Workspace")
            };
            let group_area =
                Rect::new(area.x + column * (width + 4), area.y + first - 1, width, 16);
            if index == 0 {
                text(frame, group_area, 0, Line::from(bold(group, t.brand)));
            }
            let y = 2 + index as u16 * 3;
            let row = Rect::new(group_area.x, group_area.y + y, width, 2);
            let selected = app.selection == i;
            if selected {
                frame.render_widget(
                    Block::default().style(Style::default().bg(t.selection)),
                    row,
                );
            }
            text(
                frame,
                row,
                0,
                Line::from(vec![
                    bold(if selected { "> " } else { "  " }, t.brand),
                    bold(*label, t.fg),
                ]),
            );
            text(
                frame,
                row,
                1,
                Line::from(span(format!("  {detail}"), t.muted)),
            );
        }
        return;
    }
    text(
        frame,
        area,
        first - 1,
        Line::from(span("Practice", t.muted)),
    );
    for (i, (label, detail)) in menu_items(app).iter().enumerate() {
        let y = first
            + i as u16
            + if i >= 5 {
                if area.height >= 17 { 2 } else { 1 }
            } else {
                0
            };
        if i == 5 {
            text(frame, area, y - 1, Line::from(span("Workspace", t.muted)));
        }
        let row = Rect::new(area.x, area.y + y, area.width.min(76), 1);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                bold(if app.selection == i { "> " } else { "  " }, t.brand),
                span(*label, t.fg),
            ]))
            .style(Style::default().bg(if app.selection == i {
                t.selection
            } else {
                t.bg
            })),
            row,
        );
        if area.width >= 65 {
            frame.render_widget(
                Paragraph::new(*detail)
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(t.muted)),
                Rect::new(row.x + 26, row.y, row.width - 28, 1),
            );
        }
    }
}
fn onboarding(frame: &mut Frame, area: Rect, t: Theme) {
    heading(
        frame,
        area,
        "How a practice works",
        "Your first session takes 30 seconds. There is no speed target.",
        t,
    );
    for (y, title, description) in [
        (
            5,
            "Follow the words",
            "Copy letters and spaces. Type to start the clock.",
        ),
        (
            8,
            "Fix mistakes",
            "Mistakes are highlighted. Backspace fixes them; they still count.",
        ),
        (
            11,
            "Pause anytime",
            "Esc stops the clock. Enter resumes where you left off.",
        ),
    ] {
        let y = if area.height < 14 {
            (y - 5) / 3 * 2 + 4
        } else {
            y
        };
        text(frame, area, y, Line::from(bold(title, t.fg)));
        text(frame, area, y + 1, Line::from(span(description, t.muted)));
    }
}
fn campaign(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "5 levels, 15 missions",
        "Three missions per level. Reach 94% accuracy and 20 correct characters.",
        t,
    );
    let stride = if area.height >= 19 {
        3
    } else if area.height >= 14 {
        2
    } else {
        1
    };
    for (i, level) in LEVELS.iter().enumerate() {
        let unlocked = i <= app.profile.level();
        let completed = app.profile.completed_missions.saturating_sub(i * 3).min(3);
        let y = 4 + i as u16 * stride;
        if app.selection == i {
            frame.render_widget(
                Block::default().style(Style::default().bg(t.selection)),
                Rect::new(area.x, area.y + y, area.width, 1),
            );
        }
        let color = if app.selection == i || unlocked {
            t.fg
        } else {
            t.muted
        };
        text(
            frame,
            area,
            y,
            Line::from(vec![
                bold(if app.selection == i { "> " } else { "  " }, t.brand),
                bold(format!("{:02}  {:<14}", i + 1, level.name), color),
                span(
                    if unlocked {
                        format!(
                            "{}  {completed}/3",
                            "[+]".repeat(completed) + &"[ ]".repeat(3 - completed)
                        )
                    } else {
                        "Locked".into()
                    },
                    if app.selection == i {
                        t.fg
                    } else if completed == 3 {
                        t.success
                    } else {
                        t.muted
                    },
                ),
            ]),
        );
        if stride == 3 {
            text(
                frame,
                area,
                y + 1,
                Line::from(span(format!("      {}", level.focus), t.muted)),
            );
        }
    }
}
fn quick_play(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Any level, no pass or fail",
        "All five content levels are open. Results never change the campaign.",
        t,
    );
    choice(
        frame,
        area,
        4,
        "Session",
        &format!("< {} >", QuickMode::ALL[app.quick_mode].label()),
        app.selection == 0,
        t,
    );
    text(
        frame,
        area,
        5,
        Line::from(span("  Choose a timer or a fixed word count.", t.muted)),
    );
    choice(
        frame,
        area,
        7,
        "Content",
        &format!("< {} >", LEVELS[app.quick_level].name),
        app.selection == 1,
        t,
    );
    text(
        frame,
        area,
        8,
        Line::from(span(
            format!("  {}", LEVELS[app.quick_level].focus),
            t.muted,
        )),
    );
    if area.height > 14 {
        text(
            frame,
            area,
            12,
            Line::from(vec![bold("Enter ", t.brand), span("start practice", t.fg)]),
        );
        text(
            frame,
            area,
            14,
            Line::from(span("Left/Right changes the selected option.", t.muted)),
        );
    }
}

/// Byte and character offsets are identical because every embedded prompt is ASCII.
pub fn prompt_rows(prompt: &[char], width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    while start < prompt.len() {
        let mut end = (start + width).min(prompt.len());
        if end < prompt.len()
            && let Some(space) = prompt[start..end].iter().rposition(|c| *c == ' ')
            && space > 0
        {
            end = start + space + 1;
        }
        rows.push(start..end);
        start = end;
    }
    rows
}
fn next_key_guide(frame: &mut Frame, area: Rect, next: char, t: Theme) {
    frame.render_widget(Block::default().style(Style::default().bg(t.panel)), area);
    text(
        frame,
        area,
        1,
        Line::from(vec![
            span("  Next key   ", t.muted),
            bold(
                if next == ' ' {
                    "Space".into()
                } else {
                    next.to_string()
                },
                t.brand,
            ),
        ]),
    );
    for (row, keys) in ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"].iter().enumerate() {
        let mut line = vec![Span::raw(" ".repeat(2 + row))];
        for key in keys.chars() {
            let face = format!(" {key} ");
            line.push(if next.eq_ignore_ascii_case(&key) {
                bold(face, t.bg).bg(t.brand).underlined()
            } else {
                span(face, t.muted)
            });
        }
        text(frame, area, 3 + row as u16 * 2, Line::from(line));
    }
    let space = if next == ' ' {
        bold("   space   ", t.bg).bg(t.brand).underlined()
    } else {
        span("   space   ", t.fg).bg(t.selection)
    };
    text(
        frame,
        area,
        9,
        Line::from(vec![Span::raw("          "), space]),
    );
    text(frame, area, 11, Line::from(span("  QWERTY", t.muted)));
}

fn playing(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    let Some(session) = &app.session else {
        return;
    };
    let metrics = session.metrics();
    let title = if session.daily_day.is_some() {
        "Daily challenge / 60 seconds".into()
    } else if session.focused {
        format!("Focused practice / {}", app.focus_pattern)
    } else if let Some(mission) = session.mission {
        format!(
            "{}  /  {}",
            LEVELS[session.level].name,
            MISSION_NAMES[mission % 3]
        )
    } else {
        format!("Quick play  /  {}", LEVELS[session.level].name)
    };
    let guide_width = 34;
    let gutter = 4;
    let preferred_width = app.settings.layout.prompt_width + 6;
    let has_guide = area.width >= 116
        && area.width >= preferred_width + gutter + guide_width
        && area.height >= 18;
    let side_width = if has_guide { guide_width + gutter } else { 0 };
    let width = (area.width - side_width).min(preferred_width.max(52));
    let prompt_lines = if area.height >= 30 { 5 } else { 3 };
    let stride = app.settings.layout.line_spacing + 1;
    let height = area.height.min(13 + (prompt_lines - 1) * stride);
    let x = area.x + (area.width - width - side_width) / 2;
    let y = area.y + (area.height - height) / 2;
    if has_guide && let Some(&next) = session.prompt.get(session.typed.len()) {
        next_key_guide(
            frame,
            Rect::new(x + width + gutter, y + 2, guide_width, 13),
            next,
            t,
        );
    }
    let area = Rect::new(x, y, width, height);
    text(frame, area, 0, Line::from(bold(title, t.fg)));
    let time = if let Some(remaining) = session.remaining() {
        format!("{:02}s left", remaining.as_secs_f64().ceil() as u64)
    } else {
        format!(
            "{:02}:{:02}",
            session.elapsed().as_secs() / 60,
            session.elapsed().as_secs() % 60
        )
    };
    text(
        frame,
        area,
        2,
        Line::from(vec![
            bold(format!("{:.0}", metrics.net_wpm), t.accent),
            span(" wpm       ", t.muted),
            bold(
                format!("{:.0}%", metrics.accuracy),
                if metrics.accuracy >= 94.0 {
                    t.success
                } else {
                    t.error
                },
            ),
            span(" accuracy       ", t.muted),
            bold(time, t.gold),
        ]),
    );
    let hint = if !session.has_started() {
        "Type to begin. Your first key starts the clock."
    } else if session.mission.is_some() {
        "Pass: 94% accuracy and 20 correct characters."
    } else {
        "Backspace fixes a mistake. Esc pauses."
    };
    let reading_height = area
        .height
        .saturating_sub(9)
        .min(1 + (prompt_lines - 1) * stride);
    let reading_width = app.settings.layout.prompt_width.min(area.width - 6);
    let reading = Rect::new(
        area.x + (area.width - reading_width) / 2,
        area.y + 5,
        reading_width,
        reading_height,
    );
    let canvas = Rect::new(area.x, reading.y - 1, area.width, reading.height + 2);
    frame.render_widget(Block::default().style(Style::default().bg(t.panel)), canvas);
    text(
        frame,
        area,
        area.height - 3,
        Line::from(span(hint, t.muted)),
    );
    let width = reading.width;
    let rows = prompt_rows(&session.prompt, width as usize);
    let current = rows
        .iter()
        .position(|range| range.contains(&session.typed.len()))
        .unwrap_or(rows.len().saturating_sub(1));
    let capacity = usize::from(reading.height.div_ceil(stride));
    let top = current
        .saturating_sub(usize::from(capacity > 1))
        .min(rows.len().saturating_sub(capacity));
    for (row, range) in rows.iter().skip(top).take(capacity).enumerate() {
        let active = top + row == current;
        if active {
            text(
                frame,
                canvas,
                1 + row as u16 * stride,
                Line::from(bold(if app.settings.ascii { ">" } else { "▎" }, t.brand)),
            );
        }
        let spans = range
            .clone()
            .map(|i| {
                let expected = session.prompt[i];
                let mut style = Style::default().fg(if active { t.fg } else { t.muted });
                let mut character = expected;
                if let Some(typed) = session.typed.get(i) {
                    if *typed == expected {
                        style = style.fg(t.success).add_modifier(Modifier::BOLD);
                    } else {
                        style = mistake_style(app, t);
                        if expected == ' ' {
                            character = if app.settings.ascii { '_' } else { '·' };
                        }
                    }
                } else if i == session.typed.len() {
                    style = caret_style(app, t);
                    if expected == ' ' {
                        character = if app.settings.ascii { '_' } else { '·' };
                    }
                }
                Span::styled(character.to_string(), style)
            })
            .collect::<Vec<_>>();
        text(frame, reading, row as u16 * stride, Line::from(spans));
    }
    let label = if session.duration.is_some() {
        format!("{}% complete", (session.progress() * 100.0) as u64)
    } else {
        format!(
            "{} / {} characters",
            session.typed.len(),
            session.prompt.len()
        )
    };
    track(
        frame,
        area,
        area.height - 1,
        session.progress(),
        label,
        Theme {
            accent: t.brand,
            ..t
        },
        app.settings.ascii,
    );
}
fn paused(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Paused",
        "The clock is stopped. Your words and time stay here until you exit.",
        t,
    );
    text(
        frame,
        area,
        6,
        Line::from(bold("[ Enter ]  Keep going", t.brand)),
    );
    text(
        frame,
        area,
        8,
        Line::from(span("[ R ]      Start a fresh prompt", t.fg)),
    );
    text(
        frame,
        area,
        10,
        Line::from(span("[ Esc ]    Menu (keep session)", t.muted)),
    );
    if app.session.as_ref().is_some_and(|s| s.has_started()) && area.height > 14 {
        text(
            frame,
            area,
            14,
            Line::from(span(
                "Q discards this session. Restart asks before replacing it.",
                t.muted,
            )),
        );
    }
}
fn results(frame: &mut Frame, area: Rect, app: &App, t: Theme, now: Duration) {
    let Some(result) = &app.result else {
        return;
    };
    let m = result.metrics;
    let title = if result.campaign && result.passed {
        "Mission passed"
    } else if result.campaign {
        "Mission not passed yet"
    } else {
        "Session complete"
    };
    let subtitle = if result.campaign && result.passed {
        "You reached 94% accuracy and 20 correct characters. The next mission is open."
    } else if result.campaign {
        "Try again: 94% accuracy and 20 correct characters unlock the next mission."
    } else {
        "Saved to Statistics. Quick play, daily and focused sessions never change the campaign."
    };
    heading(frame, area, title, subtitle, t);
    let cols = area.width / 4;
    for (i, (value, label, color)) in [
        (format!("{:.1}", m.net_wpm), "NET WPM", t.accent),
        (
            format!("{:.1}%", m.accuracy),
            "ACCURACY",
            if m.accuracy >= 94.0 {
                t.success
            } else {
                t.gold
            },
        ),
        (format!("{:.0}s", m.seconds), "ACTIVE TIME", t.fg),
        (format!("+{}", result.xp), "XP EARNED", t.gold),
    ]
    .iter()
    .enumerate()
    {
        let column = Rect::new(area.x + cols * i as u16, area.y, cols, area.height);
        text(frame, column, 5, Line::from(bold(value, *color)));
        text(frame, column, 6, Line::from(span(*label, t.muted)));
    }
    let mut errors: Vec<_> = app
        .session
        .as_ref()
        .map(|s| s.errors.iter().collect())
        .unwrap_or_default();
    errors.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let feedback = if errors.is_empty() {
        "No mistakes this session.".into()
    } else {
        format!(
            "Most missed: {}. Focused practice trains them.",
            errors
                .iter()
                .take(3)
                .map(|(key, _)| if key.as_str() == " " {
                    "space".into()
                } else {
                    key.to_string()
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    text(
        frame,
        area,
        9,
        Line::from(bold(
            if m.net_wpm > app.previous_best && app.profile.sessions > 1 {
                "A new personal best."
            } else {
                "Keys to practice"
            },
            t.fg,
        )),
    );
    text(frame, area, 10, Line::from(span(feedback, t.muted)));
    if area.height > 15 {
        text(
            frame,
            area,
            12,
            Line::from(span(
                format!(
                    "{:.1} gross wpm   /   {} corrections   /   {} consistency",
                    m.gross_wpm,
                    m.corrections,
                    if m.seconds >= 10.0 {
                        format!("{:.0}%", m.consistency)
                    } else {
                        "--".into()
                    }
                ),
                t.muted,
            )),
        );
        let fraction = if app.settings.reduced_motion {
            1.0
        } else {
            (now.saturating_sub(app.result_at).as_secs_f64() / 0.5).min(1.0)
        };
        track(
            frame,
            area,
            15,
            app.profile.completed_missions as f64 / 15.0 * fraction,
            format!("{} / 15 campaign missions", app.profile.completed_missions),
            t,
            app.settings.ascii,
        );
    }
}
fn statistics(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Practice history",
        "Stored only on this device. The last 100 sessions are kept.",
        t,
    );
    if app.profile.sessions == 0 {
        text(frame, area, 6, Line::from(bold("No sessions yet.", t.fg)));
        text(
            frame,
            area,
            8,
            Line::from(span(
                "Finish a practice to see speed and accuracy here.",
                t.muted,
            )),
        );
        return;
    }
    text(
        frame,
        area,
        4,
        Line::from(vec![
            bold(format!("{}", app.profile.sessions), t.accent),
            span(" sessions    ", t.muted),
            bold(format!("{:.1}", app.profile.best_wpm), t.accent),
            span(" best wpm    ", t.muted),
            bold(format!("{}", app.profile.xp), t.gold),
            span(" total XP", t.muted),
        ]),
    );
    text(
        frame,
        area,
        7,
        Line::from(bold(
            "RECENT SESSIONS           WPM      ACCURACY       XP",
            t.muted,
        )),
    );
    for (i, record) in app
        .profile
        .history
        .iter()
        .rev()
        .skip(app.history_offset)
        .take(usize::from(area.height.saturating_sub(9)))
        .enumerate()
    {
        text(
            frame,
            area,
            9 + i as u16,
            Line::from(vec![span(
                format!(
                    "{:<22} {:>6.1}       {:>5.1}%     +{}",
                    format!(
                        "{} / {}",
                        if record.daily_day.is_some() {
                            "Daily"
                        } else if record.focused {
                            "Focused"
                        } else if record.campaign {
                            "Campaign"
                        } else {
                            "Quick"
                        },
                        LEVELS[record.level.min(4)].name
                    ),
                    record.metrics.net_wpm,
                    record.metrics.accuracy,
                    record.xp
                ),
                t.fg,
            )]),
        );
    }
}
/// A mistake is a filled block in the theme's error color, distinct from the caret
/// (accent block or underline) and from correct text. Mono falls back to modifiers.
fn mistake_style(app: &App, t: Theme) -> Style {
    let style = Style::default().add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
    if t.error != Color::Reset {
        style.fg(t.bg).bg(t.error)
    } else if app.settings.underline_caret {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    }
}
fn caret_style(app: &App, t: Theme) -> Style {
    let shape = if app.settings.underline_caret {
        Modifier::UNDERLINED
    } else {
        Modifier::REVERSED
    };
    Style::default()
        .fg(t.accent)
        .add_modifier(shape | Modifier::BOLD)
}

fn type_sample(app: &App, t: Theme) -> Line<'static> {
    Line::from(vec![
        bold("find ", t.success),
        Span::styled("y", mistake_style(app, t)),
        bold("o", t.success),
        Span::styled("u", caret_style(app, t)),
        span("r rhythm", t.muted),
    ])
}

fn settings_sample(frame: &mut Frame, area: Rect, app: &App, section: usize, t: Theme) {
    frame.render_widget(Block::default().style(Style::default().bg(t.panel)), area);
    let inner = Rect::new(area.x + 2, area.y, area.width - 4, area.height);
    match section {
        0 => {
            text(frame, inner, 0, Line::from(bold("Typing preview", t.fg)));
            if area.width >= 48 && area.height >= 14 {
                next_key_guide(frame, Rect::new(inner.x, inner.y + 1, 34, 12), 'y', t);
                text(frame, inner, 13, type_sample(app, t));
                return;
            }
            let (lid, base, edge) = if app.settings.ascii {
                ("+---+", "+---+", "|")
            } else {
                ("╭───╮", "╰───╯", "│")
            };
            for (i, key) in ["T", "Y", "U"].iter().enumerate() {
                let cap = Rect::new(inner.x + 6 * i as u16, inner.y + 3, 5, 3);
                let ink = if *key == "Y" { t.brand } else { t.muted };
                text(frame, cap, 0, Line::from(span(lid, ink)));
                let face = bold(format!("{edge} {key} {edge}"), ink);
                text(
                    frame,
                    cap,
                    1,
                    Line::from(if *key == "Y" { face.reversed() } else { face }),
                );
                text(frame, cap, 2, Line::from(span(base, ink)));
            }
            text(frame, inner, 7, type_sample(app, t));
        }
        1 => {
            text(frame, inner, 1, Line::from(bold("Key sound", t.fg)));
            text(
                frame,
                inner,
                3,
                Line::from(bold(app.settings.sound.profile.label(), t.brand)),
            );
            text(frame, inner, 4, Line::from(span("Volume", t.muted)));
            track(
                frame,
                inner,
                5,
                f64::from(app.settings.sound.volume) / f64::from(MAX_SOUND_VOLUME),
                format!("{}%", app.settings.sound.volume),
                t,
                app.settings.ascii,
            );
            frame.render_widget(
                Paragraph::new(app.audio_status)
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(t.muted)),
                Rect::new(inner.x, inner.y + 6, inner.width, inner.height - 6),
            );
        }
        2 => {
            text(frame, inner, 1, Line::from(bold("Practice patterns", t.fg)));
            let patterns = crate::content::focus_patterns(&app.profile.errors);
            for (i, (pattern, count)) in patterns.iter().take(3).enumerate() {
                let note = if *count == 0 {
                    "starter".into()
                } else {
                    format!("{count} errors")
                };
                text(
                    frame,
                    inner,
                    3 + i as u16,
                    Line::from(vec![
                        bold(format!("{pattern:<6}"), t.brand),
                        span(note, t.muted),
                    ]),
                );
            }
        }
        _ => {}
    }
}

fn settings(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    let spacious = area.height >= 24;
    let words = [("SETTINGS", t.fg)];
    let tab_y = if area.height >= 26 && !app.settings.ascii && title_fits(&words, area) {
        display_title(frame, area, &words, 2, false, t) + 1
    } else if area.height >= 15 {
        text(frame, area, 0, Line::from(bold("Practice settings", t.fg)));
        2
    } else {
        0
    };
    let section = settings_section(app.selection);
    let (_, start, end) = SETTINGS_SECTIONS[section];
    let tabs = SETTINGS_SECTIONS
        .iter()
        .enumerate()
        .map(|(i, (name, _, _))| {
            let label = format!(" {} {name} ", i + 1);
            if i == section {
                bold(label, t.brand).reversed()
            } else {
                span(label, t.muted)
            }
        })
        .collect::<Vec<_>>();
    text(frame, area, tab_y, Line::from(tabs));
    let on = |value| if value { "on" } else { "off" };
    let values = [
        ("Theme", app.settings.theme.label().to_owned()),
        ("High contrast", on(app.settings.high_contrast).into()),
        ("Reduced motion", on(app.settings.reduced_motion).into()),
        ("ASCII symbols", on(app.settings.ascii).into()),
        (
            "Typing caret",
            if app.settings.underline_caret {
                "underline"
            } else {
                "block"
            }
            .into(),
        ),
        ("Sidebar", on(app.settings.layout.sidebar).into()),
        ("Key sounds", on(app.settings.sound.enabled).into()),
        ("Sound profile", app.settings.sound.profile.label().into()),
        ("Volume", format!("{}%", app.settings.sound.volume)),
        ("Preview sound", "enter to play".into()),
        (
            "Adaptive prompts",
            on(app.settings.practice.adaptive).into(),
        ),
        (
            "Reload JSON",
            app.settings.keybindings.reload_config.clone(),
        ),
    ];
    let details = [
        "Choose a theme. Changes save automatically.",
        "Increase the contrast of secondary text.",
        "Skip the results progress animation.",
        "Use plain characters for limited terminal fonts.",
        "A steady block or underline. No blinking.",
        "Practice summary in browse views at 112+ columns.",
        app.audio_status,
        "EG Oreo and NK Cream use local recordings.",
        "0% mutes. Above 100% boosts the key sound.",
        app.audio_status,
        "Favor the letters and pairs you find difficult.",
        "Validate and apply your file without restarting.",
    ];
    let row_start = tab_y + 2;
    let sample_height = (area.height - row_start - 1).min(if section == 0 && area.width >= 116 {
        15
    } else {
        10
    });
    let split = section != 3 && area.width >= 68 && sample_height >= 8;
    let list_width = if split {
        area.width * 55 / 100
    } else {
        area.width.min(76)
    };
    let stride = if spacious { 2 } else { 1 };
    if split {
        let panel_x = list_width + 3;
        settings_sample(
            frame,
            Rect::new(
                area.x + panel_x,
                area.y + row_start,
                area.width - panel_x,
                sample_height,
            ),
            app,
            section,
            t,
        );
    }

    for (i, (label, value)) in values.iter().enumerate().take(end).skip(start) {
        let y = row_start + (i - start) as u16 * stride;
        let row = Rect::new(area.x, area.y + y, list_width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                bold(if app.selection == i { "> " } else { "  " }, t.brand),
                span(*label, t.fg),
            ]))
            .style(Style::default().bg(if app.selection == i {
                t.selection
            } else {
                t.bg
            })),
            row,
        );
        frame.render_widget(
            Paragraph::new(value.clone())
                .alignment(Alignment::Right)
                .style(Style::default().fg(if app.selection == i { t.fg } else { t.muted })),
            Rect::new(row.x + 17, row.y, row.width - 18, 1),
        );
    }
    let after_rows = row_start + (end - start - 1) as u16 * stride + 1;
    if section == 3 {
        let y = after_rows + 1;
        text(frame, area, y, Line::from(bold("Configuration file", t.fg)));
        frame.render_widget(
            Paragraph::new(app.config_path.as_str())
                .style(Style::default().fg(t.muted))
                .wrap(Wrap { trim: false }),
            Rect::new(area.x, area.y + y + 1, area.width, area.height - y - 1),
        );
    } else {
        if section == 0 && !split && after_rows + 2 < area.height {
            text(frame, area, after_rows + 1, type_sample(app, t));
        }
        if section == 1 && !split && app.selection != 6 && app.selection != 9 {
            text(
                frame,
                area,
                after_rows + 1,
                Line::from(span(app.audio_status, t.muted)),
            );
        }
        let detail_y = if split {
            (after_rows.max(row_start + sample_height) + 1).min(area.height - 1)
        } else {
            area.height - 1
        };
        let detail = details[app.selection.min(11)];
        frame.render_widget(
            Paragraph::new(detail)
                .style(Style::default().fg(t.muted))
                .wrap(Wrap { trim: true }),
            Rect::new(
                area.x,
                area.y + detail_y,
                area.width,
                area.height.saturating_sub(detail_y),
            ),
        );
    }
}
fn help(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Keys and shortcuts",
        "Menu, commands, and typing shortcuts.",
        t,
    );
    let rows = [
        ("Tab".to_owned(), "Open navigation menu; pause typing"),
        (
            app.settings.keybindings.menu.clone(),
            "Open navigation menu",
        ),
        (
            app.settings.keybindings.commands.clone(),
            "Search commands; press again to close",
        ),
        (
            app.settings.keybindings.restart.clone(),
            "Restart; confirm an unfinished attempt",
        ),
        (
            app.settings.keybindings.reload_config.clone(),
            "Reload JSON; keep the session paused",
        ),
        ("Esc".into(), "Pause / close commands / return to menu"),
        (
            "Backspace".into(),
            "Correct text; previous errors still count",
        ),
        ("Ctrl+C".into(), "Exit and restore the terminal"),
    ];
    let start = if area.height < 14 { 3 } else { 4 };
    for (i, (key, detail)) in rows.iter().enumerate() {
        text(
            frame,
            area,
            start + i as u16,
            Line::from(vec![bold(format!("{key:<10}"), t.fg), span(*detail, t.fg)]),
        );
    }
    if area.height >= 17 {
        text(
            frame,
            area,
            14,
            Line::from(span(
                "Campaign: 94% accuracy + 20 correct characters.",
                t.muted,
            )),
        );
        text(
            frame,
            area,
            16,
            Line::from(span(
                "Paste and reported repeat events are ignored.",
                t.muted,
            )),
        );
    }
}

fn focused(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Practice the keys you miss most",
        "Choose a pattern for 50 words of focused practice.",
        t,
    );
    let heatmap = area.height >= 19;
    if heatmap {
        for (row, keys) in ["qwertyuiop", "asdfghjkl", "zxcvbnm"].iter().enumerate() {
            let mut spans = vec![span(" ".repeat(row * 2), t.muted)];
            for key in keys.chars() {
                let count = app
                    .profile
                    .errors
                    .get(&key.to_string())
                    .copied()
                    .unwrap_or(0);
                let color = if count >= 5 {
                    t.error
                } else if count > 0 {
                    t.gold
                } else {
                    t.secondary
                };
                spans.push(Span::styled(
                    format!(" [{key}]"),
                    Style::default()
                        .fg(color)
                        .bg(t.panel)
                        .add_modifier(if count > 0 {
                            Modifier::BOLD | Modifier::UNDERLINED
                        } else {
                            Modifier::empty()
                        }),
                ));
            }
            text(frame, area, 4 + row as u16, Line::from(spans));
        }
        text(
            frame,
            area,
            8,
            Line::from(span(
                "Underlined keys need practice. QWERTY reference layout.",
                t.muted,
            )),
        );
    }
    let start = if heatmap { 10 } else { 4 };
    let patterns = crate::content::focus_patterns(&app.profile.errors);
    let capacity = usize::from(area.height.saturating_sub(start)).max(1);
    let top = app.selection.saturating_sub(capacity - 1);
    for (i, (pattern, count)) in patterns.iter().enumerate().skip(top).take(capacity) {
        text(
            frame,
            area,
            start + (i - top) as u16,
            Line::from(vec![
                bold(if app.selection == i { "> " } else { "  " }, t.brand),
                bold(format!("{pattern:<5}"), t.fg),
                span(
                    if *count == 0 {
                        "Starter pattern".into()
                    } else {
                        format!("{count} recent errors")
                    },
                    t.muted,
                ),
                span("   50 words", t.muted),
            ]),
        );
    }
}
fn daily(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    heading(
        frame,
        area,
        "Today's challenge",
        "60 seconds of Momentum words. Everyone gets the same words until 00:00 UTC.",
        t,
    );
    text(
        frame,
        area,
        5,
        Line::from(vec![
            bold(format!("CHALLENGE #{}", app.today), t.brand),
            span("   Retry as often as you like", t.muted),
        ]),
    );
    text(
        frame,
        area,
        7,
        Line::from(bold(
            match app.profile.daily_best.get(&app.today) {
                Some(best) => format!("Your best today: {best:.1} WPM"),
                None => "No attempt yet today.".into(),
            },
            t.fg,
        )),
    );
    text(
        frame,
        area,
        9,
        Line::from(span("Press Enter to try today's words.", t.fg)),
    );
    if area.height > 14 {
        text(
            frame,
            area,
            13,
            Line::from(span(
                "Same words for everyone, not adapted to your mistakes.",
                t.muted,
            )),
        );
    }
}
fn dialog_area(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(4));
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}
fn command_palette(frame: &mut Frame, area: Rect, app: &App, t: Theme) {
    let p = app.palette.as_ref().unwrap();
    let dialog = dialog_area(area, 68, 18);
    frame.render_widget(Clear, dialog);
    frame.render_widget(
        Block::default().style(Style::default().bg(t.panel).fg(t.fg)),
        dialog,
    );
    let inside = Rect::new(
        dialog.x + 2,
        dialog.y + 1,
        dialog.width - 4,
        dialog.height - 2,
    );
    text(
        frame,
        inside,
        0,
        Line::from(vec![
            bold("Commands", t.fg),
            span("  /  type to find an action", t.muted),
        ]),
    );
    frame.render_widget(
        Block::default().style(Style::default().bg(t.bg)),
        Rect::new(inside.x, inside.y + 2, inside.width, 1),
    );
    text(
        frame,
        inside,
        2,
        Line::from(vec![
            bold("> ", t.accent),
            span(
                if p.query.is_empty() {
                    "Search commands...".into()
                } else {
                    p.query.clone()
                },
                if p.query.is_empty() { t.muted } else { t.fg },
            ),
            bold("_", t.accent),
        ]),
    );
    let actions = app.commands();
    let capacity = usize::from(inside.height.saturating_sub(6)).max(1);
    let top = p.selection.saturating_sub(capacity - 1);
    visible_range(frame, inside, top, capacity, actions.len(), t);
    if actions.is_empty() {
        text(
            frame,
            inside,
            4,
            Line::from(span("No matching commands. Try another word.", t.muted)),
        );
    }
    for (i, action) in actions.iter().enumerate().skip(top).take(capacity) {
        let selected = i == p.selection;
        let row = Rect::new(inside.x, inside.y + 4 + (i - top) as u16, inside.width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                bold(if selected { "> " } else { "  " }, t.brand),
                span(action.label(), t.fg),
            ]))
            .style(Style::default().bg(if selected { t.selection } else { t.panel })),
            row,
        );
        if inside.width >= 60 {
            let key = match action {
                Action::Menu => "tab",
                Action::Home => "",
                Action::Reload => app.settings.keybindings.reload_config.as_str(),
                Action::Restart => app.settings.keybindings.restart.as_str(),
                Action::Quit => "ctrl+c",
                _ => "",
            };
            frame.render_widget(
                Paragraph::new(key.to_owned())
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(t.muted)),
                Rect::new(row.x + row.width - 9, row.y, 8, 1),
            );
        }
    }
    text(
        frame,
        inside,
        inside.height - 1,
        Line::from(span("Up/Down choose   Enter run   Esc close", t.muted)),
    );
}
fn confirm_restart(frame: &mut Frame, area: Rect, t: Theme) {
    let dialog = dialog_area(area, 60, 10);
    frame.render_widget(Clear, dialog);
    frame.render_widget(
        Block::default().style(Style::default().bg(t.panel).fg(t.fg)),
        dialog,
    );
    let inside = Rect::new(
        dialog.x + 2,
        dialog.y + 1,
        dialog.width - 4,
        dialog.height - 2,
    );
    text(
        frame,
        inside,
        0,
        Line::from(bold("Replace your paused session?", t.fg)),
    );
    text(
        frame,
        inside,
        2,
        Line::from(span(
            "Replacing clears the words typed in this attempt.",
            t.fg,
        )),
    );
    text(
        frame,
        inside,
        3,
        Line::from(span("Saved progress is kept.", t.muted)),
    );
    text(
        frame,
        inside,
        6,
        Line::from(vec![
            bold("Enter ", t.brand),
            span("replace   ", t.fg),
            bold("Esc ", t.fg),
            span("keep session", t.fg),
        ]),
    );
}
