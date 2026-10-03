# Library crates

The workspace's library crates, one folder per category: `crates/<category>/<name>`, where the
package name equals the folder name. Applications under `apps/` and tools under `tools/` link
them; a library crate never depends on an application.

## Contents

```text
crates/
├── ballistics/  the mortar ballistics: the catalog and flight model, the solver, the fire-mission planner
├── contracts/   shapes and policies two programs share, such as the offline cache policy
├── foundation/  dependency-free building blocks, such as the HTTP URL guard
├── geometry/  plain geometry, map coordinates, camera arithmetic and spatial indexes, such as the BVH
├── graphics/  map-agnostic renderer building blocks, such as the render primitives
├── line_of_sight/  visibility over the bare ground, inside one building and through the placed world
├── map_overlay/  what the map draws on the terrain and in what order, such as the draw lanes and unit symbology
├── mission/   the mission domain's shared crates, such as the wire-safety scans
├── mission_editing/  the Mission Creator's editing layer over the mission document, such as the editing session
├── streaming/  the streamed world's CPU half, such as the chunk scheduler and the draw buffers
├── terrain/  the ground the map reads, such as the elevation model and the satellite container reader
├── world_formats/  the files a terrain's map data is stored in and their readers, such as the chunks
└── world_objects/  what stands on the ground, such as the vegetation and the building interiors
```

## How it works

Every manifest under `crates/` is a workspace member through the root `Cargo.toml` entry
`crates/*/*`, and each one declares `[package.metadata.layout]`: its `category` (the parent
folder, such as `crates/foundation`), its `tier` and its `targets`. A crate's tier is 0 with no
workspace dependency and otherwise 1 plus the highest tier it depends on, so edges point strictly
down. Foundation crates depend on foundation crates only, most on none; contracts crates depend
on foundation crates only. Each library crate keeps the same anatomy: a `lib.rs` of at most 80
lines holding only the module header, `mod` lines and `pub use` lines, a `prelude` module, an
`error.rs` when its public API is fallible, and a README with a Contents block.

## Getting started

Run from the repository root:

```bash
cargo test -p http_url_guard -p offline_cache_policy   # every library crate's unit tests
cargo xtask verify crate-tiers                         # membership, layout, tiers, category edges
cargo xtask verify crate-anatomy                       # lib.rs, prelude, error, README, manifest
```

## Boundaries

- Depends on: external crates from the root `[workspace.dependencies]` only.
- Used by: the applications under `apps/` and the tools under `tools/`.
- Rules: every manifest here is a workspace member with a layout declaration whose category equals
  its parent folder and whose package name equals its folder name, and every dependency edge
  points to a lower tier along the category matrix (`cargo xtask verify crate-tiers`); every
  library crate keeps the anatomy above (`cargo xtask verify crate-anatomy`); nothing here depends
  on `legacy/` (`cargo xtask verify strangler`).

## Related documentation

- [Laws and gates](/documentation/restructure/laws_and_gates.md) — the crate-tier and
  crate-anatomy laws in full.
- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the dependency
  directions between the workspace crates.
