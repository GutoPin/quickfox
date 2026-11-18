//! Optional, non-blocking key feedback. No device is opened while sound is off.
use crate::config::{SoundProfile, SoundSettings};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU32, Ordering},
        mpsc::{self, SyncSender},
    },
    time::Instant,
};

pub const SAMPLE_RATE: u32 = 48_000;

/// Original synthesized key sounds; no downloaded or licensed recordings.
pub fn synthesize(profile: SoundProfile) -> Vec<f32> {
    let (duration, tone, decay, noise, gain) = match profile {
        SoundProfile::Creamy => (0.065, 185.0, 65.0, 0.22, 0.48),
        SoundProfile::Soft => (0.035, 420.0, 115.0, 0.10, 0.24),
        SoundProfile::Typewriter => (0.085, 1550.0, 90.0, 0.65, 0.43),
        SoundProfile::EgOreo | SoundProfile::NkCream => return Vec::new(),
    };
    let count = (duration * SAMPLE_RATE as f32) as usize;
    let mut random = 0x1234_abcd_u32;
    let mut lowpass = 0.0;
    (0..count)
        .map(|i| {
            let time = i as f32 / SAMPLE_RATE as f32;
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            let white = random as f32 / u32::MAX as f32 * 2.0 - 1.0;
            lowpass += 0.18 * (white - lowpass);
            let texture = if profile == SoundProfile::Typewriter {
                white
            } else {
                lowpass
            };
            let attack = (time / 0.0015).min(1.0);
            let fade = ((count - i - 1) as f32 / 240.0).min(1.0);
            let body = (time * tone * std::f32::consts::TAU).sin() * (-time * decay).exp();
            let click = texture * (-time * decay * 1.8).exp();
            let release = if profile == SoundProfile::Typewriter && time > 0.024 {
                ((time - 0.024) * 2300.0 * std::f32::consts::TAU).sin()
                    * (-(time - 0.024) * 180.0).exp()
                    * 0.24
            } else {
                0.0
            };
            (body * (1.0 - noise) + click * noise + release) * gain * attack * fade
        })
        .collect()
}

/// User-installed recordings stay outside the executable and distributable archive.
pub fn reference_directory() -> std::path::PathBuf {
    crate::persistence::project_dirs()
        .map(|dirs| dirs.data_dir().join("sounds/reference"))
        .unwrap_or_else(|| std::path::PathBuf::from("sounds/reference"))
}

pub fn load_sound(
    profile: SoundProfile,
    directory: Option<&std::path::Path>,
) -> anyhow::Result<Vec<f32>> {
    use anyhow::{Context, ensure};
    use std::io::{Cursor, Read};
    let (filename, start_ms, duration_ms) = match profile {
        SoundProfile::EgOreo => ("oreo.ogg", 11171, 109),
        SoundProfile::NkCream => ("nk-cream.wav", 0, 185),
        _ => return Ok(synthesize(profile)),
    };
    let root = directory
        .map(std::path::Path::to_owned)
        .unwrap_or_else(reference_directory);
    let file =
        std::fs::File::open(root.join(filename)).context("Reference sound is not installed")?;
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 16 * 1024 * 1024, "Sound file exceeds 16 MiB");
    let (samples, channels, rate) = if filename.ends_with(".ogg") {
        let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes))?;
        let channels = usize::from(reader.ident_hdr.audio_channels);
        let rate = reader.ident_hdr.audio_sample_rate;
        ensure!(
            (1..=2).contains(&channels) && (8000..=192000).contains(&rate),
            "Unsupported sound format"
        );
        let mut samples = Vec::new();
        while let Some(packet) = reader.read_dec_packet_itl()? {
            ensure!(
                samples.len() + packet.len() <= rate as usize * channels * 60,
                "Sound exceeds 60 seconds"
            );
            samples.extend(packet.into_iter().map(|value| value as f32 / 32768.0));
        }
        (samples, channels, rate)
    } else {
        let mut reader = hound::WavReader::new(Cursor::new(bytes))?;
        let spec = reader.spec();
        let channels = usize::from(spec.channels);
        ensure!(
            (1..=2).contains(&channels) && (8000..=192000).contains(&spec.sample_rate),
            "Unsupported sound format"
        );
        ensure!(
            (1..=32).contains(&spec.bits_per_sample),
            "Unsupported sample depth"
        );
        ensure!(
            reader.len() as usize <= spec.sample_rate as usize * channels * 60,
            "Sound exceeds 60 seconds"
        );
        let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
            reader.samples::<f32>().collect::<Result<_, _>>()?
        } else {
            let divisor = (1_u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|value| value.map(|v| v as f32 / divisor))
                .collect::<Result<_, _>>()?
        };
        (samples, channels, spec.sample_rate)
    };
    let start = start_ms * rate as usize / 1000;
    let end = (start_ms + duration_ms) * rate as usize / 1000;
    ensure!(
        samples.len() / channels >= end,
        "Reference sound is incomplete"
    );
    let mono: Vec<f32> = samples[start * channels..end * channels]
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    ensure!(mono.iter().all(|v| v.is_finite()), "Invalid audio samples");
    let length = duration_ms * SAMPLE_RATE as usize / 1000;
    Ok((0..length)
        .map(|i| {
            let position = i as f64 * rate as f64 / SAMPLE_RATE as f64;
            let index = position as usize;
            let fraction = (position - index as f64) as f32;
            let a = mono[index.min(mono.len() - 1)];
            let b = mono[(index + 1).min(mono.len() - 1)];
            let fade = (i.min(length - i - 1) as f32 / 24.0).min(1.0);
            (a + (b - a) * fraction).clamp(-1.0, 1.0) * fade
        })
        .collect())
}

#[cfg(any(target_os = "linux", test))]
fn output_sample(mix: f32, gain: f32) -> i16 {
    // Preserve the original mix up to unity. Boost quiet samples linearly
    // and ease peaks into the remaining PCM headroom.
    let level = mix.clamp(-0.9, 0.9) * gain;
    let excess = (level.abs() - 0.9).max(0.0);
    let limited = level.abs().min(0.9) + excess / (1.0 + excess * 10.0);
    (level.signum() * limited * i16::MAX as f32) as i16
}

#[cfg(test)]
mod tests {
    use super::output_sample;

    #[test]
    fn boost_doubles_quiet_samples_and_limits_overlapping_peaks() {
        assert_eq!(output_sample(0.3, 0.0), 0);
        assert_eq!(output_sample(0.9, 1.0), (0.9 * i16::MAX as f32) as i16);
        assert!((output_sample(0.2, 2.0) - 2 * output_sample(0.2, 1.0)).abs() <= 1);
        let peak = output_sample(4.0, 2.0);
        assert!(peak > output_sample(4.0, 1.0) && peak < i16::MAX);
        assert_eq!(output_sample(-4.0, 2.0), -peak);
    }
}

enum SoundEvent {
    Key(Instant),
    Preview,
}
impl SoundEvent {
    fn is_fresh(&self) -> bool {
        match self {
            Self::Key(at) => at.elapsed().as_millis() <= 150,
            Self::Preview => true,
        }
    }
}

pub struct SoundEngine {
    sender: Option<SyncSender<SoundEvent>>,
    settings: SoundSettings,
    state: Arc<AtomicU32>,
    status: Arc<AtomicU8>,
}
impl Default for SoundEngine {
    fn default() -> Self {
        Self {
            sender: None,
            settings: SoundSettings::default(),
            state: Arc::new(AtomicU32::new(0)),
            status: Arc::new(AtomicU8::new(0)),
        }
    }
}
impl SoundEngine {
    pub fn configure(&mut self, settings: &SoundSettings) {
        if &self.settings == settings {
            return;
        }
        let restart = self.settings.profile != settings.profile
            || self.settings.directory != settings.directory;
        if restart {
            self.state.store(0, Ordering::Relaxed);
            self.sender = None;
        }
        self.settings = settings.clone();
        if !settings.enabled {
            self.state.store(0, Ordering::Relaxed);
            self.sender = None;
            self.status.store(0, Ordering::Relaxed);
            return;
        }
        let profile = SoundProfile::ALL
            .iter()
            .position(|p| *p == settings.profile)
            .unwrap_or(0) as u32;
        let packed = 1 | profile << 1 | u32::from(settings.volume) << 8;
        if self.sender.is_some() {
            self.state.store(packed, Ordering::Relaxed);
            return;
        }
        self.state = Arc::new(AtomicU32::new(packed));
        self.status = Arc::new(AtomicU8::new(1));
        let (sender, receiver) = mpsc::sync_channel(8);
        let state = self.state.clone();
        let status = self.status.clone();
        let directory = settings.directory.clone();
        match std::thread::Builder::new()
            .name("key-sounds".into())
            .spawn(move || {
                worker(receiver, state, status, directory);
            }) {
            Ok(_) => self.sender = Some(sender),
            Err(_) => self.status.store(3, Ordering::Relaxed),
        }
    }
    pub fn play(&self) {
        if self.settings.enabled
            && self.settings.volume > 0
            && let Some(sender) = &self.sender
        {
            // Drop excess sound events instead of delaying input or playing a backlog.
            let _ = sender.try_send(SoundEvent::Key(Instant::now()));
        }
    }
    pub fn preview(&self) {
        if self.settings.enabled
            && self.settings.volume > 0
            && let Some(sender) = &self.sender
        {
            let _ = sender.try_send(SoundEvent::Preview);
        }
    }
    pub fn is_starting(&self) -> bool {
        self.status.load(Ordering::Relaxed) == 1
    }
    pub fn status(&self) -> &'static str {
        match self.status.load(Ordering::Relaxed) {
            0 => "Sound is off",
            1 => "Connecting to system audio...",
            2 => "System audio ready",
            4 => "Sound pack missing or invalid; run the installer",
            _ => "Audio unavailable; typing stays silent",
        }
    }
}
impl Drop for SoundEngine {
    fn drop(&mut self) {
        self.state.store(0, Ordering::Relaxed);
    }
}

#[cfg(target_os = "linux")]
fn worker(
    receiver: mpsc::Receiver<SoundEvent>,
    state: Arc<AtomicU32>,
    status: Arc<AtomicU8>,
    directory: Option<std::path::PathBuf>,
) {
    let selected = ((state.load(Ordering::Relaxed) >> 1) & 7) as usize;
    let Some(profile) = SoundProfile::ALL.get(selected) else {
        return;
    };
    let Ok(samples) = load_sound(*profile, directory.as_deref()) else {
        status.store(4, Ordering::Relaxed);
        return;
    };
    let mut bank: [Vec<f32>; 5] = std::array::from_fn(|_| Vec::new());
    bank[selected] = samples;
    if state.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Ok(mut output) = pulse::Output::open() else {
        status.store(3, Ordering::Relaxed);
        return;
    };
    status.store(2, Ordering::Relaxed);
    let mut voices: Vec<(usize, usize)> = Vec::with_capacity(8);
    loop {
        if state.load(Ordering::Relaxed) == 0 {
            return;
        }
        if voices.is_empty() {
            let Ok(event) = receiver.recv() else {
                return;
            };
            if event.is_fresh() {
                voices.push((((state.load(Ordering::Relaxed) >> 1) & 7) as usize, 0));
            }
        }
        while let Ok(event) = receiver.try_recv() {
            if event.is_fresh() && voices.len() < 8 {
                voices.push((((state.load(Ordering::Relaxed) >> 1) & 7) as usize, 0));
            }
        }
        let value = state.load(Ordering::Relaxed);
        if value == 0 {
            return;
        }
        let volume = (value >> 8) as f32 / 100.0;
        let mut buffer = [0_i16; 240]; // Five milliseconds; backend writes never touch the UI thread.
        for sample in &mut buffer {
            let mut mixed = 0.0;
            for (profile, cursor) in &mut voices {
                mixed += bank[*profile].get(*cursor).copied().unwrap_or(0.0);
                *cursor += 1;
            }
            *sample = output_sample(mixed, volume);
        }
        voices.retain(|(profile, cursor)| *cursor < bank[*profile].len());
        if output.write(&buffer).is_err() {
            status.store(3, Ordering::Relaxed);
            return;
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn worker(
    _: mpsc::Receiver<SoundEvent>,
    _: Arc<AtomicU32>,
    status: Arc<AtomicU8>,
    _: Option<std::path::PathBuf>,
) {
    status.store(3, Ordering::Relaxed);
}

// Load the optional Linux runtime library instead of requiring audio development
// headers or making sound a startup dependency. PulseAudio, PipeWire's PulseAudio
// compatibility server, and WSLg expose this interface.
#[cfg(target_os = "linux")]
mod pulse {
    use libloading::Library;
    use std::ffi::{c_char, c_int, c_void};
    #[repr(C)]
    struct SampleSpec {
        format: c_int,
        rate: u32,
        channels: u8,
    }
    #[repr(C)]
    struct BufferAttr {
        maxlength: u32,
        tlength: u32,
        prebuf: u32,
        minreq: u32,
        fragsize: u32,
    }
    type New = unsafe extern "C" fn(
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        *const c_char,
        *const SampleSpec,
        *const c_void,
        *const BufferAttr,
        *mut c_int,
    ) -> *mut c_void;
    type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_int) -> c_int;
    type Free = unsafe extern "C" fn(*mut c_void);
    pub struct Output {
        stream: *mut c_void,
        write: Write,
        free: Free,
        _library: Library,
    }
    impl Output {
        pub fn open() -> Result<Self, ()> {
            // SAFETY: These signatures and repr(C) structures match libpulse-simple's
            // stable C API. The library remains alive until after its stream is freed.
            unsafe {
                let library = Library::new("libpulse-simple.so.0").map_err(|_| ())?;
                let new = *library.get::<New>(b"pa_simple_new\0").map_err(|_| ())?;
                let write = *library.get::<Write>(b"pa_simple_write\0").map_err(|_| ())?;
                let free = *library.get::<Free>(b"pa_simple_free\0").map_err(|_| ())?;
                let format = if cfg!(target_endian = "little") { 3 } else { 4 }; // PA_SAMPLE_S16LE / BE
                let spec = SampleSpec {
                    format,
                    rate: super::SAMPLE_RATE,
                    channels: 1,
                };
                let attr = BufferAttr {
                    maxlength: 4800,
                    tlength: 1920,
                    prebuf: 480,
                    minreq: 480,
                    fragsize: u32::MAX,
                };
                let stream = new(
                    std::ptr::null(),
                    c"Quickfox".as_ptr(),
                    1,
                    std::ptr::null(),
                    c"Key sounds".as_ptr(),
                    &spec,
                    std::ptr::null(),
                    &attr,
                    std::ptr::null_mut(),
                );
                if stream.is_null() {
                    return Err(());
                }
                Ok(Self {
                    stream,
                    write,
                    free,
                    _library: library,
                })
            }
        }
        pub fn write(&mut self, samples: &[i16]) -> Result<(), ()> {
            // SAFETY: Live stream, initialized contiguous samples, exact byte length.
            let code = unsafe {
                (self.write)(
                    self.stream,
                    samples.as_ptr().cast(),
                    std::mem::size_of_val(samples),
                    std::ptr::null_mut(),
                )
            };
            if code < 0 { Err(()) } else { Ok(()) }
        }
    }
    impl Drop for Output {
        fn drop(&mut self) {
            // SAFETY: This object uniquely owns the stream and its library is still loaded.
            unsafe { (self.free)(self.stream) };
        }
    }
}
