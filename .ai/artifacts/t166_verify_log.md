# T-166 verify log — Leptos full map host (W1–W5)

**Date:** 2026-07-17  
**Ticket:** T-166  
**Scope:** Host wiring only (Leptos `world_assets/` + gate `serve` Range/206 + `smoke_fullmap`). Engine kernels untouched.

## Pin table (Class-R)

| Pin | Value | Evidence |
|-----|-------|----------|
| Sat bytes | 152713114 | `manifest.json` `tiles.satellite.unified.bytes` == disk `everon-sat.tbd-sat` |
| Roads | 888 | `road_segments === 888` in smoke |
| Landcover | 36 | `landcover_polygons === 36` |
| Default zoom | −2.0 | `mission_editor` `INITIAL_ZOOM` |
| Tree glyphs @ −2 | 0 | `A_trees_off` |
| Tree glyphs @ z=2 probe | >0 | `A_trees_on` (heatmap-safe zoom; island-center z=0 clears glyphs by INSTANCE_BUDGET) |
| Chunk census | 315 | store / disk `objects/chunks` |
| Density bins | 625 | disk `objects/density` |

## CDP — `gate smoke fullmap`

Navigate: `/missions/smoke/edit?force=webgl&sat=preview` with `map_assets_dir=packages/map-assets`.

```json
{
  "gate": "editor-fullmap-smoke",
  "path": "/missions/smoke/edit?force=webgl&sat=preview",
  "pins": {
    "sat_full_bytes": 152713114,
    "roads": 888,
    "landcover": 36,
    "default_zoom": -2.0,
    "tree_glyph_min_zoom": 0.0
  },
  "checks": {
    "bridgeSettled": true,
    "A_hs": true,
    "A_sat": true,
    "A_roads": true,
    "A_lc": true,
    "A_sea": true,
    "A_cont": true,
    "A_bld": true,
    "A_forest": true,
    "A_atlas": true,
    "A_trees_off": true,
    "A_trees_probe": true,
    "A_trees_on": true,
    "A_sat_bytes": true,
    "A_panic": true
  },
  "panics": [],
  "pass": true
}
```

## Must-ship matrix

| Item | Status |
|------|--------|
| W1 TBDS sat + hillshade + `set_grid` | PASS |
| W2/W3 WorldStore + Residency + buildings | PASS |
| W4 DEM sea/contours + landcover 36 + roads 888 + TBDD forest fill | PASS |
| W5 glyph atlas + icon lanes | PASS |
| Gate serve Range → 206 (no full sat body under `sat=preview`) | PASS (`A_sat_bytes`) |
| `smoke_fullmap` in `EDITOR_SUITE` | PASS |
| Forest outline / tree glyphs not required at z=−2 | PASS |

## Implementation notes

- Module: `apps/website-leptos/src/world_assets/{mod,bridge,fetch,tbd_sat,satellite,dem_vectors,world_host,forest_mass}.rs`
- Soft HTTP failures must not `note_undelivered` (empty stub forever).
- Camera-settle drains up to 24 pending chunks and prioritizes newest fetches — the 4ms/frame budget otherwise starved the z≥0 tree probe behind the z=−2 backlog.
- Tree zoom probe uses `window.__editorCamSet(6400,6400,2)` (not CDP `mouseWheel`).

## Operator polish (explicitly later — not blocking T-166 ship)

**Operator (2026-07-17):** *"forest is also a little bit fucked but that can be fixed in the future"* · *"Laggy start … the entire website is Laggy … that can be fixed In the future"*

Record for a future perf / forest-fidelity pass (not inventing a deferral of T-166 scope):

| Note | Detail |
|------|--------|
| Forest visual | Mass/fill looks wrong or incomplete in manual Mission Creator — tune later |
| Editor cold start | Slow first paint while sat + chunk residency stream |
| Site-wide lag | Whole Leptos SPA feels laggy on boot — platform perf pass later |
| Full satellite | Dev defaults to Range preview; `?sat=full` for the complete TBDS mip chain (152 MB GET) |

## Ship blocker — editor-suite `arsenal` wedge (diagnosed + fixed 2026-07-17)

`make leptos-gates` hung: `gate editor-suite` wedged on `smoke arsenal` right after `selfcheck`,
even though `gate smoke arsenal` and `gate smoke fullmap` passed alone. Full diagnosis:

- **Symptom:** a single `Runtime.evaluate` (the `__editorSelection.probe()` centring call) never
  returned — the page main thread sat **idle at low CPU** (not a busy loop, not a throw: the probe
  is wrapped in `try/catch`), so the CDP call blocked indefinitely.
- **Bisect:** arsenal passes alone *intermittently*, then fails **3/3 alone** once the machine is
  under memory pressure (dev desktop: ~3 GB free). `fullmap` — strictly heavier (real world + sat
  + DEM) — **passes**, because it navigates with `?force=webgl`. Running `arsenal` itself with
  `?force=webgl` **passes**.
- **Root cause:** the software **WebGPU/lavapipe** backend. Its rAF render loop intermittently
  stalls the page main thread long enough (headless, under memory pressure) that the next
  `Runtime.evaluate` starves and never returns. `arsenal` is simply the **first default-backend
  smoke after WebGL2 `selfcheck`** in the glob order, so the suite always died there first —
  every other default-backend smoke (`attributes`, `cur`, `doc`, …) was equally exposed but never
  reached. Not the app, not the profile, not CPU orphans, not `/dev/shm` — all ruled out by
  experiment.

**Fix (`tools/tbd-tools`):**
- `smokes.rs` `EDIT_PATH` now pins **`force=webgl`** (SwiftShader WebGL2) suite-wide. These smokes
  exercise doc / UI / interaction, not the GPU backend; the GPU-byte gates (`selfcheck`,
  `fullmap`, `hillshade`, `marquee-drag`, `undo`) already forced WebGL2. `force_webgl()` made
  idempotent.
- `cdp.rs` hardening (defense-in-depth, operator-sanctioned): a **130 s bounded wait on
  `Page::send`** so any future wedge fails the smoke loudly instead of hanging the suite forever;
  a **per-launch `--user-data-dir`** (no cross-smoke OPFS/IndexedDB bleed); chrome spawned in its
  **own process group** and shutdown **reaps the whole tree** (SIGTERM → wait → SIGKILL) + removes
  the profile dir, so debug ports and renderer children free deterministically between smokes.

**Final evidence (2026-07-17, live API on :8080 + `db-up`):**

```
gate editor-suite → SUITE_EXIT=0 — 17/17 smokes, 0 fail
  selfcheck arsenal attributes cur doc editor fullmap hillshade hydrate
  keyboard-settings marquee-drag outliner-palette pan persist save-export select undo
gate smoke fullmap → pass:true (Class-R matrix unchanged; see §CDP above)
make leptos-gates → exit 0 (editor-suite + v-suite verify, fresh release dist)
make ci-local     → exit 0
make verify-no-node → exit 0
```

## Follow-on (other tickets — not T-166 deferrals)

T-152.4 fences/piers · T-152.5 apron · T-152.7–.9 labels · T-151.8 heatmap · W6 slot polish.
