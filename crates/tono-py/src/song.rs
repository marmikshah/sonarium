//! The typed song API (ADR 0004): `tono.Song` / `tono.Pattern` / `tono.Track` /
//! `tono.Program` wrap the native Rust objects directly — building, compiling,
//! and rendering a song never crosses a JSON boundary. Rust owns semantics, so
//! an equivalent song compiles to the same Program hash from either language
//! (`crates/tono-core/tests/equivalence.rs` pins the contract).
//!
//! This API is **experimental** through the 1.10.0 alphas (docs/api-tiers.md).

use std::collections::HashMap;

use numpy::{IntoPyArray, PyArray1, PyArray2, PyArrayMethods};
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};

use tono_core::catalog::{self, Voice as CoreVoice};
use tono_core::diag::{CompileError as CoreCompileError, Diagnostic};
use tono_core::dsl::SeqWave;
use tono_core::program::Program as CoreProgram;
use tono_core::song::{CompileOptions, CompileTarget, Phrase, Song as CoreSong};

pyo3::create_exception!(
    tono,
    TonoError,
    pyo3::exceptions::PyException,
    "The base error for every typed-API failure (program load, …)."
);
pyo3::create_exception!(
    tono,
    CompileError,
    TonoError,
    "A song failed to compile. `.diagnostics` is a list of dicts — \
     {code, severity, path, message, remediation?} — one per problem found."
);

/// The document string of a seq wave (`SeqWave` serializes lowercase).
fn wave_str(wave: SeqWave) -> String {
    serde_json::to_value(wave)
        .expect("a SeqWave serializes")
        .as_str()
        .expect("a SeqWave serializes to a string")
        .to_owned()
}

/// Diagnostics as Python dicts: {code, severity, path, message, remediation?}.
fn diagnostics_list<'py>(
    py: Python<'py>,
    diagnostics: &[Diagnostic],
) -> PyResult<Bound<'py, PyList>> {
    let list = PyList::empty(py);
    for d in diagnostics {
        let dict = PyDict::new(py);
        dict.set_item("code", d.code)?;
        dict.set_item("severity", d.severity.to_string())?;
        dict.set_item("path", &d.path)?;
        dict.set_item("message", &d.message)?;
        if let Some(remediation) = &d.remediation {
            dict.set_item("remediation", remediation)?;
        }
        list.append(dict)?;
    }
    Ok(list)
}

/// Map a core compile failure to the Python `CompileError`, with the
/// structured diagnostics attached as `.diagnostics`.
fn compile_error(py: Python<'_>, err: CoreCompileError) -> PyErr {
    let pyerr = CompileError::new_err(err.to_string());
    match diagnostics_list(py, &err.0) {
        Ok(list) => {
            // Attaching to a fresh exception instance cannot realistically
            // fail; worst case the exception carries only its message.
            let _ = pyerr.value(py).setattr("diagnostics", list);
        }
        Err(e) => return e,
    }
    pyerr
}

/// A catalog instrument voice — what a `Song` track plays. Construct one with
/// the `tono.instruments` functions; tune it with the builder methods.
#[pyclass(module = "tono")]
struct Voice {
    inner: CoreVoice,
}

#[pymethods]
impl Voice {
    /// The voice's display name (read-only).
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The synthesis wave the voice plays, as its document string (`"piano"`,
    /// `"bass"`, `"kit"`, …) (read-only).
    #[getter]
    fn wave(&self) -> String {
        wave_str(self.inner.wave)
    }

    /// Set the channel fader (0..2, 1 = unity); returns the voice for chaining.
    fn gain<'py>(mut slf: PyRefMut<'py, Self>, x: f32) -> PyRefMut<'py, Self> {
        slf.inner.gain = x;
        slf
    }

    /// Set the stereo position (−1 hard left .. 1 hard right); returns the
    /// voice for chaining.
    fn pan<'py>(mut slf: PyRefMut<'py, Self>, x: f32) -> PyRefMut<'py, Self> {
        slf.inner.pan = x;
        slf
    }

    /// Set the reverb send (0..1, 0 = dry); returns the voice for chaining.
    fn reverb<'py>(mut slf: PyRefMut<'py, Self>, x: f32) -> PyRefMut<'py, Self> {
        slf.inner.reverb = x;
        slf
    }

    /// Override the song's swing for this track (0..1); returns the voice for
    /// chaining.
    fn swing<'py>(mut slf: PyRefMut<'py, Self>, x: f32) -> PyRefMut<'py, Self> {
        slf.inner.swing = Some(x);
        slf
    }

    /// Override the song's humanize for this track (0..1); returns the voice
    /// for chaining.
    fn humanize<'py>(mut slf: PyRefMut<'py, Self>, x: f32) -> PyRefMut<'py, Self> {
        slf.inner.humanize = Some(x);
        slf
    }

    fn __repr__(&self) -> String {
        format!("Voice('{}')", self.inner.name)
    }
}

/// Reject an unknown instrument variant, naming the valid ones.
fn unknown_variant(family: &str, variant: &str, valid: &[&str]) -> PyErr {
    PyValueError::new_err(format!(
        "unknown {family} variant {variant:?} — expected one of: {}",
        valid.join(", ")
    ))
}

/// One `tono.instruments` family function: variant slug → catalog constructor
/// (the same slugs the `tono catalog` CLI lists).
macro_rules! instrument_fn {
    ($name:ident, $doc:literal, $family:literal, $default:literal, $($slug:literal => $ctor:path),+ $(,)?) => {
        #[doc = $doc]
        #[pyfunction]
        #[pyo3(signature = (variant = $default))]
        fn $name(variant: &str) -> PyResult<Voice> {
            let inner = match variant {
                $($slug => $ctor(),)+
                other => return Err(unknown_variant($family, other, &[$($slug),+])),
            };
            Ok(Voice { inner })
        }
    };
}

instrument_fn!(
    piano,
    "A piano voice — variants: grand (default), bright, mellow, felt, upright, honky-tonk.",
    "piano",
    "grand",
    "grand" => catalog::GrandPiano::grand,
    "bright" => catalog::GrandPiano::bright,
    "mellow" => catalog::GrandPiano::mellow,
    "felt" => catalog::GrandPiano::felt,
    "upright" => catalog::GrandPiano::upright,
    "honky-tonk" => catalog::GrandPiano::honky_tonk,
);
instrument_fn!(
    electric_piano,
    "An electric piano voice — variants: rhodes (default), wurli, dx.",
    "electric_piano",
    "rhodes",
    "rhodes" => catalog::ElectricPiano::rhodes,
    "wurli" => catalog::ElectricPiano::wurli,
    "dx" => catalog::ElectricPiano::dx,
);
instrument_fn!(
    organ,
    "An organ voice — variants: tonewheel (default), rock.",
    "organ",
    "tonewheel",
    "tonewheel" => catalog::Organ::tonewheel,
    "rock" => catalog::Organ::rock,
);
instrument_fn!(
    strings,
    "A string ensemble voice — variants: warm (default), ensemble.",
    "strings",
    "warm",
    "warm" => catalog::Strings::warm,
    "ensemble" => catalog::Strings::ensemble,
);
instrument_fn!(
    bass,
    "A bass voice — variants: finger (default), pick, sub, synth.",
    "bass",
    "finger",
    "finger" => catalog::Bass::finger,
    "pick" => catalog::Bass::pick,
    "sub" => catalog::Bass::sub,
    "synth" => catalog::Bass::synth,
);
instrument_fn!(
    guitar,
    "A guitar voice — variants: nylon (default), steel, electric.",
    "guitar",
    "nylon",
    "nylon" => catalog::Guitar::nylon,
    "steel" => catalog::Guitar::steel,
    "electric" => catalog::Guitar::electric,
);
instrument_fn!(
    drums,
    "A General MIDI drum kit — variants: acoustic (default), classic, electronic, tr808.",
    "drums",
    "acoustic",
    "acoustic" => catalog::Drums::acoustic,
    "classic" => catalog::Drums::classic,
    "electronic" => catalog::Drums::electronic,
    "tr808" => catalog::Drums::tr808,
);
instrument_fn!(
    brass,
    "A brass voice — variants: section (default), stab.",
    "brass",
    "section",
    "section" => catalog::Brass::section,
    "stab" => catalog::Brass::stab,
);
instrument_fn!(
    flute,
    "A flute voice — variants: concert (default).",
    "flute",
    "concert",
    "concert" => catalog::Flute::concert,
);
instrument_fn!(
    mallets,
    "A mallets voice — variants: marimba (default), vibraphone, glockenspiel.",
    "mallets",
    "marimba",
    "marimba" => catalog::Mallets::marimba,
    "vibraphone" => catalog::Mallets::vibraphone,
    "glockenspiel" => catalog::Mallets::glockenspiel,
);
instrument_fn!(
    bells,
    "A bells voice — variants: tubular (default).",
    "bells",
    "tubular",
    "tubular" => catalog::Bells::tubular,
);

/// A reusable musical phrase, `bars` long, on the song's grid of 4 steps per
/// beat (sixteenth notes) — songs created through this API keep that default
/// grid. Notes are placed by beat (floats welcome: beat 0.5 is the second
/// eighth note) and snap to the grid exactly like the Rust `Phrase` writer.
#[pyclass(module = "tono")]
struct Pattern {
    bars: u32,
    phrase: Phrase,
    /// The written-note count, for `__repr__` (the phrase keeps its notes
    /// private; the count is all the repr needs).
    count: usize,
}

#[pymethods]
impl Pattern {
    /// An empty pattern `bars` long.
    #[new]
    #[pyo3(signature = (bars=1))]
    fn new(bars: u32) -> Self {
        Pattern {
            bars: bars.max(1),
            phrase: Phrase::new(4),
            count: 0,
        }
    }

    /// The pattern's length in bars.
    #[getter]
    fn bars(&self) -> u32 {
        self.bars
    }

    /// Place a note at beat `at`, `duration` beats long, with velocity `gain`
    /// (0..1). `pitch` is a note name (`"C4"`, `"F#3"`), a MIDI note
    /// (`"midi:36"`), or Hz.
    #[pyo3(signature = (pitch, at=0.0, duration=1.0, gain=1.0))]
    fn note(&mut self, pitch: &str, at: f32, duration: f32, gain: f32) {
        self.phrase.at(at).vel(gain).note(pitch, duration);
        self.count += 1;
    }

    /// Place `pitches` one after another from the pattern's start (the cursor
    /// advances past each note). `durations` is one float (the same length for
    /// every note) or a list of floats (one per pitch).
    #[pyo3(signature = (pitches, durations=None))]
    fn notes(
        &mut self,
        pitches: Vec<String>,
        durations: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let durs: Vec<f32> = match durations {
            None => vec![1.0; pitches.len()],
            Some(obj) => {
                if let Ok(d) = obj.extract::<f32>() {
                    vec![d; pitches.len()]
                } else if let Ok(ds) = obj.extract::<Vec<f32>>() {
                    if ds.len() != pitches.len() {
                        return Err(PyValueError::new_err(format!(
                            "durations has {} entries but {} pitches were given — \
                             pass one float or one per pitch",
                            ds.len(),
                            pitches.len()
                        )));
                    }
                    ds
                } else {
                    return Err(PyValueError::new_err(
                        "durations must be a float or a list of floats",
                    ));
                }
            }
        };
        for (pitch, dur) in pitches.iter().zip(durs) {
            self.phrase.play(pitch, dur);
        }
        self.count += pitches.len();
        Ok(())
    }

    /// Hit a drum at each beat in `beats` — a one-step GM hit, meaningful on a
    /// drums voice. `drum` is one of: kick, snare, hat, openhat, clap, crash,
    /// ride, tom.
    fn hit(&mut self, drum: &str, beats: Vec<f32>) -> PyResult<()> {
        // Mirrors the GM constants of Phrase's drum helpers.
        let gm = match drum {
            "kick" => 36,
            "snare" => 38,
            "hat" => 42,
            "openhat" => 46,
            "clap" => 39,
            "crash" => 49,
            "ride" => 51,
            "tom" => 45,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown drum {other:?} — expected one of: \
                     kick, snare, hat, openhat, clap, crash, ride, tom"
                )));
            }
        };
        self.count += beats.len();
        for b in beats {
            self.phrase.at(b).hit(gm);
        }
        Ok(())
    }

    /// Stack `pitches` as a chord at beat `at`, `duration` beats long, with
    /// velocity `gain` (0..1).
    #[pyo3(signature = (pitches, at=0.0, duration=1.0, gain=1.0))]
    fn chord(&mut self, pitches: Vec<String>, at: f32, duration: f32, gain: f32) {
        let refs: Vec<&str> = pitches.iter().map(String::as_str).collect();
        self.phrase.at(at).vel(gain).chord(&refs, duration);
        self.count += pitches.len();
    }

    fn __repr__(&self) -> String {
        format!("Pattern(bars={}, notes={})", self.bars, self.count)
    }
}

/// A handle on one of a song's tracks — returned by `Song.track`. Arranging
/// onto it arranges onto the final (slugified, deduplicated) track name, which
/// is also the rendered layer id.
#[pyclass(module = "tono")]
struct Track {
    name: String,
}

#[pymethods]
impl Track {
    /// The track's final name.
    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    fn __repr__(&self) -> String {
        format!("Track('{}')", self.name)
    }
}

/// A full song: tracks (catalog voices), patterns (phrases), and an
/// arrangement. `compile` validates and lowers it to an immutable `Program`.
#[pyclass(module = "tono")]
struct Song {
    inner: CoreSong,
    /// Registered patterns, keyed by the Python `id()` of the `Pattern`
    /// object: the first `arrange` of a pattern registers it as `pattern_{n}`
    /// and keeps it alive (so its id can't be recycled) for later placements.
    patterns: HashMap<usize, (Py<Pattern>, String)>,
}

#[pymethods]
impl Song {
    /// A song named `name` at `tempo` BPM. `seed` pins the deterministic RNG
    /// stream everything stochastic draws from (None = the document default,
    /// 0): same song + same seed ⇒ same program hash.
    #[new]
    #[pyo3(signature = (name, tempo=120.0, seed=None))]
    fn new(name: &str, tempo: f32, seed: Option<u64>) -> Self {
        let mut inner = CoreSong::new(name, tempo);
        if let Some(seed) = seed {
            inner = inner.with_seed(seed);
        }
        Song {
            inner,
            patterns: HashMap::new(),
        }
    }

    /// The song's name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The tempo in beats per minute.
    #[getter]
    fn tempo(&self) -> f32 {
        self.inner.bpm
    }

    /// The track names, in declaration order.
    #[getter]
    fn track_names(&self) -> Vec<String> {
        self.inner.tracks.iter().map(|t| t.name.clone()).collect()
    }

    /// Add a track playing `voice` under `name` (slugified and deduplicated —
    /// the final name keeps layer ids stable across faces) and return its
    /// `Track` handle. Notes come from the patterns arranged onto it.
    fn track(&mut self, name: &str, voice: PyRef<'_, Voice>) -> Track {
        self.inner.add_voice(name, &voice.inner);
        let final_name = self
            .inner
            .tracks
            .last()
            .expect("add_voice just pushed a track")
            .name
            .clone();
        Track { name: final_name }
    }

    /// Arrange `pattern` onto `track` (a `Track` handle or a track name).
    /// `bars` is an int (place at that one bar) or a range/list of ints
    /// (place at each). The first arrange of a pattern registers it under
    /// `pattern_{n}` (1-based, in registration order).
    #[pyo3(signature = (track, pattern, bars=None))]
    fn arrange(
        &mut self,
        track: &Bound<'_, PyAny>,
        pattern: &Bound<'_, Pattern>,
        bars: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let track_name: String = if let Ok(cell) = track.cast::<Track>() {
            cell.borrow().name.clone()
        } else if let Ok(name) = track.extract::<String>() {
            name
        } else {
            return Err(PyValueError::new_err(
                "track must be a Track or a track name string",
            ));
        };
        let bars: Vec<u32> = match bars {
            None => vec![0],
            Some(obj) => {
                if let Ok(bar) = obj.extract::<u32>() {
                    vec![bar]
                } else if let Ok(list) = obj.extract::<Vec<u32>>() {
                    list
                } else {
                    return Err(PyValueError::new_err(
                        "bars must be an int or a range/list of ints",
                    ));
                }
            }
        };

        let key = pattern.as_ptr() as usize;
        let registered = match self.patterns.get(&key) {
            Some((_, name)) => name.clone(),
            None => {
                // 1-based count of registered patterns, skipping names a
                // loaded (from_json) song already carries.
                let mut n = self.patterns.len() + 1;
                let mut name = format!("pattern_{n}");
                while self.inner.patterns.iter().any(|p| p.name == name) {
                    n += 1;
                    name = format!("pattern_{n}");
                }
                let (bars_len, notes) = {
                    let p = pattern.borrow();
                    (p.bars, p.phrase.clone().into_notes())
                };
                self.inner.add_pattern(&name, bars_len, notes);
                self.patterns
                    .insert(key, (pattern.clone().unbind(), name.clone()));
                name
            }
        };
        for bar in bars {
            self.inner.arrange(&track_name, &registered, bar);
        }
        Ok(())
    }

    /// Compile the song to an immutable `Program` — validation collects every
    /// problem in one pass and failures raise `tono.CompileError` carrying the
    /// structured `.diagnostics`. `target` is `"offline"` (the default) or
    /// `"runtime"`; in alpha.1 both produce the same artifact.
    #[pyo3(signature = (sample_rate=None, target="offline"))]
    fn compile(&self, py: Python<'_>, sample_rate: Option<u32>, target: &str) -> PyResult<Program> {
        let target = match target {
            "offline" => CompileTarget::Offline,
            "runtime" => CompileTarget::Runtime,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown compile target {other:?} — expected 'offline' or 'runtime'"
                )));
            }
        };
        let opts = CompileOptions {
            sample_rate,
            target,
        };
        match py.detach(|| self.inner.compile(&opts)) {
            Ok(program) => Ok(Program { inner: program }),
            Err(err) => Err(compile_error(py, err)),
        }
    }

    /// Serialize the song (its saveable project form).
    fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).expect("a song serializes")
    }

    /// Load a song saved with `to_json`.
    #[classmethod]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        let inner: CoreSong =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Song {
            inner,
            patterns: HashMap::new(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Song('{}', tempo={:?}, tracks={})",
            self.inner.name,
            self.inner.bpm,
            self.inner.tracks.len()
        )
    }
}

/// A compiled song: validated, resolved, hashed — the immutable artifact
/// applications render and ship.
#[pyclass(module = "tono")]
struct Program {
    inner: CoreProgram,
}

#[pymethods]
impl Program {
    /// The canonical content hash: equivalent songs hash equal, from Rust or
    /// Python alike.
    #[getter]
    fn hash(&self) -> u64 {
        self.inner.hash
    }

    /// The sample rate the program was compiled for.
    #[getter]
    fn sample_rate(&self) -> u32 {
        self.inner.meta.sample_rate
    }

    /// The total duration in seconds, including the release/reverb tail.
    #[getter]
    fn duration_seconds(&self) -> f32 {
        self.inner.meta.duration_secs
    }

    /// One dict per track, in declaration order: id, name, wave, notes, mute,
    /// solo.
    #[getter]
    fn tracks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty(py);
        for t in &self.inner.meta.tracks {
            let dict = PyDict::new(py);
            dict.set_item("id", t.id.get())?;
            dict.set_item("name", &t.name)?;
            dict.set_item("wave", wave_str(t.wave))?;
            dict.set_item("notes", t.notes)?;
            dict.set_item("mute", t.mute)?;
            dict.set_item("solo", t.solo)?;
            list.append(dict)?;
        }
        Ok(list)
    }

    /// Bounded estimates of what the program costs to render or run:
    /// frames, events, peak_voices, memory_bytes.
    #[getter]
    fn estimates<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("frames", self.inner.estimates.frames)?;
        dict.set_item("events", self.inner.estimates.events)?;
        dict.set_item("peak_voices", self.inner.estimates.peak_voices)?;
        dict.set_item("memory_bytes", self.inner.estimates.memory_bytes)?;
        Ok(dict)
    }

    /// Compile warnings (the streaming blockers), as diagnostic dicts of the
    /// same shape as `CompileError.diagnostics`.
    #[getter]
    fn warnings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        diagnostics_list(py, &self.inner.warnings)
    }

    /// Whether the resolved document streams natively.
    #[getter]
    fn is_streamable(&self) -> bool {
        self.inner.is_streamable()
    }

    /// Render the full program to a stereo `np.ndarray` of shape
    /// `(frames, 2)`, dtype `float32`, C-order, channel order L/R. The array
    /// is an owned copy: safe to keep or mutate, and rendering again yields
    /// the same bytes.
    fn render<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f32>>> {
        let (left, right) = py.detach(|| self.inner.render_stereo());
        let frames = left.len();
        let mut interleaved = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            interleaved.push(left[i]);
            interleaved.push(right[i]);
        }
        interleaved.into_pyarray(py).reshape([frames, 2])
    }

    /// Render the full program to a mono `np.ndarray` of shape `(frames,)`,
    /// dtype `float32` — the mid of the stereo render, an owned copy.
    fn render_mono<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        py.detach(|| self.inner.render_mono()).into_pyarray(py)
    }

    /// Serialize the program bundle (compact JSON).
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// Load a program bundle: rejects bundles newer than this binary (T3001)
    /// and re-verifies the content hash (T3002).
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        CoreProgram::from_json(json)
            .map(|inner| Program { inner })
            .map_err(|e| TonoError::new_err(e.to_string()))
    }

    /// Write the program bundle to `path` (see `to_json`).
    fn save(&self, path: &str) -> PyResult<()> {
        std::fs::write(path, self.inner.to_json()).map_err(|e| PyOSError::new_err(e.to_string()))
    }

    /// Read a program bundle from `path` (see `from_json`).
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let json = std::fs::read_to_string(path).map_err(|e| PyOSError::new_err(e.to_string()))?;
        Program::from_json(&json)
    }

    fn __repr__(&self) -> String {
        format!(
            "Program('{}', hash={:#018x}, tracks={})",
            self.inner.meta.name,
            self.inner.hash,
            self.inner.meta.tracks.len()
        )
    }
}

/// Register the typed-API classes, exceptions, and the `instruments`
/// submodule on the extension module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("TonoError", m.py().get_type::<TonoError>())?;
    m.add("CompileError", m.py().get_type::<CompileError>())?;
    m.add_class::<Voice>()?;
    m.add_class::<Pattern>()?;
    m.add_class::<Track>()?;
    m.add_class::<Song>()?;
    m.add_class::<Program>()?;

    let instruments = PyModule::new(m.py(), "instruments")?;
    instruments.add_function(wrap_pyfunction!(piano, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(electric_piano, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(organ, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(strings, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(bass, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(guitar, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(drums, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(brass, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(flute, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(mallets, &instruments)?)?;
    instruments.add_function(wrap_pyfunction!(bells, &instruments)?)?;
    m.add_submodule(&instruments)?;
    Ok(())
}
