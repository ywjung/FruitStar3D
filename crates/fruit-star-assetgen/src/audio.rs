//! Procedurally synthesised sound effects (44.1 kHz, 16-bit PCM, mono WAV).
//!
//! Everything is built from oscillators, filtered noise and envelopes that are
//! implemented in this file; no samples or recordings are used. All melodies
//! are original to this project.

use std::f32::consts::{PI, TAU};

use anyhow::{Result, ensure};

use crate::rng::Rng;

pub const SAMPLE_RATE: u32 = 44_100;
const SR: f32 = SAMPLE_RATE as f32;

/// Description of one sound effect and how to synthesise it.
pub struct Spec {
    /// File stem (the file is `<name>.wav`).
    pub name: &'static str,
    /// Nominal duration in seconds.
    pub seconds: f32,
    /// Whether the file is a seamless loop.
    pub looped: bool,
    /// Peak level after normalisation, in dBFS.
    pub peak_db: f32,
    /// Short description for the asset manifest.
    pub summary: &'static str,
    synth: fn(usize) -> Vec<f32>,
}

/// All sound effects, in output order.
pub const SPECS: [Spec; 22] = [
    Spec {
        name: "power_on",
        seconds: 1.5,
        looped: false,
        peak_db: -3.0,
        summary: "relay clicks, hum swell and rising chime",
        synth: power_on,
    },
    Spec {
        name: "ambient",
        seconds: 8.0,
        looped: true,
        peak_db: -18.0,
        summary: "quiet machine-fan / arcade-room hum (seamless loop)",
        synth: ambient,
    },
    Spec {
        name: "lever_pull",
        seconds: 0.25,
        looped: false,
        peak_db: -3.0,
        summary: "lever latch, travel and sprung end stop",
        synth: lever_pull,
    },
    Spec {
        name: "lever_return",
        seconds: 0.18,
        looped: false,
        peak_db: -3.0,
        summary: "lighter spring snap-back",
        synth: lever_return,
    },
    Spec {
        name: "reel_start",
        seconds: 0.4,
        looped: false,
        peak_db: -3.0,
        summary: "stepper engaging and accelerating, with paper drum chatter",
        synth: reel_start,
    },
    Spec {
        name: "reel_loop",
        // Three full drum turns at 80 rpm, 21 steps per turn.
        seconds: 2.25,
        looped: true,
        peak_db: -3.0,
        summary: "stepper and bearings under 28 indexing ticks/s, seamless over three turns",
        synth: reel_loop,
    },
    Spec {
        name: "reel_stop",
        seconds: 0.18,
        looped: false,
        peak_db: -3.0,
        summary: "brake shoe closes, pawl rebounds and the chassis damps",
        synth: reel_stop,
    },
    Spec {
        name: "button_press",
        seconds: 0.08,
        looped: false,
        peak_db: -3.0,
        summary: "crisp plastic click with a short 1.8 kHz tick tone",
        synth: button_press,
    },
    Spec {
        name: "button_release",
        seconds: 0.06,
        looped: false,
        peak_db: -3.0,
        summary: "softer, lower click up",
        synth: button_release,
    },
    Spec {
        name: "medal_insert",
        seconds: 0.4,
        looped: false,
        peak_db: -3.0,
        summary: "slot lip strike, rolling guide and muffled acceptor gate",
        synth: medal_insert,
    },
    Spec {
        name: "medal_drop",
        seconds: 0.3,
        looped: false,
        peak_db: -3.0,
        summary: "hopper pawl indexing and a medal sliding out through the chute",
        synth: medal_drop,
    },
    Spec {
        name: "medal_collision",
        seconds: 0.1,
        looped: false,
        peak_db: -3.0,
        summary: "short, damped edge contact between loose medals",
        synth: medal_collision,
    },
    Spec {
        name: "medal_tray",
        seconds: 0.26,
        looped: false,
        peak_db: -3.0,
        summary: "coin edge ringing over the hollow modes of the metal payout tray",
        synth: medal_tray,
    },
    Spec {
        name: "bonus_start",
        seconds: 2.8,
        looped: false,
        peak_db: -3.0,
        summary: "original fanfare: rising arpeggios over C-F-G, snare, held tremolo chord",
        synth: bonus_start,
    },
    Spec {
        name: "bonus_end",
        seconds: 1.8,
        looped: false,
        peak_db: -3.0,
        summary: "original descending chip-tone phrase resolving on the tonic",
        synth: bonus_end,
    },
    Spec {
        name: "menu_select",
        seconds: 0.1,
        looped: false,
        peak_db: -3.0,
        summary: "soft two-step UI blip",
        synth: menu_select,
    },
    Spec {
        name: "win_small",
        seconds: 0.6,
        looped: false,
        peak_db: -3.0,
        summary: "rising chip-tone arpeggio C6-E6-G6 with sparkle on the last note",
        synth: win_small,
    },
    Spec {
        name: "chance_lamp",
        seconds: 0.7,
        looped: false,
        peak_db: -3.0,
        summary: "sharp rising FM/ring-mod shimmer sweep with echo tail",
        synth: chance_lamp,
    },
    Spec {
        name: "bet",
        seconds: 0.09,
        looped: false,
        peak_db: -3.0,
        summary: "short soft-square beep at A5 (880 Hz), made to be pitch-shifted",
        synth: bet,
    },
    Spec {
        name: "replay",
        seconds: 0.45,
        looped: false,
        peak_db: -3.0,
        summary: "two-tone signal E6-B5, repeated once",
        synth: replay,
    },
    Spec {
        name: "win_big",
        seconds: 1.4,
        looped: false,
        peak_db: -3.0,
        summary: "original big-win jingle: arpeggio runs over C and F, sparkling C7 finish",
        synth: win_big,
    },
    Spec {
        name: "bonus_loop",
        // 8 bars of 4/4 at 150 BPM: 32 beats of exactly 17 640 samples.
        seconds: 12.8,
        looped: true,
        peak_db: -9.0,
        summary: "original chiptune bonus music, 8 bars at 150 BPM in C major (seamless loop)",
        synth: bonus_loop,
    },
];

// ---------------------------------------------------------------------------
// Rendering, WAV encoding and verification
// ---------------------------------------------------------------------------

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Synthesises, normalises and quantises one sound.
pub fn render(spec: &Spec) -> Vec<i16> {
    let n = (spec.seconds * SR).round() as usize;
    let mut buf = (spec.synth)(n);
    buf.resize(n, 0.0);

    // Remove any DC offset, then (for one-shots) fade the ends to avoid clicks.
    let mean = buf.iter().sum::<f32>() / n as f32;
    for s in &mut buf {
        *s -= mean;
    }
    if !spec.looped {
        fade_edges(&mut buf, 0.002, 0.005);
    }

    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-9);
    let gain = db_to_linear(spec.peak_db) / peak;
    buf.iter()
        .map(|s| (s * gain * 32_767.0).round().clamp(-32_767.0, 32_767.0) as i16)
        .collect()
}

fn fade_edges(buf: &mut [f32], fade_in: f32, fade_out: f32) {
    let n = buf.len();
    let n_in = ((fade_in * SR) as usize).min(n / 2);
    let n_out = ((fade_out * SR) as usize).min(n / 2);
    for (i, s) in buf.iter_mut().take(n_in).enumerate() {
        *s *= 0.5 - 0.5 * (PI * i as f32 / n_in as f32).cos();
    }
    for (i, s) in buf.iter_mut().rev().take(n_out).enumerate() {
        *s *= 0.5 - 0.5 * (PI * i as f32 / n_out as f32).cos();
    }
}

/// Encodes mono 16-bit samples as a canonical 44-byte-header WAV file.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Decoded contents of a WAV file written by [`wav_bytes`].
#[derive(Debug)]
pub struct Wav {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub samples: Vec<i16>,
}

/// Parses a canonical PCM WAV file (as written by this tool).
pub fn parse_wav(bytes: &[u8]) -> Result<Wav> {
    ensure!(bytes.len() >= 44, "WAV file too short");
    ensure!(
        &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "not a RIFF/WAVE file"
    );
    ensure!(
        &bytes[12..16] == b"fmt " && &bytes[36..40] == b"data",
        "unexpected chunk layout"
    );
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    ensure!(u16_at(20) == 1, "not PCM");
    let data_len = u32_at(40) as usize;
    ensure!(bytes.len() == 44 + data_len, "data chunk length mismatch");
    ensure!(u32_at(4) as usize == 36 + data_len, "RIFF length mismatch");
    let (pairs, rest) = bytes[44..].as_chunks::<2>();
    ensure!(rest.is_empty(), "odd number of data bytes");
    let samples = pairs.iter().map(|c| i16::from_le_bytes(*c)).collect();
    Ok(Wav {
        sample_rate: u32_at(24),
        channels: u16_at(22),
        bits: u16_at(34),
        samples,
    })
}

/// Measured properties of a verified sound.
#[derive(Debug, Clone, Copy)]
pub struct Report {
    pub seconds: f32,
    pub peak_db: f32,
    pub rms_db: f32,
}

/// Sanity-checks a decoded file against its specification.
pub fn verify(spec: &Spec, wav: &Wav) -> Result<Report> {
    let name = spec.name;
    ensure!(
        wav.sample_rate == SAMPLE_RATE,
        "{name}: sample rate {}",
        wav.sample_rate
    );
    ensure!(wav.channels == 1, "{name}: {} channels", wav.channels);
    ensure!(wav.bits == 16, "{name}: {} bits", wav.bits);
    let n = wav.samples.len();
    ensure!(n > 16, "{name}: empty");

    let seconds = n as f32 / SR;
    ensure!(
        (seconds - spec.seconds).abs() <= spec.seconds * 0.1,
        "{name}: duration {seconds:.3}s, expected about {}s",
        spec.seconds
    );

    let peak = wav
        .samples
        .iter()
        .map(|s| i32::from(*s).abs())
        .max()
        .unwrap_or(0);
    ensure!(peak < 32_767, "{name}: clipped");
    let peak_db = 20.0 * (peak as f32 / 32_767.0).max(1e-9).log10();
    ensure!(
        (peak_db - spec.peak_db).abs() <= 0.5,
        "{name}: peak {peak_db:.2} dBFS, expected about {}",
        spec.peak_db
    );

    let mean_square = wav
        .samples
        .iter()
        .map(|s| f64::from(*s).powi(2))
        .sum::<f64>()
        / n as f64;
    let rms_db = 20.0 * ((mean_square.sqrt() / 32_767.0).max(1e-9) as f32).log10();
    ensure!(
        rms_db > spec.peak_db - 40.0,
        "{name}: nearly silent ({rms_db:.1} dBFS RMS)"
    );

    let s = |i: usize| i32::from(wav.samples[i]);
    if spec.looped {
        // The step across the loop point (and the steps next to it) must be no
        // rougher than ordinary sample-to-sample motion inside the file.
        let mut steps: Vec<i32> = (1..n).map(|i| (s(i) - s(i - 1)).abs()).collect();
        steps.sort_unstable();
        let typical = steps[steps.len() * 99 / 100].max(2);
        let wrap = (s(0) - s(n - 1)).abs();
        ensure!(
            wrap <= typical * 3 / 2,
            "{name}: loop seam step {wrap} exceeds typical step {typical}"
        );
        // Slope continuity: the sample predicted from the tail must match the head.
        let mut curves: Vec<i32> = (2..n)
            .map(|i| (s(i) - 2 * s(i - 1) + s(i - 2)).abs())
            .collect();
        curves.sort_unstable();
        let typical_curve = curves[curves.len() * 99 / 100].max(2);
        for (a, b, c) in [(n - 2, n - 1, 0), (n - 1, 0, 1)] {
            let curve = (s(c) - 2 * s(b) + s(a)).abs();
            ensure!(
                curve <= typical_curve * 2,
                "{name}: loop seam curvature {curve} exceeds typical {typical_curve}"
            );
        }
    } else {
        let edge = peak / 50 + 8;
        ensure!(
            s(0).abs() <= edge && s(n - 1).abs() <= edge,
            "{name}: does not start/end near silence ({} / {})",
            s(0),
            s(n - 1)
        );
    }

    Ok(Report {
        seconds,
        peak_db,
        rms_db,
    })
}

/// Parses and verifies WAV bytes in one step.
pub fn verify_bytes(spec: &Spec, bytes: &[u8]) -> Result<Report> {
    let wav = parse_wav(bytes)?;
    verify(spec, &wav)
}

/// Looks up a specification by file stem.
#[cfg(test)]
pub fn spec(name: &str) -> Result<&'static Spec> {
    SPECS
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("unknown sound {name:?}"))
}

// ---------------------------------------------------------------------------
// DSP building blocks
// ---------------------------------------------------------------------------

/// Second-order IIR filter (RBJ cookbook coefficients, transposed direct form II).
#[derive(Debug, Clone)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coefficients(b: [f32; 3], a: [f32; 3]) -> Self {
        Self {
            b0: b[0] / a[0],
            b1: b[1] / a[0],
            b2: b[2] / a[0],
            a1: a[1] / a[0],
            a2: a[2] / a[0],
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn lowpass(freq: f32, q: f32) -> Self {
        let w = TAU * freq / SR;
        let (alpha, cos) = (w.sin() / (2.0 * q), w.cos());
        Self::from_coefficients(
            [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    fn highpass(freq: f32, q: f32) -> Self {
        let w = TAU * freq / SR;
        let (alpha, cos) = (w.sin() / (2.0 * q), w.cos());
        Self::from_coefficients(
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    /// Band-pass with unity gain at the centre frequency.
    fn bandpass(freq: f32, q: f32) -> Self {
        let w = TAU * freq / SR;
        let (alpha, cos) = (w.sin() / (2.0 * q), w.cos());
        Self::from_coefficients([alpha, 0.0, -alpha], [1.0 + alpha, -2.0 * cos, 1.0 - alpha])
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// First-order low-pass used for smoothing control signals and soft noise.
#[derive(Debug, Clone)]
#[cfg(test)]
struct OnePole {
    a: f32,
    z: f32,
}

#[cfg(test)]
impl OnePole {
    fn lowpass(freq: f32) -> Self {
        Self {
            a: 1.0 - (-TAU * freq / SR).exp(),
            z: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
}

fn sample_index(t: f32) -> usize {
    (t * SR).round().max(0.0) as usize
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rms(buf: &[f32]) -> f32 {
    (buf.iter().map(|s| s * s).sum::<f32>() / buf.len().max(1) as f32).sqrt()
}

/// Scales `layer` to the given RMS level and mixes it into `buf`.
fn mix_at_rms(buf: &mut [f32], layer: &[f32], level: f32) {
    let gain = level / rms(layer).max(1e-9);
    for (dst, src) in buf.iter_mut().zip(layer) {
        *dst += src * gain;
    }
}

/// Adds exponentially decaying sine partials `(frequency, amplitude, decay time)`
/// starting at `t0`. With `wrap`, tails running past the end continue at the start
/// (used to keep loops periodic).
fn ping(buf: &mut [f32], t0: f32, gain: f32, partials: &[(f32, f32, f32)], wrap: bool) {
    let n = buf.len();
    let start = sample_index(t0);
    for &(freq, amp, tau) in partials {
        let count = (tau * 7.0 * SR) as usize;
        for i in 0..count {
            let mut idx = start + i;
            if idx >= n {
                if !wrap {
                    break;
                }
                idx %= n;
            }
            let t = i as f32 / SR;
            let attack = (t / 0.0002).min(1.0);
            buf[idx] += gain * amp * attack * (-t / tau).exp() * (TAU * freq * t).sin();
        }
    }
}

/// Adds a burst of filtered noise with an exponential decay.
fn burst(buf: &mut [f32], t0: f32, gain: f32, tau: f32, rng: &mut Rng, filter: &mut Biquad) {
    let start = sample_index(t0);
    let count = (tau * 7.0 * SR) as usize;
    for i in 0..count {
        let Some(slot) = buf.get_mut(start + i) else {
            break;
        };
        let t = i as f32 / SR;
        *slot += gain * (-t / tau).exp() * filter.process(rng.bipolar());
    }
}

/// Adds a sine "thump" whose pitch glides from `f_start` down/up to `f_end`.
fn thump(buf: &mut [f32], t0: f32, gain: f32, f_start: f32, f_end: f32, glide: f32, decay: f32) {
    let start = sample_index(t0);
    let count = (decay * 7.0 * SR) as usize;
    let mut phase = 0.0f32;
    for i in 0..count {
        let Some(slot) = buf.get_mut(start + i) else {
            break;
        };
        let t = i as f32 / SR;
        let freq = f_end + (f_start - f_end) * (-t / glide).exp();
        phase += TAU * freq / SR;
        let attack = (t / 0.0005).min(1.0);
        *slot += gain * attack * (-t / decay).exp() * phase.sin();
    }
}

/// Turns `long` (length `n` + overlap) into a seamless loop of length `n` by
/// cross-fading the overlap into the head with equal-power curves.
fn loopify(long: &[f32], n: usize) -> Vec<f32> {
    let overlap = long.len() - n;
    let mut out = long[..n].to_vec();
    for i in 0..overlap {
        let theta = (i as f32 + 0.5) / overlap as f32 * PI / 2.0;
        out[i] = long[i] * theta.sin() + long[n + i] * theta.cos();
    }
    out
}

/// Filtered noise of `count` samples.
fn noise(count: usize, rng: &mut Rng, filters: &mut [Biquad]) -> Vec<f32> {
    (0..count)
        .map(|_| filters.iter_mut().fold(rng.bipolar(), |x, f| f.process(x)))
        .collect()
}

// ---------------------------------------------------------------------------
// Chip-tone voices for the jingles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Wave {
    /// Pulse wave with the given duty cycle.
    Pulse(f32),
    /// Rounded square: odd harmonics falling off faster than a true square's
    /// (between a square and a triangle), the classic soft "beep" timbre.
    SoftSquare,
    Triangle,
    Sine,
}

impl Wave {
    /// Amplitude of the `k`-th harmonic (band-limited additive synthesis).
    fn harmonic(self, k: usize) -> f32 {
        let kf = k as f32;
        match self {
            Wave::Pulse(duty) => (PI * kf * duty).sin() / kf * (4.0 / PI) * 0.5,
            Wave::SoftSquare => {
                if k % 2 == 1 {
                    (2.0 / PI) / kf.powf(1.6)
                } else {
                    0.0
                }
            }
            Wave::Triangle => {
                if k % 2 == 1 {
                    let sign = if (k / 2).is_multiple_of(2) { 1.0 } else { -1.0 };
                    sign * 8.0 / (PI * PI) / (kf * kf)
                } else {
                    0.0
                }
            }
            Wave::Sine => {
                if k == 1 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// A note: start time and length in seconds, MIDI pitch and amplitude.
#[derive(Debug, Clone, Copy)]
struct Note {
    start: f32,
    length: f32,
    pitch: f32,
    amp: f32,
}

const fn note(start: f32, length: f32, pitch: f32, amp: f32) -> Note {
    Note {
        start,
        length,
        pitch,
        amp,
    }
}

fn pitch_to_hz(pitch: f32) -> f32 {
    440.0 * 2f32.powf((pitch - 69.0) / 12.0)
}

/// Per-voice articulation.
#[derive(Debug, Clone, Copy)]
struct Articulation {
    /// Vibrato depth in semitones (applied after a short delay).
    vibrato: f32,
    /// Release time constant after the note ends.
    release: f32,
    /// Time constant of the decay towards `sustain` while the note is held.
    decay: f32,
    /// Sustain level relative to the attack peak.
    sustain: f32,
}

fn voice(buf: &mut [f32], notes: &[Note], wave: Wave, art: Articulation) {
    for n in notes {
        let freq = pitch_to_hz(n.pitch);
        let harmonics = ((15_000.0 / freq) as usize).clamp(1, 28);
        let weights: Vec<f32> = (1..=harmonics).map(|k| wave.harmonic(k)).collect();
        let start = sample_index(n.start);
        let count = ((n.length + art.release * 7.0) * SR) as usize;
        let mut phase = 0.0f32;
        for i in 0..count {
            let Some(slot) = buf.get_mut(start + i) else {
                break;
            };
            let t = i as f32 / SR;
            let wobble = art.vibrato * smoothstep(0.09, 0.24, t) * (TAU * 6.2 * t).sin();
            phase += TAU * freq * 2f32.powf(wobble / 12.0) / SR;
            if phase > TAU {
                phase -= TAU;
            }
            let attack = (t / 0.004).min(1.0);
            let body = art.sustain + (1.0 - art.sustain) * (-t / art.decay).exp();
            let release = if t > n.length {
                (-(t - n.length) / art.release).exp()
            } else {
                1.0
            };
            let mut x = 0.0;
            for (k, w) in weights.iter().enumerate() {
                if *w != 0.0 {
                    x += w * (phase * (k + 1) as f32).sin();
                }
            }
            *slot += n.amp * attack * body * release * x;
        }
    }
}

// MIDI pitches used by the jingles.
const F2: f32 = 41.0;
const G2: f32 = 43.0;
const A2: f32 = 45.0;
const C3: f32 = 48.0;
const F3: f32 = 53.0;
const G3: f32 = 55.0;
const B3: f32 = 59.0;
const C4: f32 = 60.0;
const D4: f32 = 62.0;
const E4: f32 = 64.0;
const F4: f32 = 65.0;
const G4: f32 = 67.0;
const A4: f32 = 69.0;
const C5: f32 = 72.0;
const D5: f32 = 74.0;
const E5: f32 = 76.0;
const F5: f32 = 77.0;
const G5: f32 = 79.0;
const A5: f32 = 81.0;
const B5: f32 = 83.0;
const C6: f32 = 84.0;
const D6: f32 = 86.0;
const E6: f32 = 88.0;
const F6: f32 = 89.0;
const G6: f32 = 91.0;
const A6: f32 = 93.0;
const B6: f32 = 95.0;
const C7: f32 = 96.0;
const E7: f32 = 100.0;
const G7: f32 = 103.0;
const C8: f32 = 108.0;
const D8: f32 = 110.0;
const G8: f32 = 115.0;

// ---------------------------------------------------------------------------
// Shared pieces of the slot-machine sound set
// ---------------------------------------------------------------------------

/// Staccato chip-tone articulation for fast arpeggio runs.
const RUN: Articulation = Articulation {
    vibrato: 0.0,
    release: 0.012,
    decay: 0.05,
    sustain: 0.6,
};

/// Held chip-tone note with a little vibrato.
const HELD: Articulation = Articulation {
    vibrato: 0.15,
    release: 0.05,
    decay: 0.1,
    sustain: 0.65,
};

/// Plain triangle bass.
const BASS: Articulation = Articulation {
    vibrato: 0.0,
    release: 0.02,
    decay: 0.15,
    sustain: 0.75,
};

/// Folds everything past sample `n` back onto the start, so that tails ringing
/// across the loop point continue at the head: the result is exactly periodic.
fn fold_tail(long: &[f32], n: usize) -> Vec<f32> {
    let mut out = long[..n].to_vec();
    for (i, s) in long[n..].iter().enumerate() {
        out[i % n] += s;
    }
    out
}

/// Adds `repeats` echoes of `src` to `buf`, `delay` seconds apart, each
/// `feedback` times quieter than the one before.
fn add_echoes(buf: &mut [f32], src: &[f32], delay: f32, feedback: f32, repeats: usize) {
    let d = sample_index(delay);
    let mut gain = 1.0;
    for r in 1..=repeats {
        gain *= feedback;
        for (dst, s) in buf.iter_mut().skip(r * d).zip(src) {
            *dst += gain * s;
        }
    }
}

/// Like [`burst`], but with a short attack ramp, so the hit may sit exactly on
/// a loop point without a step.
fn hit(buf: &mut [f32], t0: f32, gain: f32, tau: f32, rng: &mut Rng, filter: &mut Biquad) {
    let start = sample_index(t0);
    let count = (tau * 7.0 * SR) as usize;
    for i in 0..count {
        let Some(slot) = buf.get_mut(start + i) else {
            break;
        };
        let t = i as f32 / SR;
        let attack = (t / 0.0006).min(1.0);
        *slot += gain * attack * (-t / tau).exp() * filter.process(rng.bipolar());
    }
}

/// Chip-tune noise snare: a band-passed body with a brighter sizzle on top.
fn snare(buf: &mut [f32], t0: f32, gain: f32, rng: &mut Rng) {
    hit(
        buf,
        t0,
        gain,
        0.035,
        rng,
        &mut Biquad::bandpass(1900.0, 0.8),
    );
    hit(
        buf,
        t0,
        gain * 0.5,
        0.022,
        rng,
        &mut Biquad::highpass(5500.0, 0.7),
    );
}

/// A run of short glassy pings cycling through `pitches` (MIDI), fading out.
fn sparkle(buf: &mut [f32], t0: f32, spacing: f32, pitches: &[f32], gain: f32) {
    for (j, pitch) in pitches.iter().enumerate() {
        let fade = 1.0 - j as f32 / (pitches.len() as f32 * 1.3);
        let f = pitch_to_hz(*pitch);
        ping(
            buf,
            t0 + j as f32 * spacing,
            gain * fade,
            &[(f, 1.0, 0.07), (f * 2.0, 0.3, 0.04)],
            false,
        );
    }
}

/// Fades the whole buffer out over the last `seconds`, so a held chord rings
/// out instead of stopping dead.
fn fade_tail(buf: &mut [f32], seconds: f32) {
    let end = buf.len() as f32 / SR;
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot *= 1.0 - smoothstep(end - seconds, end - 0.005, i as f32 / SR);
    }
}

/// Ringing partials of a struck medal — inharmonic, like a small metal disc:
/// `(frequency, amplitude, decay)`.
const COIN_PARTIALS: [(f32, f32, f32); 5] = [
    (2100.0, 0.5, 0.075),
    (3400.0, 0.45, 0.055),
    (5600.0, 0.35, 0.034),
    (8300.0, 0.18, 0.02),
    (11_300.0, 0.08, 0.012),
];

fn scaled_partials(partials: &[(f32, f32, f32)], pitch: f32, decay: f32) -> Vec<(f32, f32, f32)> {
    partials
        .iter()
        .map(|&(f, a, tau)| (f * pitch, a, tau * decay))
        .collect()
}

/// A medal being struck: a tiny bright click plus its ringing partials.
fn coin_strike(buf: &mut [f32], t0: f32, gain: f32, pitch: f32, decay: f32, rng: &mut Rng) {
    burst(
        buf,
        t0,
        0.5 * gain,
        0.0012,
        rng,
        &mut Biquad::highpass(3500.0, 0.8),
    );
    ping(
        buf,
        t0,
        gain,
        &scaled_partials(&COIN_PARTIALS, pitch, decay),
        false,
    );
}

/// Symbols passing the window per second at full speed (80 rpm x 21 symbols).
const REEL_TICK_HZ: f32 = 28.0;
/// One reel tick: a short bright click as a symbol passes the detent.
fn reel_tick(buf: &mut [f32], t0: f32, gain: f32, rng: &mut Rng, wrap: bool) {
    burst(
        buf,
        t0,
        0.55 * gain,
        0.0012,
        rng,
        &mut Biquad::highpass(3800.0, 0.8),
    );
    ping(
        buf,
        t0,
        gain,
        &[
            (1840.0, 0.50, 0.0038),
            (3250.0, 0.38, 0.0026),
            (5480.0, 0.16, 0.0017),
        ],
        wrap,
    );
}

// ---------------------------------------------------------------------------
// Individual sounds
// ---------------------------------------------------------------------------

fn power_on(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA001);

    // Two relay clicks.
    for (t0, gain) in [(0.03f32, 1.0f32), (0.118, 0.6)] {
        burst(
            &mut buf,
            t0,
            0.9 * gain,
            0.004,
            &mut rng,
            &mut Biquad::bandpass(2800.0, 1.5),
        );
        thump(&mut buf, t0, 0.7 * gain, 190.0, 120.0, 0.01, 0.018);
        ping(
            &mut buf,
            t0,
            gain,
            &[(930.0, 0.3, 0.009), (1510.0, 0.2, 0.006)],
            false,
        );
    }

    // Transformer hum swelling in, with a rising electronic sweep on top.
    let mut hum_phase = 0.0f32;
    let mut sweep_phase = 0.0f32;
    for (i, slot) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR;
        if t < 0.12 {
            continue;
        }
        let u = t - 0.12;
        let hum_freq = 100.0 - 14.0 * (-u / 0.25).exp();
        hum_phase += TAU * hum_freq / SR;
        let swell = (1.0 - (-u / 0.4).exp()) * (1.0 - smoothstep(1.05, 1.5, t));
        let hum = hum_phase.sin()
            + 0.5 * (2.0 * hum_phase).sin()
            + 0.28 * (3.0 * hum_phase).sin()
            + 0.12 * (5.0 * hum_phase).sin();
        *slot += 0.2 * swell * hum;

        let progress = (u / 0.85).min(1.0);
        let sweep_freq = 180.0 * (1100.0f32 / 180.0).powf(progress);
        sweep_phase += TAU * sweep_freq / SR;
        let sweep_env = smoothstep(0.0, 0.2, progress) * (1.0 - smoothstep(0.75, 1.0, progress));
        *slot += 0.13 * sweep_env * (sweep_phase.sin() + 0.3 * (2.0 * sweep_phase).sin());
    }

    // Rising chime (D major arpeggio, bell-like sine partials).
    for (k, (t0, pitch)) in [(0.42f32, 74.0f32), (0.56, 78.0), (0.70, 81.0), (0.86, 86.0)]
        .into_iter()
        .enumerate()
    {
        let f = pitch_to_hz(pitch);
        let tau = if k == 3 { 0.34 } else { 0.22 };
        ping(
            &mut buf,
            t0,
            0.42,
            &[
                (f, 1.0, tau),
                (f * 2.0, 0.35, tau * 0.6),
                (f * 3.01, 0.12, tau * 0.35),
            ],
            false,
        );
    }
    buf
}

fn ambient(n: usize) -> Vec<f32> {
    let overlap = SAMPLE_RATE as usize;
    let total = n + overlap;
    let mut rng = Rng::new(0xA002);
    let mut long = vec![0.0; total];

    let rumble = noise(
        total,
        &mut rng,
        &mut [Biquad::lowpass(110.0, 0.7), Biquad::lowpass(160.0, 0.7)],
    );
    mix_at_rms(&mut long, &rumble, 0.5);
    let fan = noise(
        total,
        &mut rng,
        &mut [Biquad::bandpass(310.0, 0.7), Biquad::lowpass(700.0, 0.7)],
    );
    mix_at_rms(&mut long, &fan, 0.32);
    let air = noise(
        total,
        &mut rng,
        &mut [Biquad::bandpass(2300.0, 0.6), Biquad::lowpass(4000.0, 0.7)],
    );
    mix_at_rms(&mut long, &air, 0.05);

    let mut out = loopify(&long, n);

    // Tonal hum and a slow breathing modulation; every frequency completes a
    // whole number of cycles over the loop, so the result stays periodic.
    let period = n as f32 / SR;
    let cycles = |hz: f32| (hz * period).round() / period;
    let tones = [
        (cycles(60.0), 0.11f32, 0.0f32),
        (cycles(120.0), 0.08, 1.1),
        (cycles(180.0), 0.03, 2.3),
        (cycles(87.5), 0.06, 0.7),
        (cycles(175.0), 0.025, 1.9),
    ];
    for (i, slot) in out.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut hum = 0.0;
        for (freq, amp, phase) in tones {
            hum += amp * (TAU * freq * t + phase).sin();
        }
        let breathe = 1.0
            + 0.14 * (TAU * cycles(0.25) * t).sin()
            + 0.07 * (TAU * cycles(0.375) * t + 1.0).sin();
        *slot = (*slot + hum) * breathe;
    }
    out
}

fn lever_pull(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA003);

    // "Ka": the latch letting go.
    burst(
        &mut buf,
        0.003,
        0.5,
        0.003,
        &mut rng,
        &mut Biquad::highpass(1800.0, 0.8),
    );
    thump(&mut buf, 0.003, 0.55, 150.0, 85.0, 0.012, 0.028);
    ping(
        &mut buf,
        0.003,
        1.0,
        &[(1240.0, 0.3, 0.012), (2090.0, 0.18, 0.008)],
        false,
    );

    // "Chunk": the lever hitting its stop.
    let hit_at = 0.07;
    thump(&mut buf, hit_at, 1.0, 118.0, 52.0, 0.025, 0.045);
    burst(
        &mut buf,
        hit_at,
        0.6,
        0.015,
        &mut rng,
        &mut Biquad::lowpass(1300.0, 0.9),
    );
    ping(
        &mut buf,
        hit_at,
        1.0,
        &[(410.0, 0.3, 0.025), (655.0, 0.2, 0.018)],
        false,
    );

    // Spring rattle: inharmonic partials and hiss, chopped by a fast tremolo.
    let mut rattle = vec![0.0; n];
    ping(
        &mut rattle,
        hit_at,
        1.0,
        &[
            (1730.0, 0.16, 0.04),
            (2590.0, 0.14, 0.035),
            (3410.0, 0.1, 0.03),
            (4870.0, 0.07, 0.025),
        ],
        false,
    );
    burst(
        &mut rattle,
        hit_at,
        0.3,
        0.03,
        &mut rng,
        &mut Biquad::bandpass(3600.0, 2.0),
    );
    for (i, (dst, src)) in buf.iter_mut().zip(&rattle).enumerate() {
        let t = i as f32 / SR - hit_at;
        let chop = 0.35 + 0.65 * (0.5 + 0.5 * (TAU * 83.0 * t).sin()).powi(2);
        *dst += src * chop;
    }

    // The catch settles against its mount after the heavy second impact.
    ping(
        &mut buf,
        0.123,
        0.24,
        &[(328., 0.7, 0.010), (1635., 0.28, 0.006)],
        false,
    );
    buf
}

fn lever_return(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA004);

    burst(
        &mut buf,
        0.003,
        0.4,
        0.002,
        &mut rng,
        &mut Biquad::highpass(2200.0, 0.8),
    );
    // Spring twang gliding upwards as the lever flies back.
    thump(&mut buf, 0.003, 0.36, 240.0, 304.0, 0.011, 0.020);
    thump(&mut buf, 0.003, 0.12, 523.0, 660.0, 0.014, 0.018);
    // Light stop.
    let stop = 0.088;
    thump(&mut buf, stop, 0.7, 230.0, 150.0, 0.008, 0.016);
    burst(
        &mut buf,
        stop,
        0.35,
        0.004,
        &mut rng,
        &mut Biquad::bandpass(2400.0, 1.2),
    );
    ping(
        &mut buf,
        stop,
        1.0,
        &[
            (1420.0, 0.22, 0.009),
            (2330.0, 0.12, 0.03),
            (3550.0, 0.06, 0.02),
        ],
        false,
    );
    buf
}

/// Stepper-motor model: a pulse train at the step rate excites body resonances,
/// plus a soft tonal whine from the step harmonics.
struct Motor {
    body_low: Biquad,
    body_high: Biquad,
    last_phase: f32,
}

impl Motor {
    fn new() -> Self {
        Self {
            body_low: Biquad::bandpass(640.0, 3.0),
            body_high: Biquad::bandpass(1750.0, 5.0),
            last_phase: 0.0,
        }
    }

    /// `phase` is the step phase in cycles (monotonic or wrapped).
    fn tick(&mut self, phase: f32) -> f32 {
        let frac = phase.fract();
        // A strong impulse per step so the body resonances are as loud as the whine.
        let pulse = if frac < self.last_phase { 10.0 } else { 0.0 };
        self.last_phase = frac;
        let body = self.body_low.process(pulse) * 3.0 + self.body_high.process(pulse) * 2.0;
        let angle = TAU * frac;
        let whine = 0.30 * angle.sin()
            + 0.22 * (2.0 * angle).sin()
            + 0.12 * (3.0 * angle).sin()
            + 0.10 * (4.0 * angle).sin()
            + 0.05 * (6.0 * angle).sin();
        body + whine * 0.5
    }
}

/// Step rate of the reels at full speed (steps per second).
const REEL_STEP_HZ: f32 = 220.0;

fn reel_start(n: usize) -> Vec<f32> {
    let mut rng = Rng::new(0xA005);
    let length = n as f32 / SR;
    let ramp = |t: f32| (t / (length * 0.9)).min(1.0);

    // Motor spin-up: stepper whine and hiss gathering speed.
    let mut motor = Motor::new();
    let mut hiss = Biquad::bandpass(900.0, 0.8);
    let mut step_phase = 0.0f32;
    let mut spin_up = vec![0.0; n];
    for (i, slot) in spin_up.iter_mut().enumerate() {
        let progress = ramp(i as f32 / SR);
        let rate = 26.0 + (REEL_STEP_HZ - 26.0) * progress.powf(0.75);
        step_phase += rate / SR;
        let level = 0.45 + 0.55 * smoothstep(0.0, 0.6, progress);
        let bed = hiss.process(rng.bipolar()) * 0.25 * progress;
        *slot = level * (motor.tick(step_phase) + bed);
    }

    let mut buf = spin_up;

    // Symbol ticks accelerating up to the spin loop's rate.
    let mut tick_phase = 0.6f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let progress = ramp(t);
        tick_phase += (5.0 + (REEL_TICK_HZ - 5.0) * progress.powf(0.7)) / SR;
        if tick_phase >= 1.0 {
            tick_phase -= 1.0;
            reel_tick(&mut buf, t, 0.35 + 0.35 * progress, &mut rng, false);
        }
    }

    // Soft engagement clunk at the very start.
    thump(&mut buf, 0.003, 0.5, 140.0, 80.0, 0.012, 0.03);
    buf
}

fn reel_loop(n: usize) -> Vec<f32> {
    let mut rng = Rng::new(0xA006);
    let period = n as f32 / SR;
    // The stepper and drum both return to their starting phase.
    let ticks = (REEL_TICK_HZ * period).round() as usize;
    let step_hz = (REEL_STEP_HZ * period).round() / period;

    // Motor whine. Run the (periodic) motor signal through its filters twice
    // and keep the second pass, which is the steady-state — and therefore
    // periodic — response.
    let mut motor = Motor::new();
    let mut whine = vec![0.0; n];
    for pass in 0..2 {
        for (i, slot) in whine.iter_mut().enumerate() {
            let phase = (i as f64 * f64::from(step_hz) / f64::from(SR) + 0.37).fract() as f32;
            let x = motor.tick(phase);
            if pass == 1 {
                *slot = x;
            }
        }
    }

    // Soft band-passed whirr, made periodic by cross-fading.
    let overlap = n / 8;
    let whirr = noise(
        n + overlap,
        &mut rng,
        &mut [Biquad::bandpass(950.0, 0.7), Biquad::lowpass(2600.0, 0.7)],
    );
    let whirr = loopify(&whirr, n);

    let mut buf = vec![0.0; n];
    mix_at_rms(&mut buf, &whine, 0.105);
    mix_at_rms(&mut buf, &whirr, 0.060);
    for (i, sample) in buf.iter_mut().enumerate() {
        let angle = TAU * 3.0 * i as f32 / n as f32;
        *sample *= 0.90 + 0.10 * angle.cos();
    }
    let spacing = period / ticks as f32;
    for j in 0..ticks {
        let turn = TAU * (j % 21) as f32 / 21.0;
        let t = (j as f32 + 0.5 + 0.018 * turn.sin()) * spacing;
        let gain = 0.36 + 0.075 * turn.cos() + 0.05 * rng.bipolar();
        reel_tick(&mut buf, t, gain, &mut rng, true);
    }
    buf
}

fn reel_stop(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA007);

    // Clack: the brake catching the reel.
    let t0 = 0.003;
    thump(&mut buf, t0, 0.8, 170.0, 110.0, 0.006, 0.014);
    burst(
        &mut buf,
        t0,
        0.6,
        0.004,
        &mut rng,
        &mut Biquad::lowpass(3200.0, 0.8),
    );
    burst(
        &mut buf,
        t0,
        0.35,
        0.0015,
        &mut rng,
        &mut Biquad::highpass(3500.0, 0.8),
    );
    ping(
        &mut buf,
        t0,
        1.0,
        &[(1180.0, 0.14, 0.006), (2350.0, 0.1, 0.004)],
        false,
    );

    // The pawl seats, rebounds once, then the stamped frame rings down.
    ping(
        &mut buf,
        0.005,
        0.83,
        &[
            (348., 0.65, 0.022),
            (893., 0.34, 0.014),
            (1747., 0.20, 0.008),
        ],
        false,
    );
    burst(
        &mut buf,
        0.023,
        0.48,
        0.003,
        &mut rng,
        &mut Biquad::bandpass(1850., 1.1),
    );
    thump(&mut buf, 0.026, 0.28, 172., 96., 0.006, 0.013);
    buf
}

fn button_press(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA008);
    // Switch click with a short high tick tone...
    burst(
        &mut buf,
        0.003,
        0.55,
        0.002,
        &mut rng,
        &mut Biquad::highpass(1500.0, 0.8),
    );
    ping(
        &mut buf,
        0.003,
        1.0,
        &[
            (1800.0, 0.8, 0.009),
            (3600.0, 0.18, 0.004),
            (5400.0, 0.08, 0.003),
        ],
        false,
    );
    // ...then the cap bottoming out with a light hollow thock.
    thump(&mut buf, 0.014, 0.45, 420.0, 300.0, 0.006, 0.01);
    burst(
        &mut buf,
        0.014,
        0.15,
        0.005,
        &mut rng,
        &mut Biquad::lowpass(1100.0, 0.8),
    );
    buf
}

fn button_release(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA009);
    burst(
        &mut buf,
        0.003,
        0.3,
        0.0018,
        &mut rng,
        &mut Biquad::bandpass(2200.0, 0.8),
    );
    // A fourth below the press tone, and shorter.
    ping(
        &mut buf,
        0.003,
        1.0,
        &[(1350.0, 0.5, 0.007), (2700.0, 0.1, 0.004)],
        false,
    );
    thump(&mut buf, 0.011, 0.25, 520.0, 440.0, 0.004, 0.007);
    buf
}

fn medal_insert(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA00A);

    // Edge on the lip, rolling contacts on the guide, then a closed gate
    // inside the cabinet. Each surface excites a different set of modes.
    coin_strike(&mut buf, 0.005, 0.70, 1.0, 0.31, &mut rng);
    let mut guide = Biquad::bandpass(2460., 1.1);
    for (at, gain) in [(0.037, 0.30), (0.059, 0.35), (0.078, 0.40), (0.094, 0.44)] {
        burst(&mut buf, at, gain, 0.005, &mut rng, &mut guide);
    }
    coin_strike(&mut buf, 0.128, 0.63, 0.98, 0.59, &mut rng);
    thump(&mut buf, 0.189, 0.58, 380., 174., 0.007, 0.021);
    ping(
        &mut buf,
        0.194,
        0.52,
        &[(718., 0.42, 0.024), (1470., 0.21, 0.013)],
        false,
    );
    buf
}

fn medal_drop(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA00B);
    // The wheel advances one pocket. The tray has its own collision sound.
    thump(&mut buf, 0.003, 0.64, 260., 117., 0.009, 0.018);
    burst(
        &mut buf,
        0.005,
        0.53,
        0.006,
        &mut rng,
        &mut Biquad::bandpass(1150., 1.1),
    );
    burst(
        &mut buf,
        0.017,
        0.38,
        0.013,
        &mut rng,
        &mut Biquad::bandpass(2800., 0.8),
    );
    coin_strike(&mut buf, 0.030, 0.58, 0.97, 0.32, &mut rng);
    buf
}

fn medal_tray(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA00D);
    // A light coin rings high while the much larger folded tray answers below it.
    coin_strike(&mut buf, 0.002, 0.87, 1.02, 0.80, &mut rng);
    ping(
        &mut buf,
        0.003,
        1.0,
        &[
            (423., 0.48, 0.045),
            (789., 0.33, 0.032),
            (1295., 0.22, 0.025),
            (2037., 0.10, 0.016),
        ],
        false,
    );
    burst(
        &mut buf,
        0.003,
        0.50,
        0.0035,
        &mut rng,
        &mut Biquad::bandpass(1800., 1.0),
    );
    buf
}

fn medal_collision(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA00C);
    // The pile damps the faces quickly. Two touching edges still ring briefly.
    coin_strike(&mut buf, 0.002, 0.73, 1.04, 0.27, &mut rng);
    coin_strike(&mut buf, 0.005, 0.51, 0.986, 0.20, &mut rng);
    burst(
        &mut buf,
        0.009,
        0.40,
        0.006,
        &mut rng,
        &mut Biquad::bandpass(1430., 1.2),
    );
    buf
}

fn bonus_start(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA00D);
    let u = 0.075f32; // sixteenth note
    let at = |steps: usize| 0.004 + steps as f32 * u;

    // Three fast two-octave arpeggios, one per chord (C, F, G), each stepping
    // back once at the top; snare and bass on the beats.
    const SHAPE: [f32; 8] = [0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0, 19.0];
    let mut lead = Vec::new();
    let mut double = Vec::new();
    let mut bass = Vec::new();
    for (chord, root) in [C5, F4, G4].into_iter().enumerate() {
        for (k, offset) in SHAPE.into_iter().enumerate() {
            let start = at(chord * 8 + k);
            lead.push(note(start, u * 0.85, root + offset, 0.5));
            double.push(note(start, u * 0.85, root + offset - 12.0, 0.22));
        }
        for beat in 0..2 {
            let start = at(chord * 8 + beat * 4);
            bass.push(note(start, u * 3.6, root - 24.0, 0.42));
            snare(&mut buf, start, 0.6, &mut rng);
        }
    }
    voice(&mut buf, &lead, Wave::Pulse(0.5), RUN);
    voice(&mut buf, &double, Wave::Triangle, RUN);
    voice(&mut buf, &bass, Wave::Triangle, BASS);

    // Snare roll into the final chord.
    for (k, gain) in [0.28f32, 0.38, 0.5].into_iter().enumerate() {
        snare(&mut buf, at(21 + k), gain, &mut rng);
    }

    // Held bright C major chord with tremolo.
    let final_at = at(24);
    let hold = n as f32 / SR - final_at - 0.08;
    let mut chord = vec![0.0; n];
    voice(
        &mut chord,
        &[note(final_at, hold, C7, 0.5)],
        Wave::Pulse(0.5),
        HELD,
    );
    voice(
        &mut chord,
        &[
            note(final_at, hold, E6, 0.26),
            note(final_at, hold, G6, 0.26),
        ],
        Wave::Pulse(0.25),
        HELD,
    );
    voice(
        &mut chord,
        &[note(final_at, hold, C6, 0.35)],
        Wave::Triangle,
        HELD,
    );
    for (i, (dst, src)) in buf.iter_mut().zip(&chord).enumerate() {
        let t = i as f32 / SR - final_at;
        let depth = 0.45 * smoothstep(0.03, 0.12, t);
        *dst += src * (1.0 - depth * (0.5 - 0.5 * (TAU * 12.0 * t).cos()));
    }
    voice(
        &mut buf,
        &[
            note(final_at, hold, C3, 0.42),
            note(final_at, hold, C4, 0.3),
        ],
        Wave::Triangle,
        BASS,
    );
    snare(&mut buf, final_at, 0.7, &mut rng);

    // Sparkle and a little shimmer over the final chord.
    sparkle(
        &mut buf,
        final_at + 0.02,
        0.045,
        &[C7, E7, G7, C8, G7, E7, C8, G7, C8, E7, G7, C8, G7, C8],
        0.12,
    );
    let shimmer_start = sample_index(final_at);
    let mut shimmer = Biquad::highpass(8000.0, 0.7);
    for (i, slot) in buf.iter_mut().enumerate().skip(shimmer_start) {
        let t = (i - shimmer_start) as f32 / SR;
        *slot += 0.035 * (t / 0.002).min(1.0) * (-t / 0.25).exp() * shimmer.process(rng.bipolar());
    }

    fade_tail(&mut buf, 0.12);
    buf
}

fn bonus_end(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let u = 0.14f32;
    let at = |steps: usize| 0.004 + steps as f32 * u;
    let home = at(7);
    let hold = u * 4.2;

    // Down the tonic chord, a step-wise turn through F and G, home to C.
    let mut lead: Vec<Note> = [G6, E6, C6, G5, A5, F5, D5]
        .into_iter()
        .enumerate()
        .map(|(k, pitch)| note(at(k), u * 0.9, pitch, 0.5))
        .collect();
    lead.push(note(home, hold, C5, 0.55));
    let settle = Articulation {
        vibrato: 0.15,
        release: 0.12,
        decay: 0.12,
        sustain: 0.5,
    };
    voice(&mut buf, &lead, Wave::Pulse(0.25), settle);
    // A triangle an octave below rounds the lead off.
    let double: Vec<Note> = lead
        .iter()
        .map(|n| note(n.start, n.length, n.pitch - 12.0, 0.3))
        .collect();
    voice(&mut buf, &double, Wave::Triangle, settle);

    let bass = [
        note(at(0), u * 3.8, C3, 0.4),
        note(at(4), u * 1.9, F3, 0.4),
        note(at(6), u * 0.9, G3, 0.4),
        note(home, hold, C3, 0.4),
    ];
    voice(
        &mut buf,
        &bass,
        Wave::Triangle,
        Articulation {
            vibrato: 0.0,
            release: 0.14,
            decay: 0.3,
            sustain: 0.7,
        },
    );
    // Resolving chord under the last note.
    let pad = [
        note(home, hold, E4, 0.2),
        note(home, hold, G4, 0.2),
        note(home + 0.01, hold, C4, 0.2),
    ];
    voice(
        &mut buf,
        &pad,
        Wave::Sine,
        Articulation {
            vibrato: 0.0,
            release: 0.2,
            decay: 0.4,
            sustain: 0.6,
        },
    );
    // Small bell on the resolution.
    let f = pitch_to_hz(C6);
    ping(
        &mut buf,
        home,
        0.16,
        &[(f, 1.0, 0.3), (f * 2.0, 0.3, 0.15)],
        false,
    );

    fade_tail(&mut buf, 0.25);
    buf
}

fn menu_select(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let soft = Articulation {
        vibrato: 0.0,
        release: 0.012,
        decay: 0.03,
        sustain: 0.5,
    };
    voice(
        &mut buf,
        &[note(0.003, 0.03, A5, 0.6)],
        Wave::Triangle,
        soft,
    );
    voice(
        &mut buf,
        &[note(0.036, 0.03, E6, 0.6)],
        Wave::Triangle,
        soft,
    );
    voice(
        &mut buf,
        &[note(0.036, 0.03, E6 + 12.0, 0.08)],
        Wave::Sine,
        soft,
    );
    buf
}

fn win_small(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA010);

    // Rising major triad: two quick notes and a held, sparkling third.
    let quick = [note(0.003, 0.08, C6, 0.5), note(0.093, 0.08, E6, 0.5)];
    let last = [note(0.183, 0.26, G6, 0.52)];
    voice(&mut buf, &quick, Wave::Pulse(0.25), RUN);
    voice(&mut buf, &last, Wave::Pulse(0.25), HELD);
    let thicken = |notes: &[Note]| -> Vec<Note> {
        notes
            .iter()
            .map(|n| note(n.start, n.length, n.pitch, 0.25))
            .collect()
    };
    voice(&mut buf, &thicken(&quick), Wave::Triangle, RUN);
    voice(&mut buf, &thicken(&last), Wave::Triangle, HELD);

    // Sparkle on harmonics of the last note (octave, twelfth, double octave).
    sparkle(&mut buf, 0.2, 0.035, &[G7, D8, G8, D8, G8, G7, G8], 0.13);
    let mut shimmer = Biquad::highpass(8000.0, 0.7);
    hit(&mut buf, 0.183, 0.05, 0.08, &mut rng, &mut shimmer);
    buf
}

fn chance_lamp(n: usize) -> Vec<f32> {
    let mut rng = Rng::new(0xA011);
    let start = 0.003f32;
    let rise = 0.2f32;
    let (f_low, f_high) = (700.0f32, 4200.0f32);

    // "Kyu": a fast, accelerating upward sweep. "In": the ringing top note with
    // a quick tremolo, which also feeds the echo. The tone is an FM pair (for
    // edge) with a ring-modulated copy (for the metallic sparkle) and a fifth.
    let mut buf = vec![0.0; n];
    let mut tail = vec![0.0; n];
    let mut phase = 0.0f32;
    let mut fifth = 0.0f32;
    let mut shimmer = Biquad::bandpass(7200.0, 1.5);
    for (i, (dry, wet)) in buf.iter_mut().zip(tail.iter_mut()).enumerate() {
        let t = i as f32 / SR - start;
        if t < 0.0 {
            continue;
        }
        let u = t - rise;
        let freq = if u < 0.0 {
            f_low * (f_high / f_low).powf((t / rise).powf(1.6))
        } else {
            f_high * (1.0 - 0.05 * (1.0 - (-u / 0.1).exp()))
        };
        phase = (phase + TAU * freq / SR) % TAU;
        fifth = (fifth + TAU * freq * 1.5 / SR) % TAU;
        let index = if u < 0.0 {
            1.4 - 0.6 * t / rise
        } else {
            0.25 + 0.55 * (-u / 0.08).exp()
        };
        let carrier = (phase + index * (2.0 * phase).sin()).sin();
        let ring_mod = carrier * (TAU * 233.0 * t).sin();
        let tone = 0.7 * carrier + 0.4 * ring_mod + 0.2 * fifth.sin();
        if u < 0.0 {
            *dry = (t / 0.003).min(1.0) * (0.5 + 0.5 * t / rise) * tone;
        } else {
            let env = (-u / 0.085).exp() * (0.7 + 0.3 * (TAU * 31.0 * u).cos());
            let sparkle = 0.12 * (-u / 0.09).exp() * shimmer.process(rng.bipolar());
            *dry = env * tone + sparkle;
            // The echoes get a softened onset.
            *wet = *dry * smoothstep(0.0, 0.004, u);
        }
    }
    // Glassy accent at the top of the sweep.
    for layer in [&mut buf, &mut tail] {
        ping(
            layer,
            start + rise,
            0.25,
            &[
                (5210.0, 1.0, 0.09),
                (7830.0, 0.5, 0.06),
                (2607.0, 0.4, 0.07),
            ],
            false,
        );
    }
    add_echoes(&mut buf, &tail, 0.095, 0.42, 4);
    buf
}

/// One medal bet: a clean "pi". The game pitch-shifts it per medal.
fn bet(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    voice(
        &mut buf,
        &[note(0.003, 0.035, A5, 0.6)],
        Wave::SoftSquare,
        Articulation {
            vibrato: 0.0,
            release: 0.012,
            decay: 0.02,
            sustain: 0.35,
        },
    );
    buf
}

/// Replay signal: a falling "pi-po" (E6, B5), said twice.
fn replay(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let notes = [
        note(0.003, 0.055, E6, 0.5),
        note(0.073, 0.085, B5, 0.5),
        note(0.223, 0.055, E6, 0.42),
        note(0.293, 0.1, B5, 0.42),
    ];
    voice(&mut buf, &notes, Wave::SoftSquare, RUN);
    let glint: Vec<Note> = notes
        .iter()
        .map(|n| note(n.start, n.length, n.pitch + 12.0, n.amp * 0.16))
        .collect();
    voice(&mut buf, &glint, Wave::Sine, RUN);
    buf
}

fn win_big(n: usize) -> Vec<f32> {
    let mut buf = vec![0.0; n];
    let mut rng = Rng::new(0xA014);
    let u = 0.055f32;
    let at = |steps: usize| 0.004 + steps as f32 * u;

    // Up and down the C chord, up and down the F chord, a leading-note step,
    // then a held, sparkling top C.
    const RUN_PITCHES: [f32; 18] = [
        C5, E5, G5, C6, E6, G6, E6, C6, F5, A5, C6, F6, A6, C7, A6, F6, G6, B6,
    ];
    let final_at = at(RUN_PITCHES.len());
    let hold = 0.3;
    let lead: Vec<Note> = RUN_PITCHES
        .into_iter()
        .enumerate()
        .map(|(k, pitch)| note(at(k), u * 0.85, pitch, 0.5))
        .collect();
    let double: Vec<Note> = lead
        .iter()
        .map(|n| note(n.start, n.length, n.pitch - 12.0, 0.22))
        .collect();
    voice(&mut buf, &lead, Wave::Pulse(0.25), RUN);
    voice(&mut buf, &double, Wave::Triangle, RUN);
    voice(
        &mut buf,
        &[note(final_at, hold, C7, 0.55)],
        Wave::Pulse(0.25),
        HELD,
    );
    voice(
        &mut buf,
        &[
            note(final_at, hold, E6, 0.22),
            note(final_at, hold, G6, 0.22),
        ],
        Wave::Pulse(0.5),
        HELD,
    );
    let bass = [
        note(at(0), u * 7.6, C3, 0.4),
        note(at(8), u * 7.6, F3, 0.4),
        note(at(16), u * 1.8, G3, 0.4),
        note(final_at, hold, C3, 0.4),
        note(final_at, hold, C4, 0.3),
    ];
    voice(&mut buf, &bass, Wave::Triangle, BASS);
    for (steps, gain) in [(0, 0.5f32), (8, 0.5), (16, 0.36), (18, 0.65)] {
        snare(&mut buf, at(steps), gain, &mut rng);
    }

    sparkle(
        &mut buf,
        final_at + 0.02,
        0.04,
        &[C7, E7, G7, C8, G7, C8, E7, G7, C8],
        0.13,
    );
    let mut shimmer = Biquad::highpass(8000.0, 0.7);
    hit(&mut buf, final_at, 0.05, 0.15, &mut rng, &mut shimmer);

    fade_tail(&mut buf, 0.1);
    buf
}

/// Tempo and length of the bonus-mode music loop (4/4).
const BONUS_BPM: f32 = 150.0;
const BONUS_BARS: usize = 8;

/// Harmony of the bonus loop per half bar: bass root and arpeggiated triad.
const BONUS_HARMONY: [(f32, [f32; 3]); BONUS_BARS * 2] = {
    const TONIC: (f32, [f32; 3]) = (C3, [C4, E4, G4]);
    const SUBDOMINANT: (f32, [f32; 3]) = (F2, [C4, F4, A4]);
    const DOMINANT: (f32, [f32; 3]) = (G2, [B3, D4, G4]);
    const RELATIVE: (f32, [f32; 3]) = (A2, [C4, E4, A4]);
    [
        TONIC,
        TONIC,
        TONIC,
        TONIC,
        SUBDOMINANT,
        SUBDOMINANT,
        DOMINANT,
        DOMINANT,
        RELATIVE,
        RELATIVE,
        SUBDOMINANT,
        SUBDOMINANT,
        DOMINANT,
        DOMINANT,
        TONIC,
        DOMINANT,
    ]
};

/// Marks a rest in [`BONUS_MELODY`].
const REST: f32 = 0.0;

/// Original lead melody of the bonus loop: `(pitch, length in eighth notes)`,
/// eight eighths per bar.
const BONUS_MELODY: [(f32, f32); 49] = [
    // Bar 1 (C)
    (E5, 1.0),
    (G5, 1.0),
    (C6, 2.0),
    (G5, 1.0),
    (E5, 1.0),
    (G5, 2.0),
    // Bar 2 (C)
    (A5, 1.0),
    (G5, 1.0),
    (E5, 1.0),
    (G5, 1.0),
    (C6, 3.0),
    (REST, 1.0),
    // Bar 3 (F)
    (F5, 1.0),
    (A5, 1.0),
    (C6, 2.0),
    (A5, 1.0),
    (F5, 1.0),
    (A5, 2.0),
    // Bar 4 (G)
    (B5, 1.0),
    (A5, 1.0),
    (G5, 1.0),
    (B5, 1.0),
    (D6, 3.0),
    (REST, 1.0),
    // Bar 5 (Am)
    (E6, 2.0),
    (C6, 1.0),
    (A5, 1.0),
    (C6, 2.0),
    (E6, 1.0),
    (D6, 1.0),
    // Bar 6 (F)
    (C6, 2.0),
    (A5, 1.0),
    (F5, 1.0),
    (A5, 2.0),
    (C6, 1.0),
    (A5, 1.0),
    // Bar 7 (G)
    (B5, 1.0),
    (D6, 1.0),
    (G6, 2.0),
    (F6, 1.0),
    (D6, 1.0),
    (B5, 1.0),
    (D6, 1.0),
    // Bar 8 (C, then G to turn around)
    (C6, 3.0),
    (G5, 1.0),
    (G5, 1.0),
    (B5, 1.0),
    (D6, 1.0),
    (G5, 1.0),
];

fn bonus_loop(n: usize) -> Vec<f32> {
    let beat = 60.0 / BONUS_BPM;
    let (eighth, sixteenth) = (beat / 2.0, beat / 4.0);
    // Render with half a second of headroom, then fold the tails back in.
    let mut long = vec![0.0; n + SAMPLE_RATE as usize / 2];
    let mut rng = Rng::new(0xA015);

    let mut bass = Vec::new();
    let mut arp = Vec::new();
    for (half, (root, triad)) in BONUS_HARMONY.into_iter().enumerate() {
        let t0 = half as f32 * beat * 2.0;
        // Octave-bouncing bass in eighths, with a fifth before each bar line.
        for k in 0..4 {
            let offset = match (half % 2, k) {
                (1, 2) => 7.0,
                (_, 1 | 3) => 12.0,
                _ => 0.0,
            };
            let start = t0 + k as f32 * eighth;
            bass.push(note(start, eighth * 0.8, root + offset, 0.4));
        }
        // Sixteenth-note arpeggio: up the triad to the octave, twice.
        for k in 0..8 {
            let pitch = if k % 4 == 3 {
                triad[0] + 12.0
            } else {
                triad[k % 4]
            };
            let start = t0 + k as f32 * sixteenth;
            arp.push(note(start, sixteenth * 0.75, pitch, 0.3));
        }
    }
    voice(&mut long, &bass, Wave::Triangle, BASS);
    voice(&mut long, &arp, Wave::Pulse(0.125), RUN);

    let mut lead = Vec::new();
    let mut cursor = 0.0f32;
    for (pitch, eighths) in BONUS_MELODY {
        if pitch > REST {
            let start = cursor * eighth;
            lead.push(note(start, eighths * eighth * 0.9, pitch, 0.5));
        }
        cursor += eighths;
    }
    voice(&mut long, &lead, Wave::Pulse(0.5), HELD);

    // Light drums: hi-hat on every eighth (stronger off the beat), a soft kick
    // on beats 1 and 3 and a soft snare on 2 and 4.
    for q in 0..BONUS_BARS * 8 {
        let t = q as f32 * eighth;
        let (gain, tau) = if q % 2 == 1 {
            (0.14, 0.014)
        } else {
            (0.09, 0.008)
        };
        hit(
            &mut long,
            t,
            gain,
            tau,
            &mut rng,
            &mut Biquad::highpass(8000.0, 0.7),
        );
        match q % 4 {
            0 => thump(&mut long, t, 0.3, 150.0, 55.0, 0.02, 0.05),
            2 => snare(&mut long, t, 0.3, &mut rng),
            _ => {}
        }
    }

    fold_tail(&long, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_passes_its_sanity_checks() {
        for spec in &SPECS {
            let samples = render(spec);
            let bytes = wav_bytes(&samples);
            let report = verify_bytes(spec, &bytes).unwrap();
            assert!(
                (report.seconds - spec.seconds).abs() < 0.01,
                "{}",
                spec.name
            );
        }
    }

    #[test]
    fn synthesis_is_deterministic() {
        for name in ["reel_loop", "medal_drop", "chance_lamp", "win_big"] {
            let spec = spec(name).unwrap();
            assert_eq!(render(spec), render(spec), "{name}");
        }
    }

    #[test]
    fn wav_header_round_trips() {
        let samples = [0i16, 1000, -1000, 32_000, -32_000];
        let bytes = wav_bytes(&samples);
        assert_eq!(bytes.len(), 44 + 10);
        let wav = parse_wav(&bytes).unwrap();
        assert_eq!(wav.sample_rate, 44_100);
        assert_eq!(wav.channels, 1);
        assert_eq!(wav.bits, 16);
        assert_eq!(wav.samples, samples);
    }

    #[test]
    fn verification_rejects_bad_files() {
        let spec = spec("reel_stop").unwrap();
        // Silence.
        let silent = wav_bytes(&vec![0i16; (spec.seconds * SR) as usize]);
        assert!(verify_bytes(spec, &silent).is_err());
        // Wrong duration.
        let mut samples = render(spec);
        samples.extend(std::iter::repeat_n(0i16, samples.len()));
        assert!(verify_bytes(spec, &wav_bytes(&samples)).is_err());
        // A loop with a discontinuity at the seam.
        let looped = super::spec("reel_loop").unwrap();
        let mut samples = render(looped);
        let last = samples.len() - 1;
        samples[last] = if samples[0] > 0 { -20_000 } else { 20_000 };
        assert!(verify_bytes(looped, &wav_bytes(&samples)).is_err());
        assert!(parse_wav(b"not a wav").is_err());
    }

    #[test]
    fn biquad_lowpass_attenuates_high_frequencies() {
        let mut lp = Biquad::lowpass(500.0, 0.707);
        let level = |freq: f32, filter: &mut Biquad| {
            let out: Vec<f32> = (0..8000)
                .map(|i| filter.process((TAU * freq * i as f32 / SR).sin()))
                .collect();
            rms(&out[4000..])
        };
        let low = level(100.0, &mut lp);
        let high = level(5000.0, &mut Biquad::lowpass(500.0, 0.707));
        assert!(low > 0.6 && high < 0.02, "low {low} high {high}");
    }

    // -----------------------------------------------------------------------
    // Character tests. Nobody can audition these files in CI, so the tests
    // below measure what each sound is supposed to be: tick rate, pitch,
    // envelope shape, brightness, loop length.
    // -----------------------------------------------------------------------

    /// Renders a sound exactly as it is written to disk, as floats in -1..1.
    fn rendered(name: &str) -> Vec<f32> {
        render(spec(name).unwrap())
            .iter()
            .map(|s| f32::from(*s) / 32_767.0)
            .collect()
    }

    /// The part of `x` between two times in seconds.
    fn span(x: &[f32], from: f32, to: f32) -> &[f32] {
        &x[sample_index(from)..sample_index(to).min(x.len())]
    }

    fn energy(x: &[f32]) -> f64 {
        x.iter().map(|s| f64::from(*s).powi(2)).sum()
    }

    /// Dominant pitch between `lo` and `hi` Hz by autocorrelation: the shortest
    /// lag that is a local maximum within 10% of the best correlation, refined
    /// by parabolic interpolation.
    fn pitch_hz(x: &[f32], lo: f32, hi: f32) -> f32 {
        let min_lag = (SR / hi) as usize;
        let max_lag = ((SR / lo) as usize).min(x.len() / 2);
        let corr = |lag: usize| -> f32 {
            x[..x.len() - lag]
                .iter()
                .zip(&x[lag..])
                .map(|(a, b)| a * b)
                .sum()
        };
        let first = min_lag - 1;
        let values: Vec<f32> = (first..=max_lag + 1).map(corr).collect();
        let best = values[1..values.len() - 1]
            .iter()
            .fold(f32::MIN, |m, v| m.max(*v));
        for j in 1..values.len() - 1 {
            let (a, b, c) = (values[j - 1], values[j], values[j + 1]);
            if b >= 0.9 * best && b >= a && b >= c {
                let shift = 0.5 * (a - c) / (a - 2.0 * b + c);
                return SR / ((first + j) as f32 + shift);
            }
        }
        panic!("no pitch found between {lo} and {hi} Hz");
    }

    /// Amplitude of the sinusoidal component of `x` at `freq` (sample rate `rate`).
    fn tone_level_at(x: &[f32], freq: f32, rate: f32) -> f32 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        let step = std::f64::consts::TAU * f64::from(freq) / f64::from(rate);
        for (i, s) in x.iter().enumerate() {
            let (sin, cos) = (step * i as f64).sin_cos();
            re += f64::from(*s) * cos;
            im += f64::from(*s) * sin;
        }
        (2.0 * re.hypot(im) / x.len() as f64) as f32
    }

    fn tone_level(x: &[f32], freq: f32) -> f32 {
        tone_level_at(x, freq, SR)
    }

    /// Sample positions of bright transients: high-pass at `cutoff`, rectify,
    /// smooth, then report every upward crossing of `threshold` x the maximum,
    /// ignoring anything within `min_gap` seconds of the previous onset.
    fn bright_onsets(x: &[f32], cutoff: f32, threshold: f32, min_gap: f32) -> Vec<usize> {
        let mut high = Biquad::highpass(cutoff, 0.7);
        let mut smooth = OnePole::lowpass(150.0);
        let envelope: Vec<f32> = x
            .iter()
            .map(|s| smooth.process(high.process(*s).abs()))
            .collect();
        let level = threshold * envelope.iter().fold(0.0f32, |m, v| m.max(*v));
        let gap = sample_index(min_gap);
        let mut onsets = Vec::new();
        let mut i = 0;
        while i < envelope.len() {
            if envelope[i] >= level {
                onsets.push(i);
                i += gap;
            } else {
                i += 1;
            }
        }
        onsets
    }

    /// Slow amplitude envelope, decimated to `SR / step` samples per second.
    fn envelope(x: &[f32], smoothing_hz: f32, step: usize) -> Vec<f32> {
        let mut smooth = OnePole::lowpass(smoothing_hz);
        let full: Vec<f32> = x.iter().map(|s| smooth.process(s.abs())).collect();
        full.iter().step_by(step).copied().collect()
    }

    /// In-place radix-2 FFT.
    fn fft(re: &mut [f32], im: &mut [f32]) {
        let n = re.len();
        assert!(n.is_power_of_two() && im.len() == n);
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j |= bit;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let angle = -TAU / len as f32;
            for start in (0..n).step_by(len) {
                for k in 0..len / 2 {
                    let (sin, cos) = (angle * k as f32).sin_cos();
                    let (a, b) = (start + k, start + k + len / 2);
                    let (tr, ti) = (re[b] * cos - im[b] * sin, re[b] * sin + im[b] * cos);
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len <<= 1;
        }
    }

    /// Power-weighted mean frequency ("brightness") of the first 0.74 s.
    fn spectral_centroid(x: &[f32]) -> f32 {
        const SIZE: usize = 32_768;
        let mut re = vec![0.0f32; SIZE];
        let mut im = vec![0.0f32; SIZE];
        for (dst, src) in re.iter_mut().zip(x) {
            *dst = *src;
        }
        fft(&mut re, &mut im);
        let (mut weighted, mut total) = (0.0f64, 0.0f64);
        for k in 1..SIZE / 2 {
            let power = f64::from(re[k]).powi(2) + f64::from(im[k]).powi(2);
            weighted += k as f64 * power;
            total += power;
        }
        (weighted / total) as f32 * SR / SIZE as f32
    }

    /// True if a note of MIDI `pitch` sounds between the two times: its
    /// fundamental is present and stands clear of the levels a whole tone
    /// above and below it.
    fn sounds(x: &[f32], from: f32, to: f32, pitch: f32) -> bool {
        let part = span(x, from, to);
        let level = tone_level(part, pitch_to_hz(pitch));
        let beside = tone_level(part, pitch_to_hz(pitch + 2.0))
            .max(tone_level(part, pitch_to_hz(pitch - 2.0)));
        level > 0.02 && level > 2.5 * beside
    }

    fn assert_near(actual: f32, expected: f32, tolerance: f32, what: &str) {
        assert!(
            (actual / expected - 1.0).abs() <= tolerance,
            "{what}: measured {actual:.1}, expected {expected:.1} within {:.0}%",
            tolerance * 100.0
        );
    }

    #[test]
    fn analysis_helpers_measure_known_signals() {
        let sine = |freq: f32, count: usize| -> Vec<f32> {
            (0..count)
                .map(|i| (TAU * freq * i as f32 / SR).sin())
                .collect()
        };
        let a440 = sine(440.0, 8000);
        assert_near(pitch_hz(&a440, 200.0, 2000.0), 440.0, 0.005, "pitch");
        assert_near(tone_level(&a440, 440.0), 1.0, 0.02, "tone level");
        assert!(tone_level(&a440, 660.0) < 0.02);
        assert_near(
            spectral_centroid(&sine(3000.0, 32_768)),
            3000.0,
            0.02,
            "centroid",
        );
        // Clicks every 50 ms on top of a low tone.
        let mut clicks = sine(200.0, 22_050);
        for k in 0..10 {
            clicks[1000 + k * 2205] += 1.0;
        }
        assert_eq!(bright_onsets(&clicks, 3000.0, 0.35, 0.02).len(), 10);
    }

    #[test]
    fn every_spec_has_a_unique_name() {
        for (i, a) in SPECS.iter().enumerate() {
            for b in &SPECS[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn reel_loop_ticks_28_times_per_second() {
        let x = rendered("reel_loop");
        let n = x.len();
        let rate = SAMPLE_RATE as usize;
        assert_eq!(n * 28 % rate, 0, "fractional tick count");
        let ticks = n * 28 / rate;
        assert_eq!(ticks, 3 * 21);
        let mut high = Biquad::highpass(3000.0, 0.7);
        let high: Vec<_> = x.iter().map(|v| high.process(*v)).collect();
        // The bearing noise changes per tick. Fold each whole turn to check
        // whether contact transients stay in phase above the continuous bed.
        for turn in 0..3 {
            let mut strikes = 0.;
            let mut background = 0.;
            for step in 0..21 {
                let base = (turn * 21 + step) * rate / 28;
                let centre = base + rate / 56;
                strikes += energy(&high[centre - 40..centre + 220]);
                background += energy(&high[base + 100..base + 360]);
            }
            assert!(
                strikes > 1.5 * background,
                "turn {turn}: {strikes} vs {background}"
            );
        }
    }

    #[test]
    fn reel_start_reaches_the_running_motor_pitch() {
        let x = rendered("reel_start");
        let early = pitch_hz(span(&x, 0.10, 0.12), 65., 200.);
        let running = pitch_hz(span(&x, 0.36, 0.395), 140., 280.);
        assert!(early < running * 0.75, "{early} vs {running}");
        assert_near(running, REEL_STEP_HZ, 0.05, "motor at speed");
        let bed = rendered("reel_loop");
        assert_near(
            pitch_hz(span(&bed, 0.1, 0.19), 150., 280.),
            running,
            0.05,
            "running loop",
        );
    }

    #[test]
    fn brake_damps_quickly_and_bet_retains_its_pitch() {
        let stop = rendered("reel_stop");
        assert!(energy(span(&stop, 0., 0.060)) / energy(&stop) > 0.94);
        assert!(rms(span(&stop, 0.023, 0.036)) > 0.03);

        let bet = rendered("bet");
        let pitch = pitch_hz(span(&bet, 0.005, 0.06), 200.0, 2000.0);
        assert_near(pitch, 880.0, 0.02, "bet");
    }

    #[test]
    fn button_press_has_a_short_high_tick_tone() {
        let press = rendered("button_press");
        let pitch = pitch_hz(span(&press, 0.004, 0.013), 800.0, 4000.0);
        assert_near(pitch, 1800.0, 0.05, "button_press");
        // The release is the lower, duller of the two.
        let release = rendered("button_release");
        assert!(spectral_centroid(&release) < spectral_centroid(&press));
    }

    #[test]
    fn lever_has_distinct_latch_and_end_stop() {
        let x = rendered("lever_pull");
        let travel = rms(span(&x, 0.048, 0.065));
        assert!(rms(span(&x, 0.003, 0.017)) > 2.0 * travel);
        assert!(rms(span(&x, 0.070, 0.089)) > 2.0 * travel);
        assert!(rms(span(&x, 0.19, 0.23)) < 0.04);
    }

    #[test]
    fn tray_rings_longer_than_loose_medals_or_the_chute() {
        let tray = rendered("medal_tray");
        let pile = rendered("medal_collision");
        let chute = rendered("medal_drop");
        let tail = |x: &[f32]| energy(span(x, 0.06, 0.10)) / energy(x);
        assert!(tail(&tray) > 3.0 * tail(&pile));
        assert!(tail(&tray) > tail(&chute));
        assert!(spectral_centroid(&pile) > spectral_centroid(&chute));
        let ringing = span(&tray, 0.014, 0.075);
        for freq in [423., 789., 2100. * 1.02] {
            assert!(tone_level(ringing, freq) > 2.5 * tone_level(ringing, freq * 1.15));
        }
    }

    #[test]
    fn medal_insert_has_a_bright_lip_and_a_muffled_gate() {
        let x = rendered("medal_insert");
        let lip = span(&x, 0.005, 0.025);
        let gate = span(&x, 0.19, 0.22);
        assert!(rms(lip) > 0.05 && rms(gate) > 0.05);
        assert!(spectral_centroid(lip) > spectral_centroid(gate) * 2.0);
    }

    #[test]
    fn win_small_is_three_ascending_notes() {
        let x = rendered("win_small");
        let notes = [(0.02, 0.08, C6), (0.11, 0.17, E6), (0.22, 0.40, G6)];
        let mut previous = 0.0;
        for (from, to, pitch) in notes {
            let measured = pitch_hz(span(&x, from, to), 300.0, 4000.0);
            assert_near(measured, pitch_to_hz(pitch), 0.03, "win_small note");
            assert!(measured > previous * 1.1, "{measured} after {previous}");
            previous = measured;
        }
    }

    #[test]
    fn replay_is_a_falling_two_tone_signal_said_twice() {
        let x = rendered("replay");
        for (from, to, pitch) in [
            (0.01, 0.055, E6),
            (0.085, 0.15, B5),
            (0.23, 0.275, E6),
            (0.305, 0.385, B5),
        ] {
            let measured = pitch_hz(span(&x, from, to), 300.0, 4000.0);
            assert_near(measured, pitch_to_hz(pitch), 0.03, "replay note");
        }
        // A clear gap separates the two statements.
        let gap = rms(span(&x, 0.19, 0.22));
        assert!(gap < 0.1 * rms(span(&x, 0.01, 0.055)), "gap level {gap}");
    }

    #[test]
    fn chance_lamp_is_a_sharp_rising_shimmer() {
        let x = rendered("chance_lamp");
        // The sweep rises by well over an octave within 0.2 s...
        let early = pitch_hz(span(&x, 0.01, 0.04), 400.0, 6000.0);
        let late = pitch_hz(span(&x, 0.17, 0.2), 400.0, 6000.0);
        assert!(early < 1100.0, "early {early}");
        assert!(late > 2.5 * early, "late {late} vs early {early}");
        // ...and it is the brightest of the tonal signals.
        let centroid = spectral_centroid(&x);
        for other in ["win_small", "win_big", "replay", "bet", "reel_stop"] {
            let theirs = spectral_centroid(&rendered(other));
            assert!(centroid > theirs, "{other}: {theirs} vs {centroid}");
        }
        // Echo tail: still audible 0.3 s after the top of the sweep.
        let tail = rms(span(&x, 0.5, 0.6));
        assert!(tail > 0.01, "tail level {tail}");
    }

    #[test]
    fn bonus_start_runs_up_three_chords_to_a_held_tremolo_chord() {
        let x = rendered("bonus_start");
        // Three chords (C, F, G), each a run of eight sixteenth notes rising
        // two octaves through the major triad and stepping back once.
        let u = 0.075;
        for (chord, root) in [C5, F4, G4].into_iter().enumerate() {
            let shape = [0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0, 19.0];
            for (k, offset) in shape.into_iter().enumerate() {
                let t0 = 0.004 + (chord * 8 + k) as f32 * u;
                assert!(
                    sounds(&x, t0 + 0.008, t0 + 0.062, root + offset),
                    "chord {chord} note {k} is missing"
                );
            }
        }
        // Snare on the beats: a noisy onset every four sixteenths.
        // (The last beat is left out: a snare roll fills the gap after it.)
        for beat in 0..5 {
            let t0 = 0.004 + beat as f32 * 4.0 * u;
            let mut high = Biquad::highpass(6000.0, 0.7);
            let sizzle: Vec<f32> = x.iter().map(|s| high.process(*s)).collect();
            let on = rms(span(&sizzle, t0, t0 + 0.03));
            let off = rms(span(&sizzle, t0 + 2.0 * u, t0 + 2.0 * u + 0.03));
            assert!(on > 1.5 * off, "beat {beat}: {on} vs {off}");
        }
        // Final chord: C major (C7 on top), held, with a 12 Hz tremolo.
        // (Short window: the chord has vibrato, which smears a long one.)
        for pitch in [C6, E6, G6, C7] {
            assert!(
                sounds(&x, 2.0, 2.06, pitch),
                "chord note {pitch} is missing"
            );
        }
        let held = span(&x, 2.0, 2.6);
        let step = 90;
        let env = envelope(held, 60.0, step);
        let mean = env.iter().sum::<f32>() / env.len() as f32;
        let centred: Vec<f32> = env.iter().map(|v| v - mean).collect();
        let rate = SR / step as f32;
        let tremolo = tone_level_at(&centred, 12.0, rate);
        assert!(tremolo > 0.08 * mean, "tremolo depth {}", tremolo / mean);
        for other in [8.0, 17.0] {
            assert!(tremolo > 2.0 * tone_level_at(&centred, other, rate));
        }
    }

    #[test]
    fn bonus_end_descends_and_resolves_on_the_tonic() {
        let x = rendered("bonus_end");
        let u = 0.14;
        // The lead falls from G6 to C5. Each note is present in its own slot
        // and was not yet sounding in the slot before.
        let phrase = [G6, E6, C6, G5, A5, F5, D5, C5];
        assert!(phrase[0] > phrase[7] + 12.0);
        for (k, pitch) in phrase.into_iter().enumerate() {
            let t0 = 0.004 + k as f32 * u;
            let level = tone_level(span(&x, t0 + 0.01, t0 + 0.12), pitch_to_hz(pitch));
            assert!(level > 0.05, "note {k} level {level}");
            if k > 0 {
                let before = tone_level(span(&x, t0 - u + 0.01, t0 - 0.02), pitch_to_hz(pitch));
                assert!(level > 2.5 * before, "note {k}: {level} vs {before} before");
            }
        }
        // The last second is a C chord: its pitch class is C in some octave.
        let home = pitch_hz(span(&x, 1.05, 1.5), 60.0, 600.0);
        let octaves = (home / pitch_to_hz(C4)).log2();
        assert!(
            (octaves - octaves.round()).abs() < 0.04,
            "final pitch {home} Hz is not a C"
        );
    }

    #[test]
    fn win_big_runs_over_two_chords_to_a_top_c() {
        let x = rendered("win_big");
        let u = 0.055;
        // Up and down the C chord, up and down the F chord, then G6-B6.
        let run = [
            C5, E5, G5, C6, E6, G6, E6, C6, F5, A5, C6, F6, A6, C7, A6, F6, G6, B6,
        ];
        for (k, pitch) in run.into_iter().enumerate() {
            let t0 = 0.004 + k as f32 * u;
            assert!(
                sounds(&x, t0 + 0.006, t0 + 0.048, pitch),
                "run note {k} is missing"
            );
        }
        // Held final C7 with sparkle above it.
        assert!(sounds(&x, 1.02, 1.28, C7), "final note");
        let held = span(&x, 1.02, 1.28);
        assert!(tone_level(held, pitch_to_hz(C8)) > 0.005);
    }

    #[test]
    fn bonus_melody_fills_eight_bars() {
        let eighths: f32 = BONUS_MELODY.iter().map(|(_, length)| length).sum();
        assert_eq!(eighths, (BONUS_BARS * 8) as f32);
        // Every bar line falls between notes (no note is tied across a bar).
        let mut cursor = 0.0;
        let mut bars = 0;
        for (_, length) in BONUS_MELODY {
            cursor += length;
            if cursor % 8.0 == 0.0 {
                bars += 1;
            }
        }
        assert_eq!(bars, BONUS_BARS);
    }

    #[test]
    fn bonus_loop_is_eight_seamless_bars_at_150_bpm() {
        let spec = spec("bonus_loop").unwrap();
        let samples = render(spec);
        // Seam continuity (step and slope) is part of `verify`.
        verify_bytes(spec, &wav_bytes(&samples)).unwrap();

        // Exactly 32 beats; one beat at 150 BPM is exactly 17 640 samples.
        let beat = (SR * 60.0 / BONUS_BPM).round() as usize;
        assert_eq!(beat, 17_640);
        assert_eq!(samples.len() % beat, 0);
        assert_eq!(samples.len() / beat, BONUS_BARS * 4);

        // The loop really is periodic: rotating it by half its length moves
        // the old seam into the middle, and the new file still verifies.
        let mut rotated = samples.clone();
        rotated.rotate_left(samples.len() / 2 + 1234);
        verify_bytes(spec, &wav_bytes(&rotated)).unwrap();

        // Rhythm: the amplitude envelope repeats every eighth note (0.2 s)
        // far more strongly than at unrelated lags.
        let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32_767.0).collect();
        let step = 90; // one eighth note = 98 envelope samples
        let env = envelope(&x, 30.0, step);
        let mean = env.iter().sum::<f32>() / env.len() as f32;
        let centred: Vec<f32> = env.iter().map(|v| v - mean).collect();
        let corr = |lag: usize| -> f32 {
            centred[..centred.len() - lag]
                .iter()
                .zip(&centred[lag..])
                .map(|(a, b)| a * b)
                .sum()
        };
        let eighth = beat / 2 / step;
        let on_grid = corr(eighth).min(corr(2 * eighth));
        let off_grid = corr(eighth * 3 / 4).max(corr(eighth * 5 / 4));
        assert!(
            on_grid > 0.0 && on_grid > 2.0 * off_grid,
            "{on_grid} vs {off_grid}"
        );

        // Bass and melody are present: tonic bass C3 in bar 1, and the first
        // melody note E5.
        assert!(tone_level(span(&x, 0.01, 0.16), pitch_to_hz(C3)) > 0.03);
        assert!(sounds(&x, 0.01, 0.18, E5), "first melody note");
        assert!(sounds(&x, 0.21, 0.38, G5), "second melody note");
    }

    #[test]
    fn levels_follow_the_mix_plan() {
        for spec in &SPECS {
            let expected = match spec.name {
                "ambient" => -18.0,
                "bonus_loop" => -9.0,
                _ => -3.0,
            };
            assert_eq!(spec.peak_db, expected, "{}", spec.name);
        }
        // Both loops are flagged as loops, and nothing else is.
        let loops: Vec<&str> = SPECS.iter().filter(|s| s.looped).map(|s| s.name).collect();
        assert_eq!(loops, ["ambient", "reel_loop", "bonus_loop"]);
    }

    #[test]
    #[ignore = "diagnostic dump, run with --ignored --nocapture"]
    fn dump_measurements() {
        for spec in &SPECS {
            let x = rendered(spec.name);
            let peak = x.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            println!(
                "{:16} {:7.3}s peak {:6.2} dB rms {:6.2} dB centroid {:7.0} Hz",
                spec.name,
                x.len() as f32 / SR,
                20.0 * peak.log10(),
                20.0 * rms(&x).log10(),
                spectral_centroid(&x)
            );
        }
    }
}
