"""Type stubs for the `tono` package.

Stability tiers (docs/api-tiers.md): the legacy JSON-string API is
*deprecated* (the typed API is the successor); the typed song API is
*experimental* through the 1.10.0 alphas.
"""

from typing import Any, Optional, Union

from fractions import Fraction

import numpy as np
import numpy.typing as npt

from . import instruments as instruments

__all__ = [
    "render",
    "Patch",
    "Engine",
    "Instrument",
    "DrumKit",
    "AdaptiveMusic",
    "PatchVoice",
    "TonoError",
    "CompileError",
    "Voice",
    "Pattern",
    "Track",
    "Song",
    "Program",
    "Pitch",
    "Key",
    "Chord",
    "instruments",
]

# A beat position: whole beats as int, an exact binary value as float (float
# 0.1 is its binary expansion, NOT 1/10 — use Fraction for exact decimals), a
# fractions.Fraction, or a (num, den) int tuple.
Beat = Union[int, float, Fraction, tuple[int, int]]

# --- legacy JSON-string API (deprecated; the typed API is the successor) ---

def render(doc_json: str) -> npt.NDArray[np.float32]:
    """Render a `SoundDoc` (JSON string) to a mono float32 array. Deterministic."""
    ...

class Patch:
    """A zero-asset SFX patch: a graph plus named parameters."""
    def __init__(self, json: str) -> None: ...
    def render(self, **params: float) -> npt.NDArray[np.float32]:
        """Render to a mono float32 array; named kwargs set parameters."""
        ...
    def defaults(self) -> dict[str, float]: ...

class Engine:
    """A live audio engine that owns an output stream."""
    def __init__(self, sample_rate: Optional[int] = None) -> None: ...
    @property
    def sample_rate(self) -> int: ...
    def instrument(self, name: str) -> Instrument: ...
    def drumkit(self) -> DrumKit: ...
    def adaptive(self) -> AdaptiveMusic: ...
    def load_patch(self, json: str) -> PatchVoice: ...

class Instrument:
    """A playable instrument voice in the mix."""
    def note_on(self, note: Union[int, str], velocity: float = 1.0) -> None: ...
    def note_off(self, note: Union[int, str]) -> None: ...
    def set_param(self, name: str, value: float) -> bool: ...
    def all_notes_off(self) -> None: ...

class DrumKit:
    """A General MIDI drum kit in the mix."""
    def note_on(self, note: Union[int, str], velocity: float = 1.0) -> None: ...

class AdaptiveMusic:
    """An adaptive-music bed: intensity-driven stems plus one-shot stingers."""
    def add_layer(self, doc_json: str, fade_in_at: float = 0.0) -> None: ...
    def set_intensity(self, x: float) -> None: ...
    def stinger(self, doc_json: str) -> None: ...

class PatchVoice:
    """A loaded SFX patch: trigger one-shot instances with named parameters."""
    def trigger(self, **params: float) -> None: ...

# --- typed song API (experimental through the 1.10.0 alphas) ---

class TonoError(Exception):
    """The base error for every typed-API failure."""
    ...

class CompileError(TonoError):
    """A song failed to compile; `.diagnostics` carries every problem found."""
    diagnostics: list[dict[str, Any]]

class Voice:
    """A catalog instrument voice — what a `Song` track plays."""
    @property
    def name(self) -> str: ...
    @property
    def wave(self) -> str: ...
    def gain(self, x: float) -> Voice:
        """Set the channel fader (0..2, 1 = unity); returns self for chaining."""
        ...
    def pan(self, x: float) -> Voice:
        """Set the stereo position (−1 .. 1); returns self for chaining."""
        ...
    def reverb(self, x: float) -> Voice:
        """Set the reverb send (0..1, 0 = dry); returns self for chaining."""
        ...
    def swing(self, x: float) -> Voice:
        """Override the song's swing for this track (0..1); returns self."""
        ...
    def humanize(self, x: float) -> Voice:
        """Override the song's humanize for this track (0..1); returns self."""
        ...

class Pattern:
    """A reusable musical phrase, `bars` long, on a 4 steps/beat grid (16
    steps per bar). The transform methods are pure: each returns a NEW
    pattern."""
    def __init__(self, bars: int = 1) -> None: ...
    @classmethod
    def euclidean(cls, pulses: int, steps: int, pitch: str, len: int = 1, bars: int = 1) -> Pattern:
        """`pulses` hits Bresenham-evenly across `steps` grid positions
        (`euclidean(3, 8, "midi:36")` → hits at 0, 3, 6). `bars` lengthens the
        pattern the cycle sits in (≥ ceil(steps/16))."""
        ...
    @classmethod
    def tuplet(cls, count: int, in_steps: int, pitch: str, len: int = 1) -> Pattern:
        """`count` notes spaced evenly across `in_steps` steps (round(i ×
        in_steps / count)), each `len` steps long, in a one-bar pattern."""
        ...
    @property
    def bars(self) -> int: ...
    def note(self, pitch: str, at: float = 0.0, duration: float = 1.0, gain: float = 1.0) -> None:
        """Place a note (`"C4"`, `"F#3"`, `"midi:36"`, or Hz) at beat `at`."""
        ...
    def notes(self, pitches: list[str], durations: Union[float, list[float]] = 1.0) -> None:
        """Place `pitches` sequentially; one shared duration or one per pitch."""
        ...
    def hit(self, drum: str, beats: list[float]) -> None:
        """Hit a drum (kick/snare/hat/openhat/clap/crash/ride/tom) at each beat."""
        ...
    def chord(self, pitches: list[str], at: float = 0.0, duration: float = 1.0, gain: float = 1.0) -> None:
        """Stack `pitches` as a chord at beat `at`."""
        ...
    def repeat(self, times: int) -> Pattern:
        """Repeated `times` times end-to-end (`bars × times`; 0 = silence)."""
        ...
    def concat(self, other: Pattern) -> Pattern:
        """This pattern with `other` appended (`bars + other.bars`)."""
        ...
    def layer(self, other: Pattern) -> Pattern:
        """Both patterns at once (merged, sorted by step); the longer length."""
        ...
    def slice(self, start: int, len: int) -> Pattern:
        """The window `[start, start + len)` in steps, re-based to step 0."""
        ...
    def transpose(self, semitones: int) -> Pattern:
        """Every pitch shifted (canonical sharp spelling; `"midi:N"` stays).
        Out-of-range or unparseable pitches raise ValueError."""
        ...
    def stretch(self, num: int, den: int) -> Pattern:
        """Time scaled by exactly num/den; off-grid results raise ValueError."""
        ...
    def rotate(self, shift: int) -> Pattern:
        """Starts moved by `shift` steps, wrapping around the pattern."""
        ...
    def reverse(self) -> Pattern:
        """The pattern mirrored in time."""
        ...
    def quantize(self, grid: int) -> Pattern:
        """Starts snapped to the nearest multiple of `grid` steps (halves forward)."""
        ...
    def vel(self, scale: float) -> Pattern:
        """Every velocity × `scale`, clamped to 0..1."""
        ...
    def gate(self, factor: float) -> Pattern:
        """Every length × `factor` (a note never vanishes)."""
        ...
    def probability(self, keep: float, seed: int = 0) -> Pattern:
        """Deterministic per-note keep/drop; same seed ⇒ same drops."""
        ...
    def humanize(self, timing: float = 0.0, velocity: float = 0.0, seed: int = 0) -> Pattern:
        """Deterministic per-note jitter baked into the pattern (structural)."""
        ...

class Track:
    """A handle on one of a song's tracks, returned by `Song.track`. The
    routing methods mutate the parent song."""
    @property
    def name(self) -> str: ...
    def route(self, bus: str) -> None:
        """Route the main output to mix bus `bus` (unknown bus → ValueError)."""
        ...
    def route_master(self) -> None:
        """Route the main output back to the master bus (the default)."""
        ...
    def send(self, bus: str, amount: float = 0.5) -> None:
        """Add a post-fader send to `bus` (0..1); a duplicate target raises ValueError."""
        ...
    def clear_sends(self, bus: Optional[str] = None) -> None:
        """Remove all sends, or only the send to `bus`."""
        ...

class Song:
    """A full song: tracks, patterns, and an arrangement."""
    def __init__(self, name: str, tempo: float = 120.0, seed: Optional[int] = None) -> None:
        """`seed` pins the deterministic RNG stream (None = document default 0)."""
        ...
    @property
    def name(self) -> str: ...
    @property
    def tempo(self) -> float: ...
    @property
    def track_names(self) -> list[str]: ...
    def track(self, name: str, voice: Voice) -> Track:
        """Add a track playing `voice` under `name`; returns its handle."""
        ...
    def arrange(self, track: Union[Track, str], pattern: Pattern, bars: Union[int, range, list[int]] = 0) -> None:
        """Place `pattern` on `track` at one bar (int) or each bar (range/list).
        With a meter map or pickup set, an off-grid bar raises ValueError."""
        ...
    def set_tempo_map(self, points: list[tuple[Beat, float]]) -> None:
        """Tempo changes `[(beat, bpm), ...]`. Eagerly validated (first point
        at beat 0, strictly ascending, positive finite bpm); `[]` clears."""
        ...
    def set_meter_map(self, points: list[tuple[int, int, int]]) -> None:
        """Time-signature changes `[(bar, numerator, denominator), ...]`.
        Eagerly validated (first at bar 0, ascending, numerator ≥ 1,
        denominator a power of two ≤ 64); `[]` clears."""
        ...
    def set_pickup(self, beat: Beat) -> None:
        """Bar 0's length in beats when it isn't a full bar (anacrusis)."""
        ...
    def clear_pickup(self) -> None:
        """Clear the pickup — bar 0 is full length again."""
        ...
    def add_section(self, name: str, bar: int, bars: int) -> None:
        """A named section of `bars` bars starting at `bar` (Program metadata)."""
        ...
    def add_marker(self, name: str, beat: Beat) -> None:
        """A named marker at an exact beat (Program metadata)."""
        ...
    def add_bus(self, id: str, gain: float = 1.0, effects: Optional[list[tuple[str, dict[str, Any]]]] = None) -> None:
        """A mix bus with an insert chain of `(type, params)` effects, e.g.
        `("reverb", {"room": 0.5, "mix": 0.3})`. Processor node types only."""
        ...
    def automate(self, track: Union[Track, str], target: str, points: list[tuple[float, float]], curve: str = "linear") -> None:
        """Automate a track's `"gain"`/`"pan"` with `[(beat, value), ...]`
        breakpoints (curve: linear/step/exp). Replaces the lane for `target`."""
        ...
    def compile(self, sample_rate: Optional[int] = None, target: str = "offline") -> Program:
        """Compile to an immutable `Program`; failures raise `CompileError`."""
        ...
    def to_json(self) -> str: ...
    @classmethod
    def from_json(cls, s: str) -> Song: ...

class Pitch:
    """An absolute pitch (`"C4"`, `"F#3"`, `"Gb5"`, or `"midi:N"`); equality
    is by MIDI number, so enharmonics compare equal."""
    def __init__(self, name: str) -> None: ...
    @property
    def midi(self) -> int: ...
    @property
    def name(self) -> str:
        """The canonical name (sharp spelling plus octave)."""
        ...
    def transpose(self, semitones: int) -> Pitch:
        """The pitch `semitones` away; outside the MIDI range → ValueError."""
        ...

class Key:
    """A key (`"C major"`, `"A minor"`, `"F# dorian"`): a scale on a tonic."""
    def __init__(self, name: str) -> None: ...
    @property
    def name(self) -> str: ...
    def degree(self, n: int, octave: int = 4) -> Pitch:
        """The pitch of scale degree `n` (1-based) in `octave`."""
        ...
    def contains(self, pitch: Pitch) -> bool:
        """Does `pitch` belong to the key?"""
        ...

class Chord:
    """A chord (`"C"`, `"Cm"`, `"Cmaj7"`, `"Cm7"`, `"C7"`, `"Cdim"`, `"Caug"`)."""
    def __init__(self, name: str) -> None: ...
    @property
    def name(self) -> str: ...
    def notes(self) -> list[str]:
        """The chord tones as pitch-class names, ascending from the root."""
        ...
    def pitches(self, octave: int = 4) -> list[Pitch]:
        """The root-position close voicing in `octave`."""
        ...
    def invert(self, n: int = 1, octave: int = 4) -> list[Pitch]:
        """The `n`-th inversion as a voicing in `octave`."""
        ...
    def arp(self, octave: int = 4) -> list[Pitch]:
        """The ascending arpeggio from the root (the same pitches as `pitches`)."""
        ...

class Program:
    """A compiled song: validated, resolved, hashed — the immutable artifact."""
    @property
    def hash(self) -> int:
        """The canonical content hash: equivalent songs hash equal."""
        ...
    @property
    def sample_rate(self) -> int: ...
    @property
    def duration_seconds(self) -> float: ...
    @property
    def tracks(self) -> list[dict[str, Any]]:
        """One dict per track: id, name, wave, notes, mute, solo."""
        ...
    @property
    def estimates(self) -> dict[str, int]:
        """frames, events, peak_voices, memory_bytes."""
        ...
    @property
    def warnings(self) -> list[dict[str, Any]]:
        """Compile warnings, as diagnostic dicts (code/severity/path/message/remediation?)."""
        ...
    @property
    def is_streamable(self) -> bool: ...
    def render(self) -> npt.NDArray[np.float32]:
        """Render to an owned float32 array of shape (frames, 2), C-order, L/R."""
        ...
    def render_mono(self) -> npt.NDArray[np.float32]:
        """Render to an owned float32 array of shape (frames,) — the stereo mid."""
        ...
    def render_stems(self) -> dict[str, npt.NDArray[np.float32]]:
        """Per-track and per-bus stereo stems (pre-master-chain): stem id →
        owned float32 array of shape (frames, 2). Muted tracks are silent."""
        ...
    @property
    def stem_routing(self) -> dict[str, str]:
        """`{track_id: bus_id}` for tracks routed to a bus (their stem is
        already inside that bus's stem)."""
        ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(s: str) -> Program: ...
    def save(self, path: str) -> None: ...
    @staticmethod
    def load(path: str) -> Program: ...
