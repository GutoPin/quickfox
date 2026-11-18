# Optional typing sounds

Key sounds default to off. Open Settings → 2 Sound to enable them, choose a profile,
adjust volume (0–200%), and preview. JSON uses `sound.enabled`, `sound.profile`,
`sound.volume`, and optional `sound.directory`. 100% is unity gain. The boost applies to synthesized and recorded profiles, with peak limiting before PCM output. Legacy configurations remain silent.

## Profiles and installation

Creamy, Soft, and Typewriter (`creamy`, `soft`, `typewriter`) are original procedural
waveforms using damped oscillators, deterministic noise, and attack/release envelopes.
They need no media files.

EG Oreo and NK Cream (`eg-oreo`, `nk-cream`) use recordings served by
[Somewhere](https://www.somewheretypingtest.com/). Its public sound definitions identify
these two packs. This implementation uses a representative Key A segment from each
recording, not the site's full per-key mapping: EG Oreo at 11,171 ms for 109 ms, and
NK Cream at 0 ms for 185 ms.

The [EG Oreo listing](https://mechvibes.com/sound-packs/eg-oreo) and
[NK Cream listing](https://mechvibes.com/sound-packs/nk-cream-original-by-ryan) mark them
for personal use. Recordings are not included in this repository or distributable
archive. Run the separate installer for local personal use:

```bash
python3 scripts/install_reference_sounds.py
```

It downloads `oreo.ogg` and `nk-cream.wav` from Somewhere into
`$XDG_DATA_HOME/quickfox/sounds/reference`, defaulting to
`~/.local/share/quickfox/sounds/reference`, and writes provenance and SHA-256 hashes.
The game does not access the network. The installer needs Python 3 and network access.
For a custom folder, pass `--directory PATH` and set `sound.directory` to its absolute
path. This location is independent of the CLI `--data-dir` profile isolation option.
Missing or invalid media produces a visible Settings message; typing remains usable.

## Playback

On Linux, a worker loads `libpulse-simple.so.0` and connects to the user's audio service.
PulseAudio, PipeWire's compatibility service, and WSLg can provide this interface.
No audio development headers are required. macOS and Windows currently run silently.

The backend uses the [PulseAudio Simple API](https://freedesktop.org/software/pulseaudio/doxygen/simple.html).
Blocking writes and media decoding stay on the worker. Five-millisecond sample blocks
and a small requested server buffer keep feedback responsive; actual latency depends
on the audio stack and is not an end-to-end latency guarantee.

Recordings are validated, downmixed, and resampled to 48 kHz with short edge fades.
File reads are limited to 16 MiB and decoding to 60 seconds. The mixer limits output
and supports overlapping keys. A bounded queue drops excess/old typing events; explicit
previews wait through initialization. Disabling sound signals the worker without waiting
for device cleanup on the UI thread. Toggle off/on to retry after correcting the media
files or system audio service.

## Verification

`cargo test --locked --test audio` checks synthesized waveforms, silent defaults,
accepted-key gating, invalid/missing/incomplete recordings, and stereo WAV conversion.
It opens no audio device and does not depend on third-party recordings. The PTY suite
checks unavailable-audio fallback, mute, persistence, and responsive typing.

After installing both packs, explicitly check the system audio path:

```bash
cargo run --locked --example check_audio
```

This submits one sound per profile. Successful submission does not establish that a
human heard the sound or that speakers/headphones are unmuted.
