---
layout: home

hero:
  name: tono
  text: Audio as a pure function
  tagline: Procedural, deterministic, CI-testable. Compose music in code, compile it once, render or run it byte-identically anywhere.
  image:
    src: /img/logo.png
    alt: tono — a pluck waveform on a dark tile
  actions:
    - theme: brand
      text: Get started
      link: /get-started/
    - theme: alt
      text: Hear the showcase
      link: /showcase
    - theme: alt
      text: GitHub
      link: https://github.com/marmikshah/tono

features:
  - title: Sounds are data
    details: A sound is a JSON synthesis graph; rendering it is a pure function — byte-identical on every OS, pinned by engine revision. Test it, diff it, cache it in CI.
  - title: Zero-asset SFX
    details: A patch renders infinite variations from gameplay parameters — impacts that scale with collision force, footsteps that vary by surface. No sample library.
  - title: A real music runtime
    details: Sample-accurate scheduling, quantized section transitions, stingers, crossfaded swaps — plus mixer buses, polyphony caps, and adaptive intensity stems.
  - title: An ear built in
    details: Every render returns a spectrogram, a waveform, and LUFS/spectral stats — "does it sound right?" becomes numbers and pictures.
---
