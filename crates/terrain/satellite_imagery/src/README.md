# Satellite imagery source

The source of `satellite_imagery`: the container's header and both index versions, the index
checks, the level picks, the error they share, and the crate root that declares them.

## Contents

```text
crates/terrain/satellite_imagery/src/
├── archive.rs     version 2: the mip chain and tile rectangles derived from the archived index
├── error.rs       `Error` and `Result`: a `TbdSatError` behind one type
├── header.rs      the index's end byte from a 12-byte prefix, and the header and index parse
├── lib.rs         the crate root: module header, `mod` lines and re-exports
├── model.rs       `TbdSatIndex`, its levels and tiles, `TbdSatError`, and the format constants
├── prelude.rs     the names most readers import
├── selection.rs   the base level for a texture limit and the preview level for an edge size
├── tests/         unit tests that hold both container versions to the same answer
└── validation.rs  the loose and strict index checks
```

## How it works

`header` reads a container into one `TbdSatIndex`, through `archive` for version 2;
`validation` parses and then checks it, loosely or strictly; `selection` reads a checked index.
The modules are private; `lib.rs` re-exports their public items.

## Boundaries

- Depends on: `world_file_formats`, `serde`, `serde_json` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
