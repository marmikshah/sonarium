"""Tests for the typed tono API (Song/Pattern/Track/Program + instruments).

The headline check is the cross-language contract: the reference song built
here compiles to the same canonical Program hash as the identical Rust song in
crates/tono-core/tests/equivalence.rs — Rust owns semantics, so equivalent
songs must hash equal from either language. The rest covers render shapes and
determinism, bundle round-trips, and the structured error paths.

Run from the repo root after `make python`:
    make python-test
"""

import tempfile
from pathlib import Path

import numpy as np

import tono

# The cross-language pin — crates/tono-core/tests/equivalence.rs asserts the
# same value. If this changes, change both or the languages disagree.
REFERENCE_HASH = 0x0C866480DB6D4D71


def reference_program() -> "tono.Program":
    """The reference song, mirrored line-for-line by the Rust test."""
    song = tono.Song("night-drive", tempo=122.0)
    bass = song.track("bass", tono.instruments.bass("finger"))
    drums = song.track("drums", tono.instruments.drums("tr808"))
    riff = tono.Pattern(bars=1)
    riff.notes(["C2", "C2", "Eb2", "G2"], durations=0.5)
    beat = tono.Pattern(bars=1)
    beat.hit("kick", beats=[0, 2])
    beat.hit("snare", beats=[1, 3])
    song.arrange(bass, riff, bars=range(4))
    song.arrange(drums, beat, bars=range(4))
    return song.compile(sample_rate=48000)


def test_equivalence_hash_matches_rust() -> None:
    program = reference_program()
    # Cross-language contract — crates/tono-core/tests/equivalence.rs pins the
    # same hash for the same song built through the Rust API.
    assert program.hash == REFERENCE_HASH, (
        f"hash {program.hash:#x} != pinned {REFERENCE_HASH:#x} — "
        "the Rust and Python song builders diverge"
    )
    assert program.sample_rate == 48000
    assert [t["name"] for t in program.tracks] == ["bass", "drums"]
    assert [t["wave"] for t in program.tracks] == ["bass", "kit"]
    assert [t["notes"] for t in program.tracks] == [16, 16]
    assert program.estimates["events"] == 32
    assert program.is_streamable is False  # a tracks root warns (T1504)
    assert any(w["code"] == "T1504" for w in program.warnings)


def test_render_shapes_and_determinism() -> None:
    program = reference_program()
    stereo = program.render()
    assert stereo.dtype == np.float32
    assert stereo.ndim == 2 and stereo.shape[1] == 2, "shape (frames, 2)"
    assert stereo.flags["C_CONTIGUOUS"]
    frames = stereo.shape[0]
    expected = program.duration_seconds * program.sample_rate
    assert abs(frames - expected) <= 1.0, f"{frames} frames vs duration {expected}"
    assert np.array_equal(stereo, program.render()), "render must be byte-identical"

    mono = program.render_mono()
    assert mono.dtype == np.float32
    assert mono.shape == (frames,)
    # The mono render is the mid of the stereo pair.
    assert np.array_equal(mono, 0.5 * (stereo[:, 0] + stereo[:, 1]))


def test_bundle_round_trips() -> None:
    program = reference_program()
    assert tono.Program.from_json(program.to_json()).hash == program.hash
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "night-drive.program.json"
        program.save(str(path))
        assert tono.Program.load(str(path)).hash == program.hash


def test_error_paths() -> None:
    assert issubclass(tono.CompileError, tono.TonoError)

    # Arranging onto an unknown track fails the compile with structured
    # diagnostics (one per problem, each carrying code and path).
    song = tono.Song("bad", tempo=100.0)
    song.track("bass", tono.instruments.bass())
    pattern = tono.Pattern()
    pattern.note("C2")
    song.arrange("nope", pattern)
    try:
        song.compile()
        raise AssertionError("expected tono.CompileError")
    except tono.CompileError as exc:
        diags = exc.diagnostics
        assert isinstance(diags, list) and len(diags) == 1
        assert diags[0]["code"] == "T1001"
        assert diags[0]["path"] == "arrangement[0].track"
        assert diags[0]["severity"] == "error"
        assert "nope" in diags[0]["message"]
        assert "remediation" in diags[0]

    # Unknown instrument variant / drum name are plain ValueErrors.
    for bad_call in (
        lambda: tono.instruments.bass("nope"),
        lambda: tono.instruments.piano("honky_tonk"),  # the slug is honky-tonk
        lambda: tono.Pattern().hit("conga", beats=[0]),
    ):
        try:
            bad_call()
            raise AssertionError("expected ValueError")
        except ValueError as exc:
            assert "expected one of:" in str(exc), str(exc)

    # A durations list must match the pitch count.
    try:
        tono.Pattern().notes(["C4", "E4"], durations=[0.5])
        raise AssertionError("expected ValueError")
    except ValueError as exc:
        assert "durations" in str(exc)

    # A corrupt/invalid bundle is a TonoError, not a crash or a ValueError.
    for garbage in ("not json {", "{}"):
        try:
            tono.Program.from_json(garbage)
            raise AssertionError("expected tono.TonoError")
        except tono.TonoError:
            pass


def test_voice_builders_chain_and_pattern_repr() -> None:
    voice = tono.instruments.bass("pick")
    assert voice.gain(0.8) is voice, "builder methods return self"
    assert voice.pan(-0.25).reverb(0.4).swing(0.5).humanize(0.1) is voice
    assert voice.name == "pick bass"
    assert voice.wave == "bass"
    assert repr(voice) == "Voice('pick bass')"

    pattern = tono.Pattern(bars=2)
    pattern.note("C4", at=0.5, duration=0.25, gain=0.9)
    pattern.chord(["E4", "G4"])
    assert pattern.bars == 2
    assert repr(pattern) == "Pattern(bars=2, notes=3)"


def test_packaging_and_legacy_surface() -> None:
    assert (Path(tono.__file__).parent / "py.typed").exists(), "PEP 561 marker ships"
    assert callable(tono.render), "the legacy JSON render still works"
    import importlib

    importlib.import_module("tono.instruments")  # importable as a submodule too


def test_stems_render() -> None:
    song = tono.Song("stem-test", tempo=120)
    bass = song.track("bass", tono.instruments.bass("finger"))
    keys = song.track("keys", tono.instruments.piano("grand"))
    riff = tono.Pattern(bars=1)
    riff.notes(["C2", "G2"], durations=1.0)
    song.arrange(bass, riff, bars=1)
    song.arrange(keys, riff, bars=1)
    program = song.compile(sample_rate=48_000)
    stems = program.render_stems()
    assert set(stems) == {"bass", "keys"}, stems.keys()
    for arr in stems.values():
        assert arr.shape[1] == 2 and arr.dtype == np.float32
        assert np.abs(arr).max() > 0, "each stem sounds"
    # The stem sum is the mix the master chain hears (all master-routed here).
    total = sum(stems.values())
    mix = program.render()
    assert total.shape == mix.shape
    assert program.stem_routing == {}, "no bus routing in this song"


if __name__ == "__main__":
    test_equivalence_hash_matches_rust()
    test_render_shapes_and_determinism()
    test_bundle_round_trips()
    test_error_paths()
    test_voice_builders_chain_and_pattern_repr()
    test_packaging_and_legacy_surface()
    test_stems_render()
    print("all typed-API checks passed")
