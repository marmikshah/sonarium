//! `tono review` — grade a rendered sound against an archetype and the
//! universal ship checklist, worst finding first.
//!
//! `tono render` shows what a sound IS; `tono review` says whether it is
//! ready to ship — every finding names the measured value, the target, and
//! the concrete fix to try next. The grading itself is `tono_core`'s
//! deterministic [`review`] function; this module renders, measures, and
//! formats it for the terminal.

use tono_core::analysis;
use tono_core::dsl::{Playback, SoundDoc};
use tono_core::render;

pub use tono_core::review::{Archetype, Finding, Review, Status, review};

/// Parse an `--archetype` flag value.
pub fn parse_archetype(s: &str) -> anyhow::Result<Archetype> {
    Ok(match s {
        "laser" => Archetype::Laser,
        "coin" => Archetype::Coin,
        "jump" => Archetype::Jump,
        "impact" => Archetype::Impact,
        "ui" => Archetype::Ui,
        "footstep" => Archetype::Footstep,
        "powerup" => Archetype::Powerup,
        "ambience" => Archetype::Ambience,
        "bgm" => Archetype::Bgm,
        other => anyhow::bail!(
            "--archetype must be laser, coin, jump, impact, ui, footstep, powerup, ambience, or bgm, got '{other}'"
        ),
    })
}

/// Render the doc, grade it against `archetype` (`None` runs only the
/// universal ship checklist), and return the review plus its printable
/// report. Level metrics measure the stereo pair when there is one — the
/// export — matching what `tono render` reports.
pub fn review_doc(doc: &SoundDoc, archetype: Option<Archetype>) -> (Review, String) {
    let product = render::render_product(doc);
    let stats = match &product.stereo {
        Some((left, right)) => analysis::stats_stereo(left, right, doc.sample_rate),
        None => analysis::stats(&product.mono, doc.sample_rate),
    };
    // A loop's seam is measured from the same render, never fabricated.
    let seam =
        matches!(doc.playback, Playback::Loop { .. }).then(|| render::loop_seam_db(&product.mono));
    let graded = review(doc, &stats, archetype, seam);
    let report = format_review(&graded);
    (graded, report)
}

/// The printable report: the one-line summary, then every finding
/// worst-first — status, what was measured, the target, and the fix to try.
fn format_review(r: &Review) -> String {
    let mut out = format!("{}\n", r.summary);
    for f in &r.findings {
        let status = format!("{:?}", f.status).to_uppercase();
        out.push_str(&format!(
            "{status:<5} {:<14} {:<16} {:<22} {}\n",
            f.criterion, f.value, f.target, f.fix
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(json: &str) -> SoundDoc {
        serde_json::from_str(json).expect("deserialize")
    }

    #[test]
    fn every_archetype_name_parses() {
        for name in [
            "laser", "coin", "jump", "impact", "ui", "footstep", "powerup", "ambience", "bgm",
        ] {
            assert!(parse_archetype(name).is_ok(), "parses: {name}");
        }
        let err = parse_archetype("pew-pew").err().unwrap();
        assert!(err.to_string().contains("pew-pew"), "names the offender");
    }

    #[test]
    fn a_clean_sound_passes_the_generic_checklist() {
        let d = doc(
            r#"{ "name": "blip", "duration": 0.3, "root": { "type": "mul", "inputs": [
                { "type": "sine", "freq": 880 },
                { "type": "env", "a": 0.002, "d": 0.08, "s": 0.0, "r": 0.05 } ] } }"#,
        );
        let (review, report) = review_doc(&d, None);
        assert_ne!(review.grade, Status::Fail, "report:\n{report}");
        assert!(report.contains("blip [generic]"), "summary line: {report}");
    }

    #[test]
    fn findings_print_worst_first_with_the_fix() {
        // A percussive blip judged as an ambience bed: crest is out of spec.
        let d = doc(
            r#"{ "name": "blip", "duration": 0.3, "root": { "type": "mul", "inputs": [
                { "type": "sine", "freq": 660 },
                { "type": "env", "a": 0.0, "d": 0.05, "s": 0.0, "r": 0.02, "punch": 0.6 } ] } }"#,
        );
        let (review, report) = review_doc(&d, Some(Archetype::Ambience));
        assert_ne!(review.grade, Status::Pass);
        let crest = report
            .lines()
            .find(|l| l.contains("crest"))
            .expect("a crest finding");
        assert!(
            crest.starts_with("WARN") || crest.starts_with("FAIL"),
            "{crest}"
        );
        // The first finding line is never a PASS when anything worse exists.
        let first = report.lines().nth(1).unwrap();
        assert!(!first.starts_with("PASS"), "worst first: {first}");
    }
}
