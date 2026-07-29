//! Golden-corpus replay tests.
//!
//! Every byte-identity test elsewhere is *relative* (streaming vs offline, or
//! render(d) == render(d) in one process). These tests pin the exact rendered
//! bytes of representative documents against checked-in hashes, so a kernel
//! change that shifts BOTH paths together — the failure the engine-version
//! gate exists to prevent — fails CI instead of silently breaking every
//! existing user document.
//!
//! ## Known limitation: byte-identity is per-platform
//!
//! The DSP calls platform libm (`sin`/`cos`/`exp`/`powf`/`tanh`), whose last
//! bits differ between macOS-arm64 and linux-x86_64 — pinning this corpus is
//! what surfaced it. Documents built purely from integer RNG, polynomial
//! oscillators (PolyBLEP saw/square), and rational filter math replay
//! identically across platforms; sine/FM/piano/bass/kit content does not.
//! So each case carries reference pins (macOS aarch64, where the corpus was
//! authored) plus linux-x86_64 overrides where libm makes them differ; other
//! platforms only check in-process determinism. Making the invariant truly
//! cross-platform means deterministic transcendental kernels behind a future
//! engine revision.
//!
//! When a mismatch is *intentional* (a new engine revision changing only docs
//! that opt in), re-pin by running:
//! `cargo test -p tono-core --test golden -- --nocapture` and copying the
//! printed table (on a CI failure, the log prints the same table).

use tono_core::dsl::{Adsr, SeqWave, SoundDoc};
use tono_core::render::render_product;
use tono_core::song::{Song, note};

/// FNV-1a over the little-endian bit patterns of the samples.
fn hash_signal(samples: &[f32]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for s in samples {
        for b in s.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }
    h
}

fn parse(json: &str) -> SoundDoc {
    let doc: SoundDoc = serde_json::from_str(json).expect("corpus doc parses");
    doc.validate().expect("corpus doc validates");
    doc
}

/// Pinned hashes: (mono, stereo pair when the render has one).
type Pins = (u64, Option<(u64, u64)>);

struct Case {
    name: &'static str,
    json: &'static str,
    /// Reference pins: macOS aarch64.
    mac: Pins,
    /// linux x86_64 pins where libm divergence changes the bytes
    /// (`None` = bit-identical to `mac` there).
    linux: Option<Pins>,
}

/// Select the pin this platform must reproduce, or `None` on an unpinned
/// platform — the single place the pinned-platform list lives.
fn platform_pin<T>(mac: T, linux: Option<T>) -> Option<T> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(mac)
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        linux
    } else {
        None
    }
}

/// The pins this platform must reproduce, or `None` on an unpinned platform.
fn expected(c: &Case) -> Option<Pins> {
    platform_pin(c.mac, Some(c.linux.unwrap_or(c.mac)))
}

/// One document per kernel family × engine revision. Durations are short so
/// the whole corpus renders in well under a second.
const CORPUS: &[Case] = &[
    Case {
        name: "blip-legacy",
        json: r#"{ "name": "blip-legacy", "duration": 0.2, "root": { "type": "mul", "inputs": [
            { "type": "sine", "freq": 880 },
            { "type": "env", "a": 0.002, "d": 0.08, "s": 0.0, "r": 0.05 } ] } }"#,
        mac: (0xecdcebf2ca9d38c4, None),
        linux: Some((0x17933ff851439897, None)),
    },
    Case {
        name: "noise-v0",
        json: r#"{ "name": "noise-v0", "duration": 0.15, "seed": 7,
            "root": { "type": "noise", "color": "pink" } }"#,
        mac: (0xa3f0e6e7c8d7e183, None),
        linux: None,
    },
    Case {
        name: "noise-v2",
        json: r#"{ "name": "noise-v2", "duration": 0.15, "seed": 7, "engine": 2,
            "root": { "type": "noise", "color": "pink" } }"#,
        mac: (0xb73db8207746cdbf, None),
        linux: None,
    },
    Case {
        name: "pwm-slide",
        json: r#"{ "name": "pwm-slide", "duration": 0.25, "engine": 2, "root": { "type": "square",
            "freq": { "slide": { "from": 220, "to": 440, "secs": 0.2, "curve": "exp" } },
            "duty": { "lfo": { "rate": 8, "depth": 0.3, "center": 0.5 } } } }"#,
        mac: (0x6ca3dc84607bbe3c, None),
        linux: Some((0xadc525301c30dc7b, None)),
    },
    Case {
        name: "fm-bell",
        json: r#"{ "name": "fm-bell", "duration": 0.3, "engine": 2, "root": { "type": "fm",
            "freq": 660, "ratio": 3.5,
            "index": { "slide": { "from": 6, "to": 0.5, "secs": 0.25 } } } }"#,
        mac: (0x9ee1ff99f21948fc, None),
        linux: Some((0x56d0e63a1796bd80, None)),
    },
    Case {
        name: "supersaw-fx",
        json: r#"{ "name": "supersaw-fx", "duration": 0.3, "engine": 2, "root": { "type": "chain", "stages": [
            { "type": "super", "freq": 110, "voices": 7, "detune_cents": 25 },
            { "type": "lowpass", "cutoff": 1200, "q": 0.9 },
            { "type": "delay", "secs": 0.09, "feedback": 0.35 },
            { "type": "reverb", "room": 0.4, "mix": 0.25 } ] } }"#,
        mac: (0xb0f3fe096484eb4f, None),
        linux: None,
    },
    Case {
        name: "dust-crackle",
        json: r#"{ "name": "dust-crackle", "duration": 0.25, "seed": 11, "engine": 2,
            "root": { "type": "chain", "stages": [
            { "type": "dust", "density": 220, "decay": 0.004 },
            { "type": "highpass", "cutoff": 1800, "q": 0.7 } ] } }"#,
        mac: (0xf983e1ee44a7124e, None),
        linux: None,
    },
    Case {
        name: "seq-saw-groove",
        json: r#"{ "name": "seq-saw-groove", "duration": 1.0, "seed": 3, "engine": 2,
            "root": { "type": "seq", "bpm": 240, "wave": "sawtooth", "swing": 0.55, "humanize": 0.2,
            "env": { "a": 0.005, "d": 0.05, "s": 0.6, "r": 0.08 },
            "notes": [
                { "step": 0, "len": 2, "pitch": "C2" },
                { "step": 2, "len": 2, "pitch": "G2", "gain": 0.8 },
                { "step": 4, "len": 2, "pitch": "A#2" },
                { "step": 6, "len": 2, "pitch": "C3", "gain": 0.7 } ] } }"#,
        mac: (0x0667b70a01616ae1, None),
        linux: None,
    },
    Case {
        name: "seq-piano-v3",
        json: r#"{ "name": "seq-piano-v3", "duration": 1.0, "engine": 3,
            "root": { "type": "seq", "bpm": 240, "wave": "piano",
            "env": { "a": 0.002, "s": 1.0, "r": 0.2 },
            "notes": [
                { "step": 0, "len": 4, "pitch": "C4" },
                { "step": 0, "len": 4, "pitch": "E4", "gain": 0.9 },
                { "step": 4, "len": 4, "pitch": "G3", "gain": 0.7 } ] } }"#,
        mac: (0x3f76e2c14bb45187, None),
        linux: Some((0x55a1dea67a6118b9, None)),
    },
    Case {
        name: "seq-kit-808",
        json: r#"{ "name": "seq-kit-808", "duration": 1.0, "seed": 5, "engine": 3,
            "root": { "type": "seq", "bpm": 240, "wave": "kit", "kit": "808",
            "env": { "a": 0.001, "d": 0.1, "s": 0.5, "r": 0.1 },
            "notes": [
                { "step": 0, "len": 1, "pitch": "midi:36" },
                { "step": 2, "len": 1, "pitch": "midi:38", "gain": 0.9 },
                { "step": 4, "len": 1, "pitch": "midi:36" },
                { "step": 5, "len": 1, "pitch": "midi:42", "gain": 0.6 },
                { "step": 6, "len": 1, "pitch": "midi:38" } ] } }"#,
        mac: (0x7cc71e1213407ad3, None),
        linux: Some((0x55dbea35e8061d42, None)),
    },
    Case {
        name: "seq-bass-v3",
        json: r#"{ "name": "seq-bass-v3", "duration": 1.0, "engine": 3,
            "root": { "type": "seq", "bpm": 240, "wave": "bass",
            "env": { "a": 0.005, "d": 0.08, "s": 0.7, "r": 0.1 },
            "notes": [
                { "step": 0, "len": 3, "pitch": "E1" },
                { "step": 4, "len": 3, "pitch": "G1", "gain": 0.85 } ] } }"#,
        mac: (0x221a786a5641fc9d, None),
        linux: Some((0x3ee2634cb20f6fd5, None)),
    },
    Case {
        // Pins the whole mixer path: pan law, automation lanes, master
        // reverb's decorrelated tails, per-channel normalize (the engine ≤ 3
        // behavior), and the final peak limit — mono mid plus both channels.
        name: "tracks-mix",
        json: r#"{ "name": "tracks-mix", "duration": 0.5, "seed": 9, "engine": 3,
            "normalize": { "target_lufs": -14, "ceiling_dbtp": -1.0 },
            "root": { "type": "tracks", "tracks": [
                { "id": "pad", "node": { "type": "sine", "freq": 220 }, "pan": -0.8, "gain": 0.3 },
                { "id": "hiss", "node": { "type": "noise", "color": "white" }, "pan": 0.9, "gain": 0.6,
                  "automation": [ { "target": "gain", "points": [
                      { "t": 0.0, "v": 0.1 }, { "t": 0.4, "v": 0.8 } ] } ] },
                { "id": "lead", "node": { "type": "square", "freq": 440, "duty": 0.25 }, "gain": 0.4, "at": 0.1 }
            ], "master": [ { "type": "reverb", "room": 0.3, "mix": 0.2 } ] } }"#,
        mac: (
            0xfbd248e0796957b7,
            Some((0xf7767b2a46f37bc2, 0x413e706e94cb48a9)),
        ),
        linux: Some((
            0x24e29f5b35976ce7,
            Some((0x4e168a3117a2f7a7, 0x413e706e94cb48a9)),
        )),
    },
    Case {
        name: "loop-bed",
        json: r#"{ "name": "loop-bed", "duration": 0.6, "seed": 2, "engine": 2,
            "playback": { "mode": "loop", "start_secs": 0.1, "end_secs": 0.5, "crossfade_secs": 0.08 },
            "root": { "type": "chain", "stages": [
                { "type": "noise", "color": "brown" },
                { "type": "lowpass", "cutoff": 600, "q": 0.8 } ] } }"#,
        mac: (0x430d046059c90676, None),
        linux: None,
    },
    Case {
        // The same mixer as tracks-mix, opted into engine 4: joint gated
        // loudness with one shared gain and an oversampled true-peak ceiling.
        name: "tracks-mix-v4",
        json: r#"{ "name": "tracks-mix-v4", "duration": 0.5, "seed": 9, "engine": 4,
            "normalize": { "target_lufs": -14, "ceiling_dbtp": -1.0 },
            "root": { "type": "tracks", "tracks": [
                { "id": "pad", "node": { "type": "sine", "freq": 220 }, "pan": -0.8, "gain": 0.3 },
                { "id": "hiss", "node": { "type": "noise", "color": "white" }, "pan": 0.9, "gain": 0.6,
                  "automation": [ { "target": "gain", "points": [
                      { "t": 0.0, "v": 0.1 }, { "t": 0.4, "v": 0.8 } ] } ] },
                { "id": "lead", "node": { "type": "square", "freq": 440, "duty": 0.25 }, "gain": 0.4, "at": 0.1 }
            ], "master": [ { "type": "reverb", "room": 0.3, "mix": 0.2 } ] } }"#,
        mac: (
            0x9cf01dcf618e7053,
            Some((0x6851c603baadc1f8, 0xe7e3536620aae2c7)),
        ),
        linux: Some((
            0x3c8de23ac5905e22,
            Some((0x9576fcfbea2777d3, 0xe7e3536620aae2c7)),
        )),
    },
    Case {
        name: "normalize-mono-v4",
        json: r#"{ "name": "normalize-mono-v4", "duration": 0.5, "engine": 4,
            "normalize": { "target_lufs": -16, "ceiling_dbtp": -1.0 },
            "root": { "type": "chain", "stages": [
                { "type": "sine", "freq": 440 }, { "type": "gain", "amount": 0.05 } ] } }"#,
        mac: (0xb6b0676f4086d076, None),
        linux: Some((0x1c0c5f125c813202, None)),
    },
    // v1.10 additions: every new deterministic node and voice gets a pin. All
    // stamped with the current schema/engine pins, the way `SoundDoc::new`
    // writes new documents.
    Case {
        name: "wavetable-basic",
        json: r#"{ "name": "wavetable-basic", "duration": 0.25, "version": 2, "engine": 4,
            "root": { "type": "wavetable", "wave": "basic", "freq": 220, "position": 0.5 } }"#,
        mac: (0x41a5677aee1b1037, None),
        linux: Some((0x1287ed13afb3ead8, None)),
    },
    Case {
        name: "wavetable-morph",
        json: r#"{ "name": "wavetable-morph", "duration": 0.3, "version": 2, "engine": 4,
            "root": { "type": "wavetable", "wave": "harmonics", "freq": 110,
                "position": { "lfo": { "rate": 2, "depth": 0.5, "center": 0.5 } } } }"#,
        mac: (0xd38f9cd51e44ad24, None),
        linux: Some((0xced37eb5dea735d1, None)),
    },
    Case {
        name: "tremolo-wobble",
        json: r#"{ "name": "tremolo-wobble", "duration": 0.3, "version": 2, "engine": 4,
            "root": { "type": "chain", "stages": [
                { "type": "sine", "freq": 440 },
                { "type": "tremolo", "rate": 6, "depth": 0.8 } ] } }"#,
        mac: (0x2fb9fc3269c3669b, None),
        linux: Some((0x9098b89ad7d576b6, None)),
    },
    Case {
        name: "convolve-space",
        json: r#"{ "name": "convolve-space", "duration": 0.5, "seed": 4, "version": 2, "engine": 4,
            "root": { "type": "chain", "stages": [
                { "type": "mul", "inputs": [
                    { "type": "noise", "color": "white" },
                    { "type": "env", "a": 0.001, "d": 0.02, "s": 0.0, "r": 0.01 } ] },
                { "type": "convolve", "decay": 0.25, "predelay": 0.01, "damp": 0.5, "mix": 0.45 } ] } }"#,
        mac: (0x3877da20916325a4, None),
        linux: Some((0xaff8051356e767b3, None)),
    },
    Case {
        name: "granular-cloud",
        json: r#"{ "name": "granular-cloud", "duration": 0.4, "seed": 8, "version": 2, "engine": 4,
            "root": { "type": "chain", "stages": [
                { "type": "sine", "freq": 220 },
                { "type": "granular", "grain_ms": 40, "density": 25, "pitch": 2.0,
                    "spread": 0.3, "mix": 0.7 } ] } }"#,
        mac: (0x85ac18c492b92392, None),
        linux: Some((0xb91cd059c9dda11f, None)),
    },
    Case {
        // Kick track ducks the sustained bass via a tracks-level sidechain.
        name: "tracks-sidechain",
        json: r#"{ "name": "tracks-sidechain", "duration": 1.0, "seed": 6, "version": 2, "engine": 4,
            "root": { "type": "tracks", "tracks": [
                { "id": "kick", "node": { "type": "seq", "bpm": 240, "wave": "kit", "kit": "808",
                    "env": { "a": 0.001, "d": 0.1, "s": 0.5, "r": 0.1 },
                    "notes": [
                        { "step": 0, "len": 1, "pitch": "midi:36" },
                        { "step": 4, "len": 1, "pitch": "midi:36" },
                        { "step": 8, "len": 1, "pitch": "midi:36" },
                        { "step": 12, "len": 1, "pitch": "midi:36" } ] } },
                { "id": "bass", "node": { "type": "seq", "bpm": 240, "wave": "sawtooth",
                    "env": { "a": 0.005, "d": 0.05, "s": 0.8, "r": 0.05 },
                    "notes": [ { "step": 0, "len": 16, "pitch": "C2" } ] },
                  "sidechain": { "source": "kick", "amount": 0.8, "attack": 0.005, "release": 0.15 } }
            ] } }"#,
        mac: (
            0xf152984e8fa7017a,
            Some((0xf152984e8fa7017a, 0xf152984e8fa7017a)),
        ),
        linux: Some((
            0xce41f23ba0ba3ca2,
            Some((0xce41f23ba0ba3ca2, 0xce41f23ba0ba3ca2)),
        )),
    },
    Case {
        name: "seq-brass",
        json: r#"{ "name": "seq-brass", "duration": 0.8, "version": 2, "engine": 4,
            "root": { "type": "seq", "bpm": 240, "wave": "brass",
                "env": { "a": 0.01, "d": 0.05, "s": 1.0, "r": 0.1 },
                "notes": [
                    { "step": 0, "len": 4, "pitch": "C3" },
                    { "step": 4, "len": 4, "pitch": "E3" },
                    { "step": 8, "len": 6, "pitch": "G3", "gain": 0.85 } ] } }"#,
        mac: (0xe46f21522d9b95a5, None),
        linux: Some((0x9e4ee6a530469f9b, None)),
    },
    Case {
        name: "seq-flute",
        json: r#"{ "name": "seq-flute", "duration": 0.8, "version": 2, "engine": 4,
            "root": { "type": "seq", "bpm": 240, "wave": "flute",
                "env": { "a": 0.02, "d": 0.05, "s": 1.0, "r": 0.15 },
                "notes": [
                    { "step": 0, "len": 8, "pitch": "A4" },
                    { "step": 8, "len": 6, "pitch": "C5", "gain": 0.9 } ] } }"#,
        mac: (0x99b9315441f1a433, None),
        linux: Some((0x59cfce8c3f6f9411, None)),
    },
    Case {
        name: "seq-mallet",
        json: r#"{ "name": "seq-mallet", "duration": 0.8, "version": 2, "engine": 4,
            "root": { "type": "seq", "bpm": 240, "wave": "mallet",
                "env": { "a": 0.001, "d": 0.2, "s": 0.3, "r": 0.1 },
                "notes": [
                    { "step": 0, "len": 2, "pitch": "C5" },
                    { "step": 4, "len": 2, "pitch": "E5" },
                    { "step": 8, "len": 2, "pitch": "G5" },
                    { "step": 12, "len": 4, "pitch": "C6", "gain": 0.85 } ] } }"#,
        mac: (0x289a1a60103010a2, None),
        linux: Some((0x53ac9091cdbb3478, None)),
    },
    Case {
        name: "seq-bell",
        json: r#"{ "name": "seq-bell", "duration": 1.0, "version": 2, "engine": 4,
            "root": { "type": "seq", "bpm": 240, "wave": "bell",
                "env": { "a": 0.001, "d": 0.1, "s": 1.0, "r": 0.3 },
                "notes": [
                    { "step": 0, "len": 8, "pitch": "C4" },
                    { "step": 8, "len": 8, "pitch": "G4", "gain": 0.8 } ] } }"#,
        mac: (0xcc7cf5e7ed38019f, None),
        linux: Some((0x405eda555be5a8d1, None)),
    },
    Case {
        // A tempo change mid-sequence: segment-wise exact placement (ADR 0002).
        name: "seq-tempo-map",
        json: r#"{ "name": "seq-tempo-map", "duration": 2.5, "version": 2, "engine": 4,
            "root": { "type": "seq", "bpm": 120, "wave": "square",
                "tempo_map": [ { "at": { "num": 0, "den": 1 }, "bpm": 120 },
                               { "at": { "num": 4, "den": 1 }, "bpm": 90 } ],
                "env": { "a": 0.005, "d": 0.05, "s": 0.6, "r": 0.08 },
                "notes": [
                    { "step": 0, "len": 2, "pitch": "C4" },
                    { "step": 8, "len": 2, "pitch": "E4" },
                    { "step": 16, "len": 2, "pitch": "G4" },
                    { "step": 24, "len": 4, "pitch": "C5", "gain": 0.8 } ] } }"#,
        mac: (0x93bbbd9fe2057929, None),
        linux: None,
    },
    Case {
        // Step and exp automation curves on one track.
        name: "tracks-automation-curves",
        json: r#"{ "name": "tracks-automation-curves", "duration": 1.0, "seed": 9, "version": 2, "engine": 4,
            "root": { "type": "tracks", "tracks": [
                { "id": "pad", "node": { "type": "sawtooth", "freq": 220 }, "gain": 0.5,
                  "automation": [
                    { "target": "gain", "curve": "exp", "points": [
                        { "t": 0.0, "v": 0.2 }, { "t": 0.6, "v": 0.9 } ] },
                    { "target": "pan", "curve": "step", "points": [
                        { "t": 0.0, "v": -0.5 }, { "t": 0.5, "v": 0.5 } ] }
                  ] }
            ] } }"#,
        mac: (
            0xecd8fea88e3d1125,
            Some((0xc9d8d2534019a40a, 0xdc1a60d87163b2be)),
        ),
        linux: None,
    },
    Case {
        // Bus routing: a pad routed to a reverb bus, a kick sent to it.
        name: "tracks-bus-mix",
        json: r#"{ "name": "tracks-bus-mix", "duration": 1.0, "seed": 6, "version": 2, "engine": 4,
            "root": { "type": "tracks",
                "buses": [ { "id": "verb", "gain": 0.8, "effects": [
                    { "type": "reverb", "room": 0.5, "mix": 0.6 } ] } ],
                "tracks": [
                    { "id": "kick", "node": { "type": "seq", "bpm": 240, "wave": "kit", "kit": "808",
                        "env": { "a": 0.001, "d": 0.1, "s": 0.5, "r": 0.1 },
                        "notes": [ { "step": 0, "len": 1, "pitch": "midi:36" } ] },
                      "sends": [ { "bus": "verb", "amount": 0.3 } ] },
                    { "id": "pad", "node": { "type": "sawtooth", "freq": 110 }, "gain": 0.4,
                      "pan": 0.3, "bus": "verb" }
                ] } }"#,
        mac: (
            0x9be3ed8b32bcf69f,
            Some((0x90a3cdf1747ca6c6, 0x577c2b180bd35620)),
        ),
        linux: None,
    },
];

/// Song fluent path golden: the arrangement layer compiles through `to_doc`
/// into the same render path, pinned end to end.
fn fluent_song_doc() -> SoundDoc {
    let amp = Adsr {
        a: 0.005,
        d: 0.1,
        s: 0.8,
        r: 0.2,
        punch: 0.0,
    };
    let mut song = Song::new("golden-groove", 240.0);
    song.add_track("bass", SeqWave::Bass, amp);
    song.add_track("keys", SeqWave::Epiano, amp);
    song.add_pattern(
        "riff",
        1,
        vec![note(0, 4, "C2"), note(8, 4, "G2"), note(12, 4, "A#2")],
    );
    song.add_pattern("stab", 1, vec![note(4, 2, "C4"), note(6, 2, "D#4")]);
    song.arrange("bass", "riff", 0);
    song.arrange("keys", "stab", 0);
    song.to_doc().expect("song compiles")
}

const FLUENT_SONG: Case = Case {
    name: "fluent-song",
    json: "", // built through the Song API, not JSON
    mac: (
        0x2184e2154f5b92e6,
        Some((0x2184e2154f5b92e6, 0x2184e2154f5b92e6)),
    ),
    linux: Some((
        0xa940563791f3bcf2,
        Some((0xa940563791f3bcf2, 0xa940563791f3bcf2)),
    )),
};

fn check(c: &Case, doc: &SoundDoc) -> Option<String> {
    let p = render_product(doc);
    let got: Pins = (
        hash_signal(&p.mono),
        p.stereo
            .as_ref()
            .map(|(l, r)| (hash_signal(l), hash_signal(r))),
    );
    match expected(c) {
        Some(pins) if pins == got => None,
        // Unpinned platform: only assert in-process determinism.
        None => {
            let q = render_product(doc);
            (hash_signal(&q.mono) != got.0).then(|| format!("{}: non-deterministic", c.name))
        }
        Some(_) => Some(match got.1 {
            Some((l, r)) => format!(
                "{}: (0x{:016x}, Some((0x{l:016x}, 0x{r:016x})))",
                c.name, got.0
            ),
            None => format!("{}: (0x{:016x}, None)", c.name, got.0),
        }),
    }
}

#[test]
fn golden_corpus_replays_byte_identically() {
    let mut mismatches = Vec::new();
    for c in CORPUS {
        if let Some(m) = check(c, &parse(c.json)) {
            mismatches.push(m);
        }
    }
    if let Some(m) = check(&FLUENT_SONG, &fluent_song_doc()) {
        mismatches.push(m);
    }
    assert!(
        mismatches.is_empty(),
        "golden renders changed — if intentional (an engine-gated upgrade), re-pin \
         these (mono, stereo) values for this platform:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn serde_roundtrip_preserves_the_render() {
    for c in CORPUS {
        let doc = parse(c.json);
        let reparsed: SoundDoc =
            serde_json::from_str(&serde_json::to_string(&doc).expect("serialize"))
                .expect("reparse");
        let a = render_product(&doc);
        let b = render_product(&reparsed);
        let bits = |s: &[f32]| s.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(
            bits(&a.mono),
            bits(&b.mono),
            "{}: serialize→reparse changed the render",
            c.name
        );
    }
}

#[test]
fn example_recipes_replay_byte_identically() {
    // CLAUDE.md promises the docs/examples recipes replay in CI — enforce it.
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/examples/");
    for (file, mac, linux) in [
        ("blip.json", 0x412f073451d5a804u64, 0x736497864af55267u64),
        ("hat.json", 0xbc3ab825679ceef7u64, 0xbc3ab825679ceef7u64),
    ] {
        let json = std::fs::read_to_string(format!("{root}{file}"))
            .unwrap_or_else(|e| panic!("read {file}: {e}"));
        let doc = parse(&json);
        let got = hash_signal(&render_product(&doc).mono);
        let expected = platform_pin(mac, Some(linux));
        if let Some(expected) = expected {
            assert_eq!(
                got, expected,
                "{file}: expected 0x{expected:016x}, got 0x{got:016x}"
            );
        }
    }
}
