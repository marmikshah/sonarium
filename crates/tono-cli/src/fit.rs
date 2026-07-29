//! `tono fit` — target-driven sound design, automated.
//!
//! `tono match` scores a doc against a reference WAV; `tono fit` closes the
//! loop. It hill-climbs the match score with `vary::mutate` — every candidate
//! is a seeded perturbation of the incumbent, kept when it scores closer — so
//! the whole search is a deterministic pure function of
//! `(reference, doc, rounds, amount, seed)`, reproducible and cacheable like
//! every other render.

use std::path::Path;

use anyhow::Result;
use tono_core::analysis::{self, Analysis};
use tono_core::dsl::SoundDoc;
use tono_core::{render, vary};

use crate::target;

/// The outcome of a fit run.
pub struct FitResult {
    /// The best-scoring document found (named `<name>_fit`).
    pub doc: SoundDoc,
    /// The starting doc's match score.
    pub initial: f32,
    /// The best doc's match score.
    pub score: f32,
    /// How many candidates were scored.
    pub rounds: u32,
    /// How many candidates improved on the incumbent.
    pub improvements: u32,
}

/// Render and score one candidate against the reference stats.
fn score_doc(reference: &Analysis, doc: &SoundDoc) -> f32 {
    let rendered = render::render(doc);
    let stats = analysis::stats(&rendered, doc.sample_rate);
    target::score(reference, &stats)
}

/// Hill-climb from `start` toward the reference WAV. Each round perturbs the
/// incumbent with `vary::mutate` under a fresh derived seed and adopts it when
/// the [`target::score`] improves; after four stalled rounds the step size
/// halves, so the search coarse-tunes first and fine-tunes late.
pub fn fit(
    reference: &Path,
    start: &SoundDoc,
    rounds: u32,
    amount: f32,
    seed: u64,
) -> Result<FitResult> {
    let (mono, sr) = target::read_wav_mono(reference)?;
    let ref_stats = analysis::stats(&mono, sr);

    let mut best = start.clone();
    let mut best_score = score_doc(&ref_stats, &best);
    let initial = best_score;
    let mut step = amount.clamp(0.01, 1.0);
    let mut stall = 0u32;
    let mut improvements = 0u32;

    for round in 0..rounds {
        let candidate = vary::mutate(&best, step, seed.wrapping_add(round as u64).wrapping_add(1));
        // mutate() promises a still-valid doc; a candidate that slipped a
        // bound is skipped, never rendered.
        if candidate.validate().is_err() {
            continue;
        }
        let score = score_doc(&ref_stats, &candidate);
        if score < best_score {
            best = candidate;
            best_score = score;
            improvements += 1;
            stall = 0;
        } else {
            stall += 1;
            if stall >= 4 {
                step = (step * 0.5).max(0.01);
                stall = 0;
            }
        }
    }

    best.name = if start.name.is_empty() {
        "fit".to_string()
    } else {
        format!("{}_fit", start.name)
    };
    Ok(FitResult {
        doc: best,
        initial,
        score: best_score,
        rounds,
        improvements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_doc(freq: u32) -> SoundDoc {
        serde_json::from_str(&format!(
            r#"{{ "name":"t", "duration":0.3, "root": {{ "type":"sine", "freq":{freq} }} }}"#
        ))
        .unwrap()
    }

    fn wav_of(doc: &SoundDoc, name: &str) -> std::path::PathBuf {
        let (l, r) = tono_core::player::render_stereo(doc);
        let dir = std::env::temp_dir().join("tono-fit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        crate::audio::write_wav_stereo(&path, &l, &r, doc.sample_rate, 16).unwrap();
        path
    }

    #[test]
    fn fit_pulls_a_mistuned_oscillator_toward_the_reference() {
        // Two octaves too bright: the search has one knob to find.
        let reference = wav_of(&sine_doc(220), "ref.wav");
        let result = fit(&reference, &sine_doc(880), 64, 0.5, 1).unwrap();
        assert!(
            result.score < result.initial * 0.5,
            "score {:.2} → {:.2}",
            result.initial,
            result.score
        );
        assert!(result.improvements > 0);
        assert_eq!(result.doc.validate(), Ok(()));
        assert_eq!(result.doc.name, "t_fit");
    }

    #[test]
    fn fit_is_deterministic_per_seed() {
        let reference = wav_of(&sine_doc(330), "det.wav");
        let a = fit(&reference, &sine_doc(700), 32, 0.4, 9).unwrap();
        let b = fit(&reference, &sine_doc(700), 32, 0.4, 9).unwrap();
        assert_eq!(a.score, b.score);
        assert_eq!(
            serde_json::to_value(&a.doc).unwrap(),
            serde_json::to_value(&b.doc).unwrap()
        );
    }

    #[test]
    fn fit_never_worsens_the_starting_doc() {
        // Already a close match: the incumbent's score is the floor, and the
        // run stays in close-match territory (small improvements from 16-bit
        // quantization noise in the reference are fine — that's the search
        // working, not a regression).
        let start = sine_doc(440);
        let reference = wav_of(&start, "self.wav");
        let result = fit(&reference, &start, 16, 0.3, 4).unwrap();
        assert!(result.score <= result.initial);
        assert!(result.score < 1.0, "stays a close match: {}", result.score);
    }
}
