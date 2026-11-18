# Changelog

## Unreleased

- Named the project Quickfox, with an italic pixel wordmark on Home and pixel titles
  for Menu and Settings.
- Mistakes are filled blocks in each theme's error color, distinct from the cursor.
- Plainer interface text; mission names are now Warm-up, Pace and Endurance.
- Theme colors have fixed roles: brand marks the selection and main action, shortcut
  keys use the text color, and state colors appear only for states.

## 0.4.0 — Neutral workspace and recorded key sounds

- Rebuilt Nightfall around black, white, and gray, including neutral 256-color shading.
- Replaced large browsing panels with a flat grouped menu and aligned setting rows.
- Split Settings into Appearance, Sound, Practice, and Config, selected with 1–4.
- Added locally installed EG Oreo and NK Cream recordings used by Somewhere; personal-use files are excluded from distribution.
- Added bounded OGG/WAV decoding, resampling, explicit preview events, and missing-pack feedback.
- Preserved Tab for navigation, Ctrl+P for commands, optional palettes, and JSON customization.

## 0.3.0 — A consistent workspace and optional key sounds

- Separated the navigation menu (Tab / Ctrl+G) from searchable commands (Ctrl+P).
- Unified work surfaces, selection rows, search, settings, and the focused typing canvas.
- Added Nord, Dracula, Catppuccin, Tokyo Night, Gruvbox, Rose Pine, Solarized Light, and Forest palettes.
- Added optional Creamy, Soft, and Typewriter sounds, volume, preview, and safe silent fallback. Playback is available through the Linux PulseAudio-compatible system backend.
- Made Settings scroll on small terminals and show theme/audio previews on wide terminals.
- Corrected global reload/restart behavior while commands are open; help reflects configured shortcuts.
- Added navigation, sound, settings visibility, and unavailable-audio regression checks.

## 0.2.1 — Reference-led redesign

- Rebuilt Home around a centered block wordmark and a single launch panel.
- Changed Nightfall to neutral black and charcoal with blue, teal, violet, and amber accents.
- Added Tab to open the searchable menu from Home; existing shortcuts and paused sessions remain available.
- Removed filled header/footer bars and made the browsing sidebar opt-in for new configurations.
- Preserved JSON customization, existing preferences, save compatibility, and every practice mode.

## 0.2.0 — Navigation and personalization

- Redesigned the terminal with colored navigation, shaded panels, and persistent shortcuts.
- Added searchable commands, a return-to-menu shortcut, resumable in-memory sessions,
  and confirmation before replacing an unfinished session.
- Added Ember and Custom themes with JSON color tokens, safe global keybindings,
  prompt layout controls, and configurable practice defaults.
- Added config init/show/path/validate commands and live reload with external-edit protection.
- Added daily UTC challenges, local daily bests, focused word practice, and an error heatmap.
- Preserved v0.1 save compatibility; added navigation/config/challenge regression tests.
- Batched queued terminal input between redraws without dropping evaluated key events.
- Updated captures, documentation, and CI matrix configuration.

## 0.1.0 — Local implementation

- Added the native Rust terminal game with guided onboarding and immediate offline play.
- Added five campaign levels, fifteen missions, and timed/word-count quick play.
- Added transparent typing metrics, adaptive prompts, XP, and local session history.
- Added Nightfall, Orchid Circuit, Paperwave, Mono, and accessibility preferences.
- Added local versioned saves, recovery backups, export, and terminal diagnostics.
- Added deterministic UI captures, engine/storage/CLI tests, and Unix PTY smoke checks.
- Added CI checks and a Linux archive with checksums.
