//! `tono presets` / `tono catalog` — the factory sounds, from the shell.
//!
//! Discovery is docs-only without these: the 16 factory presets and the 31
//! catalog voices lived behind `cargo add tono-core` and a Rust file. With no
//! argument each subcommand lists; with a NAME it renders a short demo —
//! a preset plays a C-major arpeggio through the live `Instrument` engine
//! bounced offline, a catalog voice plays a scale (a two-bar groove for the
//! drum kits) as a tiny `Song` compiled to an ordinary `SoundDoc`.

use anyhow::{Error, Result};
use tono_core::catalog::{
    Bass, Bells, Brass, Drums, ElectricPiano, Flute, GrandPiano, Guitar, Mallets, Organ, Strings,
    Voice,
};
use tono_core::dsl::{SeqWave, SoundDoc};
use tono_core::dsp;
use tono_core::instrument::{Instrument, Note};
use tono_core::presets::{self, Preset};
use tono_core::runtime::AudioSource;
use tono_core::song::Song;

/// The demo riff every preset audition plays (C major arpeggio up and back) —
/// the same figure the `presets` playground example performs live.
const RIFF: [Note; 6] = [Note::C4, Note(64), Note(67), Note(72), Note(67), Note(64)];

/// The printable preset list: name, category, blurb — one line per factory
/// preset, in catalog order.
pub fn preset_list() -> String {
    let mut out = String::new();
    for p in presets::PRESETS {
        out.push_str(&format!(
            "{:<14}{:<8}{}\n",
            p.name,
            format!("{:?}", p.category).to_lowercase(),
            p.description
        ));
    }
    out
}

/// Look up a factory preset by its slug.
pub fn find_preset(name: &str) -> Option<&'static Preset> {
    presets::PRESETS.iter().find(|p| p.name == name)
}

/// Bounce a preset's demo riff through the live `Instrument` engine, offline:
/// sample-accurate note events, stereo out, the renderer's transparent peak
/// safety applied so the file on disk can't clip.
pub fn bounce_preset(preset: &Preset, sample_rate: u32) -> Result<(Vec<f32>, Vec<f32>)> {
    let mut inst = Instrument::new(preset.design(), sample_rate).map_err(Error::msg)?;
    let on = (0.180 * sample_rate as f32) as usize;
    let gap = (0.040 * sample_rate as f32) as usize;
    let tail = (0.700 * sample_rate as f32) as usize;
    let total = RIFF.len() * (on + gap) + tail;

    let mut left = Vec::with_capacity(total);
    let mut right = Vec::with_capacity(total);
    let mut frame = 0usize;
    for (i, note) in RIFF.iter().enumerate() {
        let at = i * (on + gap);
        fill(&mut inst, at - frame, &mut left, &mut right);
        frame = at;
        inst.note_on(*note, 0.9);
        fill(&mut inst, on, &mut left, &mut right);
        frame += on;
        inst.note_off(*note);
    }
    fill(&mut inst, total - frame, &mut left, &mut right);
    dsp::peak_limit(&mut [&mut left, &mut right]);
    Ok((left, right))
}

/// Pull `frames` frames of interleaved stereo out of the source.
fn fill(source: &mut impl AudioSource, frames: usize, left: &mut Vec<f32>, right: &mut Vec<f32>) {
    let mut block = [0.0f32; 4096];
    let mut done = 0;
    while done < frames {
        let take = (frames - done).min(block.len() / 2);
        let n = source.fill(&mut block[..take * 2]);
        for f in 0..n {
            left.push(block[f * 2]);
            right.push(block[f * 2 + 1]);
        }
        done += n;
        if n == 0 {
            break; // an exhausted source pads with silence, never spins
        }
    }
}

/// One catalog voice: the family it lists under, its CLI slug, its builder.
pub struct CatalogEntry {
    /// Family grouping for the listing (`piano`, `bass`, …).
    pub family: &'static str,
    /// The CLI name (`grand`, `tr808`, …).
    pub slug: &'static str,
    /// The catalog constructor.
    pub make: fn() -> Voice,
}

/// Every catalog voice, grouped by family. Keep in sync with `catalog.rs`
/// (the count test below pins it).
pub static CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        family: "piano",
        slug: "grand",
        make: GrandPiano::grand,
    },
    CatalogEntry {
        family: "piano",
        slug: "bright",
        make: GrandPiano::bright,
    },
    CatalogEntry {
        family: "piano",
        slug: "mellow",
        make: GrandPiano::mellow,
    },
    CatalogEntry {
        family: "piano",
        slug: "felt",
        make: GrandPiano::felt,
    },
    CatalogEntry {
        family: "piano",
        slug: "upright",
        make: GrandPiano::upright,
    },
    CatalogEntry {
        family: "piano",
        slug: "honky-tonk",
        make: GrandPiano::honky_tonk,
    },
    CatalogEntry {
        family: "epiano",
        slug: "rhodes",
        make: ElectricPiano::rhodes,
    },
    CatalogEntry {
        family: "epiano",
        slug: "wurli",
        make: ElectricPiano::wurli,
    },
    CatalogEntry {
        family: "epiano",
        slug: "dx",
        make: ElectricPiano::dx,
    },
    CatalogEntry {
        family: "organ",
        slug: "tonewheel",
        make: Organ::tonewheel,
    },
    CatalogEntry {
        family: "organ",
        slug: "rock",
        make: Organ::rock,
    },
    CatalogEntry {
        family: "strings",
        slug: "ensemble",
        make: Strings::ensemble,
    },
    CatalogEntry {
        family: "strings",
        slug: "warm",
        make: Strings::warm,
    },
    CatalogEntry {
        family: "brass",
        slug: "section",
        make: Brass::section,
    },
    CatalogEntry {
        family: "brass",
        slug: "stab",
        make: Brass::stab,
    },
    CatalogEntry {
        family: "flute",
        slug: "concert",
        make: Flute::concert,
    },
    CatalogEntry {
        family: "bass",
        slug: "finger",
        make: Bass::finger,
    },
    CatalogEntry {
        family: "bass",
        slug: "pick",
        make: Bass::pick,
    },
    CatalogEntry {
        family: "bass",
        slug: "sub",
        make: Bass::sub,
    },
    CatalogEntry {
        family: "bass",
        slug: "synth",
        make: Bass::synth,
    },
    CatalogEntry {
        family: "guitar",
        slug: "nylon",
        make: Guitar::nylon,
    },
    CatalogEntry {
        family: "guitar",
        slug: "steel",
        make: Guitar::steel,
    },
    CatalogEntry {
        family: "guitar",
        slug: "electric",
        make: Guitar::electric,
    },
    CatalogEntry {
        family: "mallets",
        slug: "marimba",
        make: Mallets::marimba,
    },
    CatalogEntry {
        family: "mallets",
        slug: "vibraphone",
        make: Mallets::vibraphone,
    },
    CatalogEntry {
        family: "mallets",
        slug: "glockenspiel",
        make: Mallets::glockenspiel,
    },
    CatalogEntry {
        family: "bells",
        slug: "tubular",
        make: Bells::tubular,
    },
    CatalogEntry {
        family: "drums",
        slug: "acoustic",
        make: Drums::acoustic,
    },
    CatalogEntry {
        family: "drums",
        slug: "classic",
        make: Drums::classic,
    },
    CatalogEntry {
        family: "drums",
        slug: "electronic",
        make: Drums::electronic,
    },
    CatalogEntry {
        family: "drums",
        slug: "tr808",
        make: Drums::tr808,
    },
];

/// The printable catalog: voices grouped by family.
pub fn catalog_list() -> String {
    let mut out = String::new();
    let mut families: Vec<&str> = CATALOG.iter().map(|e| e.family).collect();
    families.dedup();
    for family in families {
        let slugs: Vec<&str> = CATALOG
            .iter()
            .filter(|e| e.family == family)
            .map(|e| e.slug)
            .collect();
        out.push_str(&format!("{family:<9}{}\n", slugs.join(", ")));
    }
    out
}

/// Look up a catalog voice by its CLI slug.
pub fn find_voice(slug: &str) -> Option<Voice> {
    CATALOG.iter().find(|e| e.slug == slug).map(|e| (e.make)())
}

/// A voice's demo document: an ascending C-major scale resolving to a chord
/// (a two-bar groove for the drum kits), compiled to an ordinary `SoundDoc`
/// so the usual render pipeline — images, stats, every format — applies.
pub fn voice_demo_doc(voice: &Voice) -> Result<SoundDoc> {
    let kit = voice.wave == SeqWave::Kit;
    let song = Song::new(voice.name.clone(), 120.0).add(voice.clone(), |t| {
        if kit {
            for _ in 0..2 {
                t.kick().hat().rest(1.0).hat().rest(1.0);
                t.snare().hat().rest(1.0).hat().rest(1.0);
            }
        } else {
            for n in ["C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5"] {
                t.play(n, 0.5);
            }
            t.chord(&["C4", "E4", "G4", "C5"], 2.0);
        }
    });
    song.to_doc().map_err(Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tono_core::render;

    #[test]
    fn preset_list_covers_every_factory_preset() {
        let list = preset_list();
        assert_eq!(list.lines().count(), presets::PRESETS.len());
        for p in presets::PRESETS {
            assert!(list.contains(p.name), "lists {}", p.name);
            assert!(find_preset(p.name).is_some(), "lookup {}", p.name);
        }
        assert!(find_preset("nope").is_none());
    }

    #[test]
    fn catalog_slugs_are_unique_and_build() {
        assert_eq!(CATALOG.len(), 31, "keep CATALOG in sync with catalog.rs");
        for (i, e) in CATALOG.iter().enumerate() {
            assert!(
                CATALOG.iter().skip(i + 1).all(|o| o.slug != e.slug),
                "duplicate slug {}",
                e.slug
            );
            let voice = (e.make)();
            assert!(!voice.name.is_empty());
        }
        assert!(find_voice("grand").is_some());
        assert!(find_voice("nope").is_none());
    }

    #[test]
    fn catalog_list_groups_by_family() {
        let list = catalog_list();
        for family in [
            "piano", "epiano", "organ", "strings", "brass", "flute", "bass", "guitar", "mallets",
            "bells", "drums",
        ] {
            assert!(list.lines().any(|l| l.starts_with(family)), "{family}");
        }
        assert!(list.contains("honky-tonk") && list.contains("tr808"));
    }

    #[test]
    fn a_preset_bounce_is_full_length_and_sounds() {
        let sr = 48_000u32;
        let preset = find_preset("pluck").unwrap();
        let (left, right) = bounce_preset(preset, sr).unwrap();
        let expected = RIFF.len() * ((0.220 * sr as f32) as usize) + (0.700 * sr as f32) as usize;
        assert_eq!(left.len(), expected);
        assert_eq!(left.len(), right.len());
        assert!(left.iter().all(|x| x.is_finite() && x.abs() <= dsp::CEIL));
        let peak = left.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05, "the riff sounds: peak {peak}");
    }

    #[test]
    fn a_pitched_voice_demo_renders() {
        let doc = voice_demo_doc(&GrandPiano::grand()).unwrap();
        doc.validate().unwrap();
        let audio = render::render(&doc);
        let peak = audio.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05, "the scale sounds: peak {peak}");
    }

    #[test]
    fn a_drum_voice_demo_plays_the_groove() {
        let doc = voice_demo_doc(&Drums::tr808()).unwrap();
        doc.validate().unwrap();
        let audio = render::render(&doc);
        let stats = tono_core::analysis::stats(&audio, doc.sample_rate);
        // The groove is 12 hits over two bars; the onset detector reliably
        // catches the four prominent kick/snare downbeats (the 808's hats
        // sit under its threshold).
        assert!(stats.onset_count >= 4, "onsets: {}", stats.onset_count);
    }
}
