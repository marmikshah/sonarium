//! Print a small compiled Program bundle's JSON to stdout — the C smoke
//! test's fixture. `make capi` pipes this into
//! `target/capi-smoke.program.json` and hands the path to `tests/smoke.c`.

use tono_core::dsl::{Adsr, SeqWave};
use tono_core::song::{CompileOptions, Song, note};

fn main() {
    let mut song = Song::new("capi-smoke", 120.0);
    song.add_track(
        "bass",
        SeqWave::Bass,
        Adsr {
            a: 0.005,
            d: 0.1,
            s: 0.8,
            r: 0.2,
            punch: 0.0,
        },
    );
    song.add_pattern("riff", 1, vec![note(0, 4, "C2"), note(8, 4, "G2")]);
    song.arrange("bass", "riff", 0);
    let program = song
        .compile(&CompileOptions::default())
        .expect("the smoke song compiles");
    print!("{}", program.to_json());
}
