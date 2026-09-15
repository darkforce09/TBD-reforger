# Mission core

`website-mission-core` owns headless mission compilation and validation. The API consumes the default `compiler` feature. The frontend currently reaches these interfaces through the temporary `map-engine-core` re-exports while the document store moves into this crate.

## Layout

- `src/mission/ast`: authored payload and compiled wire structures, faction hierarchy projections.
- `src/mission/compiler`: saved payloads, export envelopes, game-document compilation, and kit aliases.
- `src/mission/extensions`: the supported authored-block registry and validators.
- `src/mission/validation`: validation registry, failure fixtures, wire safety, and cargo constraints.
- `src/slot_line`: plain-text ORBAT summaries.
- `src/doc`: scaffold for the document store and operations relocation.

## Contracts

Mission wire keys, numeric conversions, authored order, diagnostics, and resource-substitution behavior are preserved. The crate does not import graphics, DOM, or UI types. The temporary development dependency keeps compiler/document round-trip tests live until document ownership is transferred; it is not a production dependency.

## Verification

Run `cargo test -p website-mission-core --all-features`. The feature tripwire rejects incomplete test selections. Each production Rust file stays below 500 lines and tests below 1,000; tests are declared out of line.
