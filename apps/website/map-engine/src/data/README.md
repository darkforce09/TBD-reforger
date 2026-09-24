# Mission data

The map engine's [mission](/documentation_v2/glossary.md#mission) data: the mission domain that
the browser and the server share, and the Yjs CRDT document the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits. Neither half knows the
map, the GPU or the page it runs in.

## Contents

```text
apps/website/map-engine/src/data/
├── mod.rs     the module tree
├── scenario/  the mission domain: its shapes, compiler, checks, authored blocks and mortar solver
└── store/     the mission's Yjs document: CRDT arrays, undo groups, row projections, operations
```

## How it works

```text
crate::editing (the Mission Creator's tools and undo drive)
   │  edit operations
   ▼
store: MissionDocCore, a yrs document
   │  small_maps_json and slots_json, the document as by-id JSON
   ▼
scenario: compile ─▶ editor payload ─▶ validate, wire_safety ─▶ flatten ─▶ compiled document
```

Each half sits behind a crate feature. `scenario`, the crate's default, brings `serde`,
`serde_json` and `thiserror`, and is all the [API](/documentation_v2/glossary.md#api) links;
`store` adds the `yrs` CRDT crate on top of `scenario`, and the single-page app reaches it through
the `editing` feature. The store reads the mission domain (terrain bounds, the tactical-graphics
kinds and point limits), and the mission domain reads the store only in two store-gated tests.
Everything here takes its inputs explicitly: the document operations take values and callbacks from
the host, and the host installs the clock the undo groups read (`install_wasm_now`), so no browser
binding enters this tree.

## Public surface

- `scenario`: the aliases `data::scenario` publishes (`orbat`, `compile`, `flatten`, `kit`,
  `validate`, `wire_safety`, the authored blocks, `slot_line`, `ballistics`) and
  `COMPILER_PACKAGE_VERSION`, for the API, the Mission Creator and `crate::editing`.
- `store`: `MissionDocCore` with its row and patch types, the faction library placement
  (`apply_faction_library`), `place_character_under_side`, the
  [slot](/documentation_v2/glossary.md#slot) projection and the undo constants and clock, for
  `crate::editing` and the Mission Creator.

## Boundaries

- Depends on: `serde`, `serde_json` and `thiserror`; `yrs` for `store`;
  `contracts_v2/rules/kit-aliases.json`, embedded at build time; nothing else of the crate.
- Used by: `crate::editing`; the API's [missions](/documentation_v2/glossary.md#missions) and
  [operations](/documentation_v2/glossary.md#operations) domains under
  `apps/website/api_v2/src/`; the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`,
  the mission library in `apps/website/frontend/src/v2/pages/mission_hub/library/` and the DTOs of
  `apps/website/frontend/src/v2/core/api/dto/`; the `engine-layers` and `editor-orbat-coherency`
  gates of `tools_v2/xtask/`, which scan this tree.
- Rules:
  - the tree imports none of the crate's `camera`, `diagnostics`, `doll`, `frame`, `io`,
    `overlay`, `spatial`, `streaming` and `world` modules nor the graphics engine, and the static
    world never imports it back (rule 7 of `cargo xtask verify engine-layers`);
  - `scenario`'s code never imports `store` (rule 4 of the same gate; two store-gated tests are
    pinned exceptions), so the API's build stays free of `yrs`.
