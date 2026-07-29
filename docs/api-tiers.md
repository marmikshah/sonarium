# API stability tiers

tono's compatibility promise is tiered. The tier of every public surface is
stated where it is documented (module docs, changelog entries, the Python
stubs); anything not marked is **stable**.

## Stable

The byte-identity guarantee and the core document/runtime surface: the
`SoundDoc` schema (at a pinned `version`), each `engine` revision's rendered
output, the render/streaming entry points, the catalog voices, and the
CLI's command surface.

- Follows SemVer on the 1.x line: **no breaking changes, ever** — there is
  no 2.0 to hide them in (see CLAUDE.md's versioning note).
- Evolution is additive: new nodes, voices, fields with defaults, new
  subcommands. Documents and songs pinned at older schema/engine revisions
  keep their exact historical behavior; compatibility is carried by the
  revision pins, not by API breakage.

## Experimental

New surface finding its shape — marked `experimental` in rustdoc/stubs and
in the changelog. The Song → Program composition API is experimental
through the 1.10.0 alphas and freezes at 1.10.0-rc.1.

- May change in any minor release, with the change called out in the
  changelog's `### Changed` section.
- Removal follows the repo's standing policy: deprecated experimental
  surface is removed in the next minor, no long-lived shims — one reason
  experimental stays clearly marked.

## Internal

`pub(crate)` items, `#[doc(hidden)]` items, and anything behind an
underscore in Python. No promises; never rely on them across versions.

## Deprecation

Any tier can deprecate surface: the old form keeps working for at least one
minor with the successor named in docs and changelog, then the repo's
no-shims removal policy applies. The legacy JSON-string Python calls
(`tono.render(doc_json)`, `Patch(json)`, `AdaptiveMusic.add_layer(doc_json)`)
are deprecated as of 1.10.0-alpha.1; the typed API is the successor.
