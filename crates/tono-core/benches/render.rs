//! Criterion benchmarks for the render hot path.
//!
//! The golden corpus pins every render byte-for-byte; these benches pin its
//! *speed* envelope, so a perf regression hiding inside a correct refactor
//! shows up in the report. One bench per representative kernel family —
//! oscillator+env, the stereo mixer with automation and a master reverb, a
//! melodic (piano) seq, a heavy effects chain, and block-by-block streaming.
//!
//! Documents are short (0.3–0.5 s) so the whole suite runs in a couple of
//! minutes, and each doc is built ONCE outside the measured closure. Run with
//! `make bench` (report-only — not part of `make verify`).

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tono_core::dsl::SoundDoc;
use tono_core::render;
use tono_core::streaming::StreamGraph;

/// Parse + validate a bench document once (setup, never measured).
fn doc(json: &str) -> SoundDoc {
    let doc: SoundDoc = serde_json::from_str(json).expect("bench doc parses");
    doc.validate().expect("bench doc validates");
    doc
}

/// (a) The minimal SFX kernel: one oscillator under an ADSR.
fn blip() -> SoundDoc {
    doc(r#"{ "name": "blip", "duration": 0.3, "engine": 4,
            "root": { "type": "mul", "inputs": [
                { "type": "sine", "freq": 880 },
                { "type": "env", "a": 0.002, "d": 0.08, "s": 0.0, "r": 0.05 } ] } }"#)
}

/// (b) The stereo mixer: pan law, an automation lane, and a master reverb
/// with decorrelated tails (modeled on the golden `tracks-mix` case).
fn tracks_mix() -> SoundDoc {
    doc(
        r#"{ "name": "tracks-mix", "duration": 0.5, "seed": 9, "engine": 4,
            "normalize": { "target_lufs": -14, "ceiling_dbtp": -1.0 },
            "root": { "type": "tracks", "tracks": [
                { "id": "pad", "node": { "type": "sine", "freq": 220 }, "pan": -0.8, "gain": 0.3 },
                { "id": "hiss", "node": { "type": "noise", "color": "white" }, "pan": 0.9, "gain": 0.6,
                  "automation": [ { "target": "gain", "points": [
                      { "t": 0.0, "v": 0.1 }, { "t": 0.4, "v": 0.8 } ] } ] },
                { "id": "lead", "node": { "type": "square", "freq": 440, "duty": 0.25 }, "gain": 0.4, "at": 0.1 }
            ], "master": [ { "type": "reverb", "room": 0.3, "mix": 0.2 } ] } }"#,
    )
}

/// (c) A melodic seq on the additive piano voice (the costliest seq voice).
fn seq_piano() -> SoundDoc {
    doc(r#"{ "name": "seq-piano", "duration": 0.5, "engine": 4,
            "root": { "type": "seq", "bpm": 240, "wave": "piano",
            "env": { "a": 0.002, "s": 1.0, "r": 0.2 },
            "notes": [
                { "step": 0, "len": 4, "pitch": "C4" },
                { "step": 0, "len": 4, "pitch": "E4", "gain": 0.9 },
                { "step": 4, "len": 4, "pitch": "G3", "gain": 0.7 } ] } }"#)
}

/// (d) A heavy effects chain: unison source → filter → delay → reverb →
/// compressor (delay lines, comb/allpass tails, and envelope followers).
fn fx_chain() -> SoundDoc {
    doc(r#"{ "name": "fx-chain", "duration": 0.4, "engine": 4,
            "root": { "type": "chain", "stages": [
                { "type": "super", "freq": 110, "voices": 7, "detune_cents": 25 },
                { "type": "lowpass", "cutoff": 1200, "q": 0.9 },
                { "type": "delay", "secs": 0.09, "feedback": 0.35 },
                { "type": "reverb", "room": 0.4, "mix": 0.25 },
                { "type": "compress", "threshold": -18, "ratio": 4, "makeup": 6 } ] } }"#)
}

/// (e) A streamable graph: deterministic nodes only, constant filter
/// cutoffs, no normalize/loop/stereo — so `StreamGraph` covers it natively.
fn streamable() -> SoundDoc {
    doc(r#"{ "name": "streamable", "duration": 0.5, "engine": 4,
            "root": { "type": "chain", "stages": [
                { "type": "square", "freq": 220, "duty": 0.3 },
                { "type": "lowpass", "cutoff": 1800, "q": 0.8 },
                { "type": "drive", "amount": 1.5 },
                { "type": "tremolo", "rate": 6, "depth": 0.4 },
                { "type": "delay", "secs": 0.12, "feedback": 0.3 } ] } }"#)
}

fn bench_blip(c: &mut Criterion) {
    let doc = blip();
    c.bench_function("render/blip_osc_env", |b| {
        b.iter(|| render::render(black_box(&doc)))
    });
}

fn bench_tracks_mix(c: &mut Criterion) {
    let doc = tracks_mix();
    c.bench_function("render/tracks_mix_automation_master_reverb", |b| {
        b.iter(|| render::render_product(black_box(&doc)))
    });
}

fn bench_seq_piano(c: &mut Criterion) {
    let doc = seq_piano();
    c.bench_function("render/seq_piano", |b| {
        b.iter(|| render::render(black_box(&doc)))
    });
}

fn bench_fx_chain(c: &mut Criterion) {
    let doc = fx_chain();
    c.bench_function("render/fx_chain_reverb_delay_compress", |b| {
        b.iter(|| render::render(black_box(&doc)))
    });
}

fn bench_stream_fill(c: &mut Criterion) {
    const BLOCK: usize = 512;
    let doc = streamable();
    let n = (doc.duration * doc.sample_rate as f32).ceil() as usize;
    let blocks = n.div_ceil(BLOCK);
    c.bench_function("streaming/streamgraph_fill_512", |b| {
        b.iter(|| {
            // Rebuild per iteration so every measured run starts at position 0;
            // the doc itself is built once, outside the closure.
            let mut graph = StreamGraph::try_from_doc(black_box(&doc)).expect("doc is streamable");
            let mut block = [0.0f32; BLOCK];
            for _ in 0..blocks {
                graph.fill(black_box(&mut block));
            }
            black_box(block)
        })
    });
}

criterion_group!(
    benches,
    bench_blip,
    bench_tracks_mix,
    bench_seq_piano,
    bench_fx_chain,
    bench_stream_fill
);
criterion_main!(benches);
