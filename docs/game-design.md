# Game rules

## Campaign

Fifteen ordered missions, grouped into five levels. Each level uses the same three
mission titles: Warm-up (30 seconds), Pace (45 seconds),
and Endurance (60 seconds). Content difficulty rises through First Keys, Flow,
Momentum, Precision, and Mastery. A mission passes at 94% attempt accuracy and at least
20 retained correct characters. There is no minimum WPM.

Only passing the currently active mission advances the campaign. Replaying earlier
levels awards practice XP but never skips missions. After all fifteen missions, the
final mission remains replayable. Failure leaves the current mission available.
Quick play has no locks and never advances campaign progress.

## Sessions and feedback

Each key is evaluated against the target at the current position. A wrong space is one
wrong character, not a word-skip command. Backspace removes the retained character at
the previous position, without reducing attempt/error counters. Errors in the UI mark
the expected character; a wrong space uses a visible dot or ASCII underscore.

The clock starts with the first printable ASCII key. Focus loss, Esc, or a terminal
smaller than 60×20 pauses the session. Resume is explicit. Timeouts reject late input.
Entering the entire prompt ends the session, including in timed modes. Timed prompts
contain 600 words; word mode contains exactly its selected word count.

Results save once, show the strongest error patterns, and expose retry/next/home.
Restart creates fresh content. Abandoning/restarting never awards partial-session XP.
No sound, global ranking, streak penalty, or external service is present.

Tab, Ctrl+G, and Esc from pause return to Menu without discarding the session. Resume restores
the same prompt and input; time spent navigating does not count. Ctrl+P opens commands
and pauses play. Closing it resumes only if it was opened during typing. Choosing another
mode keeps the session paused. Starting or restarting asks before replacing an unfinished
session. Closing the application still abandons unfinished sessions; in-memory resume is
not an on-disk checkpoint.

## Metrics

See README for the exact WPM, accuracy, consistency, and XP formulas. Retained correct
characters count toward net WPM; historical correct keypresses count toward accuracy and
five-second consistency windows. These intentionally answer different questions.
Before the first second, WPM is zero. Before any key, displayed accuracy is 100% as a
neutral starting state. Consistency needs two full five-second windows; idle windows
are included, partial final windows are excluded.

## Content

The executable embeds 400 common words, 95 advanced words, and 18 original sentences.
First Keys restricts common words to at most four characters. Flow uses common words
up to seven characters. Momentum adds the advanced pool. Precision uses sentences
without numbers. Mastery includes all sentences, including numeric/symbol practice.

No source corpus was copied from another typing application. See
`assets/words/LICENSES.md` for provenance and pending licensing status.

## Daily challenges and focused practice

Daily prompts use a fixed generator seed derived from the UTC day, with Momentum content
and a 60-second duration. Adaptive error weighting is disabled for this mode. Replays on
the same app/content version receive the same prompt. Local bests are indexed by UTC day;
the latest 366 days with attempts are retained. Clock changes can change which challenge
is considered today. Daily and focused sessions earn normal practice XP but never unlock
campaign missions. Prompts may change if a future version changes the corpus or generator.

Focused practice lists up to eight recent alphabetic letter/pair errors in descending
frequency. A fresh profile offers th, st, and ou. Each selection generates 50 words from
the common and advanced pools containing that pattern. Unknown patterns with no matches
fall back to normal common-word practice. The QWERTY heatmap is a reference visualization,
not a keyboard-layout remapping; operating-system keyboard layouts still govern input.

Statistics can scroll through all retained sessions. Prefixes C/Q/F/D identify campaign,
quick, focused, and daily sessions respectively.
