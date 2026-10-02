# Parked engine crates

The parking folder of the two engine monoliths, `map_engine` and `graphics_engine`. Their code
moves, a module at a time, into the tiered library crates under `crates/`; until a module has
moved, the single-page app, the API and the developer tools link it from here.

## Contents

```text
legacy/
├── graphics_engine/  the `graphics_engine` crate: the `wgpu` renderer, which knows no map concept
└── map_engine/       the `map_engine` crate: the mission domain, world, streaming, spatial queries, map rendering
```

## How it works

Both crates are members of the root Cargo workspace and build as they always do: the frontend
links `map_engine` for the browser and native targets, the API links it for the mission
domain, and the developer tools link it for the map verification commands;
only `map_engine` uses `graphics_engine`. Each stage of the
[workspace restructure](/documentation/restructure/program_plan.md) moves modules from here into a
crate under `crates/` and switches the callers to it; when both folders are empty, `legacy/` is
deleted.

## Getting started

```bash
cargo test -p map_engine        # the map engine's unit and integration tests
cargo test -p graphics_engine   # the renderer's unit tests
cargo xtask verify engine-layers  # the layer rules between the engines and their callers
```

## Boundaries

- Depends on: `contracts/` (the mission and terrain schemas the map engine reads) and external
  crates only; nothing under `crates/`, `apps/` or `tools/`.
- Used by: `apps/frontend/`, `apps/api/` and `tools/developer_tools/`; no crate under `crates/`
  depends on a crate in `legacy/`, which `cargo xtask verify strangler` and
  `cargo xtask verify crate-tiers` hold.
- Rules: the engine layer rules (`cargo xtask verify engine-layers`) keep `graphics_engine` free of
  map concepts and the app's pages away from it; new code goes into a crate under `crates/`, never
  here.

## Related documentation

- [Engine documentation](/documentation/legacy/README.md) — the two engines' deeper documents.
- [Workspace layout](/documentation/architecture/workspace_layout.md) — every top-level folder and
  workspace member.
- [Target file tree](/documentation/restructure/target_file_tree.md) — where each engine module
  goes.
