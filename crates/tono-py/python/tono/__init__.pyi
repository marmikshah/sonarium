"""Type stubs for the `tono` package.

Stability tiers (docs/api-tiers.md): the legacy JSON-string API is
*deprecated* (the typed API is the successor); the typed song API is
*experimental* through the 1.10.0 alphas.
"""

from typing import Any, Optional, Union

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
    "instruments",
]

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
    """A reusable musical phrase, `bars` long, on a 4 steps/beat grid."""
    def __init__(self, bars: int = 1) -> None: ...
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

class Track:
    """A handle on one of a song's tracks, returned by `Song.track`."""
    @property
    def name(self) -> str: ...

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
        """Place `pattern` on `track` at one bar (int) or each bar (range/list)."""
        ...
    def compile(self, sample_rate: Optional[int] = None, target: str = "offline") -> Program:
        """Compile to an immutable `Program`; failures raise `CompileError`."""
        ...
    def to_json(self) -> str: ...
    @classmethod
    def from_json(cls, s: str) -> Song: ...

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
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(s: str) -> Program: ...
    def save(self, path: str) -> None: ...
    @staticmethod
    def load(path: str) -> Program: ...
