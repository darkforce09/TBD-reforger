# Enfusion pak archive reader

The `enfusion_pak` crate: read-only access to the `.pak` archives an
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) install ships under `addons/` (the
`FORM`/`PAC1` container) and to loose extracted folders, behind one virtual file system. One
parser and one decompressor serve two consumers with different rules: the building-blueprint
compiler, and the world export, map raster and `enf` tooling.

## Contents

```text
tools/enfusion/enfusion_pak/
├── Cargo.toml  the `enfusion_pak` library package: `flate2`, `thiserror`; layout tier 0
└── src/        the archive parser, the decompressor, the merged virtual file system, the loose and layered sources, the error type
```

## How it works

```text
<game>/addons/*.pak ──▶ PakIndex::open (FORM/PAC1 chunks, FILE directory) ──▶ PakSet (sorted, first holder wins)
                                                                               ├─ blueprint policy: PakSet::from_dir
                                                                               └─ world policy:     PakVfs::open / open_default
extracted folder ──▶ DirSource ─┐
PakSet           ───────────────┴▶ LayeredSource (first source holding the path) ──▶ AssetSource::read
```

The blueprint policy folds path case, requires every entry inside the `DATA` chunk, accepts only
zlib with the exact decompressed length, and fails on any malformed archive; the world policy keeps
path case, falls back to raw deflate, and skips a malformed archive with a message on stderr.
`src/README.md` holds the full rule table and each module.

## Getting started

Run from the repository root:

```bash
cargo test -p enfusion_pak   # synthetic archives under both policies; the real-install checks are ignored
```

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ENFUSION_GAME_PATH` | `$HOME/.cache/enfusion-mcp-root` | the game folder `PakVfs::open_default` opens; its `addons/` holds the paks |
| `HOME` | none | the base of the default game folder and of `PakSet::default_dir` |

## Boundaries

- Depends on: `flate2` (zlib and raw deflate), `thiserror`; no workspace crate.
- Used by: the `blueprint_compiler`, `world_export_pipeline` and `map_raster_pipeline` crates
  and the `enfusion_script_index` crate (`enf extract`, `enf dump-entry`).
- Rules: tier 0 of `tools/enfusion`; both consumers share one parser and one decompressor and
  differ only through the crate-private read policy; every failure is an `Error`, never a panic.

## Related documentation

- [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md) — `enf
  extract` and `enf dump-entry`, which read the paks through this crate.
- [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) — the
  world export that reads the game's terrain through `PakVfs`.
