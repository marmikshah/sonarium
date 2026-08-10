"""night_drive — the typed tono API, end to end: compose, compile, render,
and run a program live with scheduled commands. Run with an installed tono
wheel (or `maturin develop` in crates/tono-py): python3 examples/night_drive.py"""

import tono

# Compose: no JSON anywhere — typed objects all the way down.
song = tono.Song("night-drive", tempo=122, seed=7)

drums = song.track("drums", tono.instruments.drums("tr808"))
bass = song.track("bass", tono.instruments.bass("finger"))
keys = song.track("keys", tono.instruments.electric_piano("rhodes"))

beat = tono.Pattern(bars=1)
beat.hit("kick", beats=[0, 2])
beat.hit("snare", beats=[1, 3])
beat.hit("hat", beats=[0.5, 1.5, 2.5, 3.5])

riff = tono.Pattern(bars=1)
riff.notes(["C2", "C2", "Eb2", "G2"], durations=0.5)

chords = tono.Pattern(bars=1)
chords.chord(["C4", "Eb4", "G4"], at=0, duration=2)
chords.chord(["Ab3", "C4", "Eb4"], at=2, duration=2)

song.arrange(drums, beat, bars=range(8))
song.arrange(bass, riff, bars=range(0, 8, 2))
song.arrange(keys, chords, bars=range(8))
song.add_section("outro", bar=6, bars=2)
song.automate(keys, "gain", [(0, 0.2), (6, 0.9)], curve="exp")

# Compile once: a hashed, validated artifact.
program = song.compile(sample_rate=48_000)
print(f"compiled: hash {hex(program.hash)}, {program.estimates}")

# Render the mix (float32, shape (frames, 2)) and the stems.
mix = program.render()
print(f"mix: {mix.shape} {mix.dtype}, peak {abs(mix).max():.3f}")
print("stems:", {k: v.shape for k, v in program.render_stems().items()})

# Run it live — headless here (no audio device needed); commands land on
# exact frames, so nothing ever sleeps.
with tono.Performance(program, headless=True) as perf:
    perf.play()
    perf.set_gain(0.6, at=tono.at_section("outro"))
    live = perf.fill(program.sample_rate * 12)
    print(f"ran live: {live.shape}; metrics: {perf.metrics()}")
