//! One-pass validation for song compilation: the tempo/meter maps, pickup,
//! grid placement, and sections/markers checks (T1003–T1006). Every problem
//! is collected, never first-error — see [`crate::diag`].

use super::Song;
use crate::diag::{CompileError, Diagnostic};
use crate::units::Beat;

impl Song {
    /// Validate the tempo/meter maps, pickup, grid placement, and
    /// sections/markers — one pass, every problem collected (T1003–T1006).
    pub(crate) fn validate_maps(&self, diags: &mut CompileError) {
        // The tempo map (T1003): first at beat 0, strictly ascending, sane
        // tempos, bounded (mirrors the document-level seq validation).
        let map = &self.tempo_map;
        if map.len() > 1024 {
            diags.push(
                Diagnostic::error(
                    "T1003",
                    "tempo_map",
                    format!("tempo_map is capped at 1024 points, got {}", map.len()),
                )
                .with_remediation("split the piece or thin the tempo changes"),
            );
        }
        if let Some(first) = map.first()
            && first.at != Beat::zero()
        {
            diags.push(
                Diagnostic::error("T1003", "tempo_map[0].at", "tempo_map's first point must be at beat 0")
                    .with_remediation("add a point at beat 0 (the song's bpm applies before the first change otherwise)"),
            );
        }
        for (i, p) in map.iter().enumerate() {
            if !(p.bpm.is_finite() && p.bpm > 0.0) {
                diags.push(
                    Diagnostic::error(
                        "T1003",
                        format!("tempo_map[{i}].bpm"),
                        format!("tempo must be positive and finite, got {}", p.bpm),
                    )
                    .with_remediation("set a tempo above 0 BPM"),
                );
            }
            if i > 0 && p.at <= map[i - 1].at {
                diags.push(
                    Diagnostic::error(
                        "T1003",
                        format!("tempo_map[{i}].at"),
                        "tempo_map must be strictly ascending by beat",
                    )
                    .with_remediation("sort the changes and merge any duplicates"),
                );
            }
        }
        // The meter map (T1004): first at bar 0, ascending bars, numerator ≥ 1,
        // power-of-two denominator, bounded.
        let meter = &self.meter_map;
        if meter.len() > 256 {
            diags.push(
                Diagnostic::error(
                    "T1004",
                    "meter_map",
                    format!("meter_map is capped at 256 points, got {}", meter.len()),
                )
                .with_remediation("consolidate repeated time-signature changes"),
            );
        }
        if let Some(first) = meter.first()
            && first.bar != 0
        {
            diags.push(
                Diagnostic::error(
                    "T1004",
                    "meter_map[0].bar",
                    "meter_map's first point must be at bar 0",
                )
                .with_remediation("add the opening time signature at bar 0"),
            );
        }
        for (i, p) in meter.iter().enumerate() {
            if p.numerator < 1 {
                diags.push(
                    Diagnostic::error(
                        "T1004",
                        format!("meter_map[{i}].numerator"),
                        "time-signature numerator must be ≥ 1",
                    )
                    .with_remediation("use a numerator like 3 (3/4) or 6 (6/8)"),
                );
            }
            if !p.denominator.is_power_of_two() || p.denominator > 64 {
                diags.push(
                    Diagnostic::error(
                        "T1004",
                        format!("meter_map[{i}].denominator"),
                        format!(
                            "time-signature denominator must be a power of two ≤ 64, got {}",
                            p.denominator
                        ),
                    )
                    .with_remediation("use 2, 4, 8, 16, 32, or 64"),
                );
            }
            if i > 0 && p.bar <= meter[i - 1].bar {
                diags.push(
                    Diagnostic::error(
                        "T1004",
                        format!("meter_map[{i}].bar"),
                        "meter_map must be strictly ascending by bar",
                    )
                    .with_remediation("sort the changes and merge any duplicates"),
                );
            }
        }
        if let Some(pickup) = self.pickup
            && pickup < Beat::zero()
        {
            diags.push(
                Diagnostic::error("T1004", "pickup", "the pickup bar can't be negative")
                    .with_remediation("use zero (no pickup) or a positive beat length"),
            );
        }
        // Grid placement (T1005): every placement must land on a step.
        if !self.plain_meter() {
            for (i, pl) in self.arrangement.iter().enumerate() {
                if self.beat_to_step(self.beat_at_bar(pl.bar)).is_err() {
                    diags.push(
                        Diagnostic::error("T1005", format!("arrangement[{i}].bar"), format!("bar {} lands between grid steps", pl.bar))
                            .with_remediation("raise steps_per_beat, or move the placement/meter change onto the grid"),
                    );
                }
            }
        }
        // Sections and markers (T1006).
        for (i, s) in self.sections.iter().enumerate() {
            if s.name.is_empty() {
                diags.push(
                    Diagnostic::error(
                        "T1006",
                        format!("sections[{i}].name"),
                        "a section needs a name",
                    )
                    .with_remediation("name it (e.g. \"verse\", \"chorus\")"),
                );
            }
            if s.bars < 1 {
                diags.push(
                    Diagnostic::error(
                        "T1006",
                        format!("sections[{i}].bars"),
                        "a section must be at least one bar",
                    )
                    .with_remediation("set bars ≥ 1"),
                );
            }
        }
        for (i, m) in self.markers.iter().enumerate() {
            if m.name.is_empty() {
                diags.push(
                    Diagnostic::error(
                        "T1006",
                        format!("markers[{i}].name"),
                        "a marker needs a name",
                    )
                    .with_remediation("name it (e.g. \"drop\", \"cue\")"),
                );
            }
        }
    }
}
