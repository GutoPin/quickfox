# Architecture

One Cargo package exposes a small library and the `quickfox` executable. Rust 1.88 is
the minimum supported version. Cargo.lock pins the complete dependency graph.

| Module | Responsibility |
|---|---|
| `src/main.rs` | Clap CLI, terminal lifecycle, event source, monotonic clock, save orchestration |
| `src/app.rs` | Screen transitions, key routing, starting/completing sessions |
| `src/game/` | Pure session state, scoring, campaign definitions |
| `src/config.rs` | Editable settings, validation, color/key parsing, and legacy defaults |
| `src/content.rs` | Embedded corpus and deterministic adaptive prompt provider |
| `src/persistence.rs` | Versioned JSON, XDG paths, atomic replacement, recovery, profile lock |
| `src/ui/` | Responsive Ratatui rendering and semantic theme tokens |
| `src/audio.rs` | Sound synthesis, local OGG/WAV decoding, bounded key queue, and optional Linux audio worker |

## Decisions

### Native terminal; no services

Quickfox uses Rust, Ratatui, and Crossterm. Ratatui 0.30.2 and
Crossterm 0.29 are compatible as documented in [Ratatui's installation guide](https://ratatui.rs/installation/).
There is no HTTP client, account, browser frontend, remote dictionary, or telemetry.
The application uses the user's existing monospace font.

### One clock, one event source

The event loop owns Crossterm input. Session methods receive monotonic `Duration` values,
so tests control time without sleeping. Wall-clock time is used only for history timestamps
and prompt seeding, never for scoring. Menus block on events; timed play uses 100 ms polls,
and isolated keys trigger immediate redraws. Queued bursts are consumed in batches of
at most 64 events between redraws; navigation, resize, and reload boundaries end a batch. Ratatui writes only changed terminal cells.

Sessions start timing with the first printable ASCII character. Pauses before the first
key do not consume time or shift the eventual start. Events arriving after a deadline
complete the old session without activating controls on the results screen. Focus loss
and undersized terminals pause active play.

### Save at transitions

Input processing performs no disk I/O. App dirty flags request saves after a completed
session or preference change. Saves serialize to a temporary file in the destination
directory, sync it, replace the destination, and sync the directory on Unix. A process
lock prevents concurrent profile writers. Persistent save failures are reported after
terminal restoration; in-memory state is retained during play and retried at exit.

Malformed progress files are preserved with unique recovery names. Configuration errors
are reported without renaming or resetting the editable file. Unsupported schemas and
semantically invalid progression are rejected, not migrated destructively. Schema 1 is
the only current schema; there is no fictional migration from an unpublished format.

### Terminal lifetime

Ratatui initializes raw mode and the alternate screen and installs its restoration panic
hook. A scope guard restores the terminal on normal return and error. The chained panic
hook and guard also disable bracketed paste and focus reporting. Ctrl+C is handled as a
key event in raw mode. Forced process termination cannot guarantee restoration.

### English input boundary

All embedded content is printable ASCII, including straight quotes and punctuation.
Each character occupies one terminal column. This avoids incomplete Unicode grapheme
support while making the v0.1 input contract explicit. Non-ASCII input is ignored;
a future multilingual corpus must change storage, matching, and display-width logic
alongside its tests.

### Adaptive selection

`PromptProvider` decouples prompts from the game. `LocalAdaptiveProvider` uses a seeded
xorshift PRNG and weighted sampling: baseline weight 10, plus matching recent error
weights capped at 30 extra. Individual error contributions cap at 10. Immediate identical
word/sentence repeats are excluded. Level-specific pools preserve short words, common
words, advanced vocabulary, punctuation, and numeric sentences. Stored error counts
decay by 25% each completed session, then receive new errors; counts cap at 1,000.
This is a lightweight practice heuristic, not a validated learning model.

### UI verification

`TestBackend` renders every screen at 60×20, 80×24, 100×30, 120×40, 160×50, and 240×70 with every theme.
Text-token contrast is computed against both theme surfaces. A scrolling regression checks
that the block caret remains visible across hundreds of character positions. Actual
buffer captures can be converted to PNG; terminal font rasterization may differ.

The Unix PTY harness checks restoration of termios flags, alternate-screen exit, and
bracketed-paste cleanup. It also exercises resize and persisted session completion.
These checks do not substitute for manual testing in GNOME Terminal, Kitty, SSH, or Windows.

## v0.2 decisions

- Configuration uses additive schema-1 fields with Serde defaults to preserve v0.1 saves.
  Unknown config keys are rejected to catch typos; profile history remains backward compatible.
- The configuration file is independent of progress and can be selected using `--config`.
  Init emits a JSON schema, show expands defaults, and validate runs the same rules as the app.
- Live reload validates a complete candidate before applying it. The last-read byte snapshot
  detects external edits before a UI settings save; conflicting saves fail instead of replacing
  an in-progress edit. Reload establishes a new snapshot. This is optimistic conflict detection,
  not an atomic locking protocol shared with external editors.
- Navigation owns palette and replacement-confirmation state. A single retained session is
  paused whenever the player visits the menu. Global shortcuts cannot consume plain typing
  characters. Ctrl+C remains a fixed escape hatch.
- Daily challenge identity is the UTC day and current content version. Daily records carry
  that day separately from completion timestamps. Focused practice selects an explicit error
  pattern instead of merely increasing its sampling weight.
- Theme conversion applies custom RGB overrides before terminal color fallback. Monochrome
  and NO_COLOR retain terminal defaults; arbitrary custom colors are syntax-validated but
  cannot be guaranteed to meet contrast guidelines.

## v0.3 navigation and audio

`Screen::Menu` is a full navigation destination. Tab and the configured menu shortcut
pause the active session and open it. The command palette remains an overlay with its
own search, selection, and prior-screen state. Escape from Menu opens Home; other
browsing screens return to Menu. Help renders configured shortcut values.

`SoundEngine` receives only accepted typing, corrections, or explicit sound previews.
It opens no device while disabled. A worker owns the optional dynamically loaded Linux
PulseAudio stream; bounded, non-blocking event submission keeps audio out of the input
path. Sound configuration is additive under schema version 1. See [audio.md](audio.md).

## v0.4 settings and recordings

Settings use four named sections with stable 1–4 shortcuts; Up/Down traverses the
underlying rows. Nightfall uses neutral tokens and a grayscale ANSI256 conversion
that preserves separate ground, panel, and selection shades.

Audio decodes user-installed EG Oreo/NK Cream files on the worker, extracts one known
key segment, downmixes and resamples to 48 kHz, and applies short edge fades. File size,
duration, channel count, rate, and sample values are bounded/validated. Changing the
profile or source directory replaces the worker without blocking the UI. Explicit
preview events survive initialization; ordinary key events older than 150 ms expire.
The app never downloads media. The separate installer records asset provenance locally.
