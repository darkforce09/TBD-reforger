# T-159.28 — map-asset host (terrain hillshade MVP) — verify log

**Slice:** T-159.28 (finish program stream 5, P0 critical-path). Sub-commit `.28a`.
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · branch `t-159-leptos-ui` · **base** `d1991c27` (T-159.26).
**Executor:** claude-code (solo session). **Result: PASS.**

## Goal

The Leptos editor rendered only a bare grid — no terrain. Build a Rust map-asset host that fetches
the terrain DEM and paints a hillshade relief lane, so the editor shows terrain (the P0 "editor
renders the map" value).

## The key finding (why this is small)

The heavy lifting is **already Rust** in `map-engine-core` — the React TS glue only fetched bytes and
called into it. Two exploration passes confirmed the exact API:
- `dem::png_decode::decode_png_to_meters(bytes, min_m, max_m) → DecodedDem{meters, width, height}`
  (16-bit DEM PNG → `Vec<f32>` meters; behind the `png` feature).
- `dem::hillshade::build_hillshade_image(&meters, w, h) → Hillshade{data: RGBA, w, h}` (Horn
  hillshade, self-downsampled to ≤1024 px edge, row-flipped north-up).
- The engine has **no** `set_dem`/`set_hillshade` — hillshade rides the texture lane: `tex_layer_begin(role=1,
  …, mode=3)` → `tex_layer_write_rgba(role=1, …)` → `tex_layer_commit(role=1, opacity, visible)`.

So the host is a thin **fetch + decode + upload** layer, ~150 LOC, all against existing Rust.

## What shipped

- **`world_assets.rs`** (new, wasm-only) — `load_hillshade(engine, terrain)`: fetch
  `/map-assets/<terrain>/manifest.json` → `worldBounds` + `dem.{path,heightRangeMinM,MaxM}` → fetch
  the DEM PNG bytes (gloo-net binary) → `decode_png_to_meters` → `build_hillshade_image` →
  `tex_layer_begin(1, bounds, w, h, 1, 3)` + `tex_layer_write_rgba(1, 0, 0, 0, w, h, &rgba)` +
  `tex_layer_commit(1, 0.4, true)`. A `window.__mapAssets` bridge exposes the uploaded dims for the
  GPU gate. Any fetch/decode failure returns early — the editor stays on the bare grid, never blocks.
- **`mission_editor.rs`** — `spawn_local(load_hillshade(engine, terrain))` after the engine is `Some`,
  off the render path; `terrain` read from the doc meta (seed/hydrate set it; default everon).
- **`Cargo.toml`** — `map-engine-core` gains the `png` feature (the DEM decoder).
- **`serve.mjs`** — opt-in `mapAssetsDir` serves `/map-assets/*` from `packages/map-assets` (the
  Trunk/prod passthrough equivalent) so the gate can fetch the committed DEM.

## Gate safety

The hillshade load fetches `/map-assets/…` which **404s on the standard gate route** (`serve.mjs`
without `mapAssetsDir`), so `load_hillshade` returns early → no lane → the 13 existing editor smokes
are untouched.

## Deferred (folded forward, per the audit's "large" call)

- **Unified satellite basemap** (`everon-sat.tbd-sat`, 152.7 MB, its own TBDS + mipped-WebP format
  parsed host-side): needs a Rust TBDS parser + WebP decode (the engine takes decoded texels only).
- **World-object streaming** (315 chunks / 1.2 M instances): `map-engine-core::world`
  (`WorldResidency` — already Rust) is viewport-driven; the host wires `set_viewport` → fetch
  `chunks/{id}.json.gz` → `ingest_chunk_gz` → `upload_world_*`. Portable but ~800–1200 LOC.

Both are the "editor looks richer" layers on top of the P0 "editor shows terrain" value shipped here.

## Gates

| Gate | Result |
|------|--------|
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | clean |
| `cargo check -p website-leptos` (native) | ≤ baseline (stash-diff: zero new) |
| `cargo clippy … wasm32` | **12** = baseline (zero new) |
| `trunk build --release` | ✅ success |
| **`editor-hillshade-smoke`** (GPU, `?force=webgl`, /map-assets served) | **PASS** — `__mapAssets.hillshadeH/W > 0` after the 72 MB DEM fetch + Rust decode + `tex_layer` upload; no panics |
| **13 editor smokes** (standard gate route, no /map-assets) | **13/13 PASS** (hillshade 404s silently → unchanged) |

## Next

**T-159.27** — Arsenal + registry compat + Faction Manager (fills the Attributes Arsenal stub), then
**T-159.29** — cutover build-out (Trunk proxy already done in .24; add `make leptos*`, CI job,
backend SPA serve, oracle freeze). Satellite + world-object streaming ride a .28 follow-on.
