# Mission core

`website-mission-core` owns headless mission compilation, validation, and the interactive mission document. The API uses the default `compiler` feature; the editor enables `doc` for Yrs and document operations.

## Layout

- `src/mission/ast`: authored payloads and compiled wire structures.
- `src/mission/compiler`: payload export, game-document compilation, and kit aliases.
- `src/mission/extensions`: authored-block contracts and validators.
- `src/mission/validation`: validation rules, failure fixtures, wire safety, and cargo constraints.
- `src/slot_line`: plain-text ORBAT summaries.
- `src/doc`: CRDT storage, ordered membership, row-based picking, and editor operations.

## Contracts

Wire keys, numeric conversions, authored order, diagnostics, and resource substitutions are preserved. Mission-core has no graphics, DOM, or UI dependencies. Host interaction state remains in the frontend; document operations accept explicit values and callbacks. The document and compiler tests use this crate directly.

## Verification

Run `cargo test -p website-mission-core --all-features` with the workspace target directory configured. The feature tripwire rejects incomplete test selections. Production Rust files remain below 500 lines, and out-of-line test files below 1,000.

The zone integration tests exercise save/reload, compiler projection, and deletion against the
document directly. They share the same headless contract as the editor and API.
