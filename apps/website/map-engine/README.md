# Map engine

The `website-map-engine` crate: everything between the platform's map data and the pixels, and
the [mission](/documentation_v2/glossary/g_to_m.md#mission) domain the
[API](/documentation_v2/glossary/a_to_f.md#api) and the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) share. It holds the mission
compiler and CRDT document, the headless editing tools, the static world streamed from a terrain's
assets, spatial queries and line of sight, the map's lanes and symbology, the cameras, and the
render engine that hands frame packets to `website-graphics-engine`. It depends on no UI
framework; the frontend supplies the canvas, the preference readers and the page around them.

## Contents

```text
apps/website/map-engine/
├── Cargo.toml  the `website-map-engine` package: its feature tiers, dependencies and wasm32 crates
├── src/        the library `website_map_engine`: eleven modules, each behind its feature
└── tests/      integration suites: deck.gl camera parity and headless document operations
```

## How it works

The dependency arrow runs one way: the frontend uses this crate, and this crate uses
`website-graphics-engine`, the pure renderer, which never names a map concept. The crate speaks
the renderer's frame vocabulary through `src/frame/mod.rs` alone: it builds its pipelines and
atlases with the renderer's constructors and hands it draw batches and frame packets.
`RenderEngine`, which owns the device, the surface and every lane, lives here in `src/frame/`.

A consumer takes only the tier it needs, because each module compiles under the feature it
belongs to (the table in the [source README](/apps/website/map-engine/src/README.md)):

```text
render ──▶ streaming ──▶ io ──▶ world ──▶ bvh
   │           │         │        └────▶ png, website-graphics-engine
   │           │         └─────▶ scenario, rkyv
   │           └──────▶ flate2
   └──────────▶ website-graphics-engine
editing ──▶ store ──▶ scenario ──▶ serde, serde_json, thiserror   (store adds yrs)
editing ──▶ world, streaming
```

`scenario`, the default and the tier the API links, is the headless mission compiler and
validator: it pulls no graphics crate, PNG decoder, rkyv or flate2. `store` adds the Yjs document
(`yrs`). `world` adds the static world, spatial queries and the overlay, and links the renderer;
`io` adds the rkyv archive formats; `streaming` adds the loader and scheduler stack (flate2);
`render` adds the GPU frame path, the diagnostics and the doll. `editing` joins the document to
the streamed world, so it takes `streaming` too. Browser code (canvas, fetch, image decoding,
timers, the console) compiles only for wasm32; native builds keep the geometry, codecs and state
machines and test them without a browser.

## Getting started

Run these from the repository root:

```bash
cargo xtask ci lfs-dem           # pull the Everon elevation model from Git LFS; one test decodes it
cargo xtask mk wasm-ci           # fmt, clippy (all features, wasm32) and tests of both engines
cargo xtask verify engine-layers # the layer rules between the engine crates and inside this one
```

`cargo test -p website-map-engine --all-features` runs this crate's tests alone. Without
`--all-features` the tripwire test `map_engine_tests_require_all_features` fails, since the
default tier compiles only a fraction of the crate. The tests read the Everon terrain under
`assets_v2/terrains/everon/` from disk. The crate has no binary of its own: the Mission Creator
runs it in the browser, inside the single-page app that `cargo xtask mk leptos` builds and serves,
and `cargo xtask mk leptos-gates` runs the editor gate, whose `selfcheck` smoke calls the render
engine's readback checks.

## Configuration

Cargo features, in `Cargo.toml`:

| Feature | Turns on | Taken by |
|---|---|---|
| `scenario` (default) | `serde`, `serde_json`, `thiserror`; `data::scenario` | the API |
| `store` | `scenario`, `yrs`; `data::store` | the frontend |
| `bvh` | no crate; gates `spatial::bvh`, which also needs `world` | implied by `world`; `tools_v2/developer-tools` names it too |
| `world` | `bvh`, `png`, `website-graphics-engine`; `world`, `spatial`, `overlay`, `frame` | the frontend, `tools_v2/developer-tools` |
| `io` | `world`, `scenario`, rkyv and float round trips in `serde_json`; `io` and `streaming`'s bridge | the frontend, `tools_v2/developer-tools` |
| `streaming` | `io`, `flate2`; the loaders, scheduler, buffers and memory ledger | the frontend on wasm32 and in its tests, `tools_v2/developer-tools` |
| `render` | `streaming`, `website-graphics-engine`; the GPU frame path, `diagnostics`, `doll` | the frontend on wasm32 |
| `editing` | `store`, `world`, `streaming`; `editing` | the frontend |

In the browser the crate also reads the page's query string: `memBudgetMb` sets the streaming
memory budget in MiB (or `window.__memBudgetMb`; default 1536), `sat=preview` loads the satellite
imagery by range requests alone and never fetches the whole bundle, and `t9382=1` (or
`window.__t9382Log`) logs chunk allocation. At build time `data::scenario` embeds
`contracts_v2/rules/kit-aliases.json`.

## Public surface

- The library `website_map_engine`: `data::scenario` for the API's missions and operations
  domains; `data`, `editing`, `world`, `streaming`, `spatial`, `overlay`, `frame`, `camera` and
  `doll` for the frontend; `world`, `io`, `spatial`, `streaming` and `overlay` for the offline
  tools.
- The JavaScript-facing methods of `RenderEngine` and `DollEngine` (`#[wasm_bindgen]`), which the
  frontend calls from Rust and the editor gate reaches through the globals the Mission Creator
  publishes (`window.__selfChecks`, `window.__editorBench`, `window.__arsenalDoll`).

## Boundaries

- Depends on: `website-graphics-engine` (optional, from the `world` tier up); `serde`,
  `serde_json`, `thiserror`, `yrs`, `png`, `rkyv`, `flate2`, `bytemuck` and `earcutr`; on wasm32,
  `wgpu`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `gloo-net`, `futures` and
  `console_error_panic_hook`; `contracts_v2/rules/kit-aliases.json`; and at run time the terrain
  assets of `assets_v2/terrains/`, which the API serves under `/map-assets`.
- Used by:
  - the API (`apps/website/api_v2/Cargo.toml`), at the default `scenario` tier;
  - the frontend (`apps/website/frontend/Cargo.toml`): `world`, `io`, `store` and `editing` on
    every target, `render` and `streaming` on wasm32, and `streaming` for its native tests;
  - `tools_v2/developer-tools/Cargo.toml`: `world`, `streaming`, `io` and `bvh`, for the world
    export, the blueprint tooling and the map checks;
  - `tools_v2/xtask/`: the `wasm-ci` lane and the `engine-layers` gate.
- Rules:
  - the arrow is one-way: `apps/website/graphics-engine/` never imports this crate (rule 1 of
    `cargo xtask verify engine-layers`) and the frontend never imports the graphics engine (rule
    6); the rules inside the crate are listed in the source README;
  - the API's tier stays thin: `data/scenario/` names no module of a higher tier and no graphics
    crate (rule 4; the source README gives the gate's exact list);
  - the tests run with `--all-features` (`map_engine_tests_require_all_features`), and the camera
    matches deck.gl's orthographic viewport over 300 golden cases
    (`tests/deckgl_ortho_parity.rs`).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the terrain assets: what
  Git LFS holds, how `/map-assets` is served, and the LFS pulls.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the editor gate, whose
  `selfcheck` smoke calls the render engine's readback checks.
- [Map engine documentation](/documentation_v2/website/map-engine/README.md) — the crate's
  overview, map streaming, the editing layer and draft persistence.
- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the layer rules
  `cargo xtask verify engine-layers` enforces, and why.
