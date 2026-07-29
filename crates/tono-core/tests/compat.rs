//! Compatibility fixtures (issue #52, WS1/WS9): supported historical
//! documents, songs, patches, and bundles load and behave per the
//! compatibility matrix. A fixture that must keep loading FOREVER lives in
//! `tests/compat/`; these tests are the promise that it does.

use tono_core::patch::Patch;
use tono_core::program::Program;
use tono_core::song::{CompileOptions, Song};

fn read(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/compat/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// The alpha.1-era song shape (mute/solo + engine/version pins, before
/// tempo/meter maps, sections, buses, and automation): parses with defaults
/// for every field added since, compiles, and its program keeps the pinned
/// hash — the exact compatibility contract for song JSON.
#[test]
fn alpha1_song_loads_and_compiles() {
    let song: Song = serde_json::from_str(&read("song-alpha1.json")).expect("alpha.1 song parses");
    assert!(song.tempo_map.is_empty() && song.meter_map.is_empty());
    assert!(song.sections.is_empty() && song.markers.is_empty() && song.buses.is_empty());
    assert_eq!(song.engine, Some(4), "the pin travels with the file");
    let program = song.compile(&CompileOptions::default()).expect("compiles");
    assert_eq!(
        program.engine_version, 4,
        "an engine-4 song renders engine 4"
    );
    program.doc.validate().expect("the resolved doc validates");
    assert_eq!(
        program.hash, 0xd5c173a2aecefbc5,
        "the alpha.1 song compiles to its historical hash — if this changed, \
         the change is a compatibility break, intentional or not"
    );
    assert!(!program.render_mono().is_empty());
}

/// The oldest song shape (no pins at all): parses, compiles with the
/// historical legacy rule — the CURRENT engine, v1 schema semantics —
/// exactly as `song_pins_engine_and_version_at_creation` documents.
#[test]
fn legacy_minimal_song_keeps_the_legacy_rule() {
    let song: Song =
        serde_json::from_str(&read("song-legacy-minimal.json")).expect("legacy song parses");
    assert!(song.engine.is_none() && song.version.is_none());
    let doc = song.to_doc().expect("compiles");
    assert_eq!(doc.engine, Some(tono_core::dsl::ENGINE_VERSION));
    assert_eq!(doc.version, None, "v1 semantics for unpinned saves");
    doc.validate().expect("validates");
}

/// A PROGRAM_VERSION 1 bundle: the format's compat fixture. When
/// PROGRAM_VERSION next bumps, this file MUST still load, verify, and
/// report its versions — that is the loader's promise.
#[test]
fn program_v1_bundle_loads_and_verifies() {
    let program = Program::from_json(&read("program-v1.program.json")).expect("v1 bundle loads");
    assert_eq!(program.program_version, 1);
    assert_eq!(program.schema_version, 2);
    assert_eq!(program.engine_version, 5);
    assert_eq!(program.hash, 0x551776bbf16d6c78);
    assert_eq!(program.meta.name, "compat-fixture");
    assert!(!program.render_mono().is_empty());
    // And a byte-level hand-edit is still caught by the hash check.
    let tampered = read("program-v1.program.json").replacen("44100", "48000", 1);
    assert!(
        Program::from_json(&tampered).is_err(),
        "tampering is detected"
    );
}

/// The shipped parametric patch example: parses, validates params, and
/// renders — the patch format's compat fixture.
#[test]
fn shipped_patch_example_loads_and_renders() {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/examples/parametric-impact.patch.json"
    ))
    .expect("read shipped patch");
    let patch: Patch = serde_json::from_str(&json).expect("patch parses");
    let samples = patch
        .render(&patch.defaults().into_iter().collect())
        .expect("patch renders with defaults");
    assert!(
        samples.iter().any(|&x| x != 0.0),
        "the shipped patch sounds"
    );
}
