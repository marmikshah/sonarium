"""tono — a deterministic sound engine, from Python.

The **typed API** (experimental through the 1.10.0 alphas, see
docs/api-tiers.md) authors and compiles songs natively — `Song`, `Pattern`,
`Track`, `Program`, the `instruments` catalog — with no JSON in the path and a
canonical program hash equivalent songs reproduce from Rust or Python alike.

The **legacy JSON-string API** (`render`, `Patch`, `Engine`, `Instrument`,
`DrumKit`, `AdaptiveMusic`, `PatchVoice`) keeps working through v1.10
(deprecated per docs/api-tiers.md; the typed API is the successor).
"""

from ._tono import *  # noqa: F401,F403 — re-exports the whole native surface
from . import _tono as _native
import sys as _sys

# `instruments` is a native submodule of the extension; register it under the
# package name so `import tono.instruments` works as well as the attribute.
_sys.modules[__name__ + ".instruments"] = _native.instruments
