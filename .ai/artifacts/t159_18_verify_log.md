# T-159.18 — Select / LMB tools (pick foundation) — verify log

**Slice:** T-159.18 — LMB click-select on the seeded mission slots (frozen-viewport unproject + Rust
`PointIndex` pick). Foundation only; drag-move / marquee / Attributes deferred.
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · branch `t-159-leptos-ui` · base `425cfa1d` (T-159.17 tip).
**Executor:** claude-code.
**Result:** **PASS** — `cargo check --target wasm32-unknown-unknown` clean, `trunk build --release`
clean; the new `editor-select-smoke` passes (select / clear / Ctrl-toggle + Class-S self-check) and
all five prior editor smokes stay green.

## What shipped

- **`apps/website-leptos/src/select_tool.rs`** (new) — the pick foundation, plain Rust reusing
  `map-engine-core` (no `map-engine-wasm` shim, one wasm module — D5):
  - `frozen_camera(w, h, tx, ty, zoom) → OrthoCamera` — the **frozen viewport** (S2/X-05): a deck-parity
    `OrthoCamera::new(...)` + `set_bounds(0,0,12800,12800)`, copied from the engine's live view at
    pointer-down. The live `RenderEngine::unproject_xy` is **not** used (deleted at `engine.rs:1592`;
    a live unproject feedback-loops mid-pan).
  - `pick(cam, soa, px, py) → Option<String>` — unproject `(px,py)` against the frozen `cam`, then
    **box-nearest** over the doc SoA: world radius `r = |unproject(px+4,py).x − unproject(px,py).x|`,
    `PointIndex::pick_rect(±r box)`, argmin `dx²+dy²` → `soa.ids[handle]`. This matches React
    `slotSpatialIndex.pickNearest` exactly (nearest-in-**box**, via `pick_rect` — **not** the core's
    circular `PointIndex::pick_nearest`).
  - `apply_click(cur, hit, additive)` — React `useSelectTool` onPointerUp rules: hit+plain → replace
    `[id]`; hit+Ctrl/Cmd → toggle (empties to none); empty+plain → clear; empty+Ctrl → preserve.
  - `pick_selfcheck(soa) → bool` — **Class S**: the `PointIndex` box-nearest agrees with a brute-force
    box scan over the same points for every seed + a spread of ± offsets (compared by nearest
    **distance**, so a measure-zero equidistant tie is not a false negative).
  - `register_editor_selection(...)` — installs `window.__editorSelection` (peer of `__missionDoc` /
    `__missionPersist`; `js_sys::Object` of `.forget()`'d closures): `count()`, `ids()` (JSON array),
    `pick_selfcheck()`, and `probe()` — a test hook that centres seed 0 (`set_view`, zoom preserved)
    and returns `{id, hit:[px,py], empty:[px,py]}` (a guaranteed-clickable seed px + a guaranteed slot-free
    px) so the smoke is deterministic and independent of where the fixed seed lands.
- **`apps/website-leptos/src/mission_editor.rs`** — wired LMB into the existing gesture host:
  - New app-side state next to the doc host: `selection: Rc<RefCell<Vec<String>>>` (the S4 selection set)
    + `lmb: Rc<RefCell<Option<PendingLeft>>>` (the pending-left gesture); `register_editor_selection`
    called synchronously on mount.
  - **pointerdown** button 0 (previously unbound) now records a `PendingLeft` = press px (container-local)
    + a frozen camera from `engine.target_x()/target_y()/zoom()`. No pointer capture — a sub-threshold
    release is a click. Buttons 1/2 (pan) unchanged.
  - **pointerup** button 0: on a `< 4 px` release, `additive = ctrlKey||metaKey`, `pick` against the
    frozen camera + current `materialize()` SoA, `apply_click`, then `engine.set_selection(ids)` (GPU
    tint lane — a no-op until a slot atlas uploads, but the honest render wire). A `≥ 4 px` release is a
    drag (deferred): no selection change. Pan-end unchanged.
  - **pointercancel** split into its own closure (was shared with pointerup): it ends a pan **and** clears
    a pending LMB **without** a click.
- **`apps/website-leptos/src/main.rs`** — `#[cfg(target_arch = "wasm32")] mod select_tool;` (wasm32-only,
  gated like `mission_doc` / `yrs_persist`).
- **`.ai/artifacts/t159_gates/driver/smoke_select_editor.mjs`** (new) — the `editor-select-smoke` gate
  (serve 5304 / debugPort 9364). Trusted CDP mouse input drives the real Leptos pointer handlers.

## Frozen-viewport / X-05

Pick unprojects against an `OrthoCamera` **copied at pointer-down** from the engine's live `target/zoom`
+ the container CSS rect, never a live-engine unproject. The engine's `unproject_xy` was deleted under
audit X-05 (`engine.rs:1592`) precisely because a camera that keeps moving during a gesture would drift;
freezing at gesture start is the React `useSelectTool` pattern (`OrthoCameraJs` snapshot there,
`map-engine-core::OrthoCamera` here — the same deck 9.3.5 parity math in one wasm module). Selection is a
one-shot click this slice (no continuous drag), so a snapshot at press is self-consistent.

## Correctness note — Class S is behavioral + a live self-check, not a fixture

The pick parity is proven two ways, both over the **real seeded SoA in-browser**:
1. **Behavioral** — `probe()` centres seed 0 and returns its screen px; a real LMB click there must
   select exactly that seed's id (`selIds == [probeId]`). The empty-px click must clear; Ctrl-clicks must
   toggle. This exercises the whole pointerdown→frozen-camera→unproject→`PointIndex`→`apply_click` path.
2. **Structural (Class S, S3)** — `pick_selfcheck()` asserts the `PointIndex` box-nearest equals a
   brute-force box scan for every seed + offsets. Comparing by nearest **distance** (bit-exact f64) makes
   the assertion tie-robust. There is no native `cargo test` in this wasm-only crate, so the check runs in
   the wasm binary against live data — a stronger proof than a fixed fixture.

## Goldens (`/missions/smoke/edit`)

| Golden | Value |
|--------|-------|
| `pick_selfcheck()` | `true` |
| `probe().id` (seed 0 id) | `"s4"` |
| `probe().hit` (centred seed px) | `[720, 450]` (container centre @ 1440×900) |
| `probe().empty` (slot-free px) | `[34.29, 34.62]` |
| click seed → `ids()` / `count()` | `["s4"]` / `1` |
| click empty → `count()` | `0` |
| Ctrl-click seed → `count()` (toggle on) | `1` |
| Ctrl-click seed again → `count()` (toggle off) | `0` |
| doc `slot_count()` | `8` (SEED_N; unchanged) |
| dist wasm | `5,232,978 B` (T-159.17 ≈ 5,231,512 B; +~1,466 B) |

## Gate results (all exit 0)

**New — `editor-select-smoke`:**
```json
{
  "gate": "editor-select-smoke",
  "path": "/missions/smoke/edit",
  "ready": true,
  "selfcheck": true,
  "probeOk": true,
  "probeId": "s4",
  "hit": [720, 450],
  "empty": [34.285714285714285, 34.61538461538462],
  "selIds": ["s4"],
  "selCount": 1,
  "clrCount": 0,
  "onCount": 1,
  "offCount": 0,
  "t1_select": true,
  "t2_clear": true,
  "t3_toggleOn": true,
  "t4_toggleOff": true,
  "panics": [],
  "pass": true
}
```

**Regression (all `pass: true`, `panics: []`):**
- `editor-smoke` — pass.
- `editor-selfcheck` — pass (backend `webgl2`; GPU readback calibration + texture).
- `editor-pan-smoke` — pass (backend `webgpu`; RMB pan + mid-pan wheel rebase). LMB additions did not
  perturb the pan gesture.
- `editor-doc-smoke` — pass (`slotCount 8`, `seeded`, `roundtripOk`, `encodeStable`).
- `editor-persist-smoke` — pass (`coldOk`, `warmOk`, `digestMatch`, `slotCount 8`). Selection touches no
  doc mutator → the semantic slot digest is unchanged.

## Build
- `cargo check --target wasm32-unknown-unknown` — clean (no warnings).
- `trunk build --release` — clean (`Finished release ... success`; dist produced).

## Non-goals held (T-159.18)
- **Persist notify hook (S8):** this slice is **selection-only**, and selection is app-side state that
  does **not** mutate `MissionDocCore` → **no encode change**, so there is no persist debounce to fire and
  `doc_ver` stays as-is (the `editor-persist-smoke` digest is unchanged, confirming it). Per S8's clause,
  documented here: the `on_change`/bump hook lands with the first real doc mutator in a later slice.
- Deferred (S6/S7): entity drag-move commit, marquee rect + `pick_rect` selection, cluster drill,
  Attributes double-click, Eden chrome. No `GpuTimer`; no live-engine `unproject_xy` (X-05).
- Selection is held in the editor's leaked `Rc<RefCell>` idiom (consistent with engine/doc/pan_px, and
  leak-safe for the leaked bridge) rather than a reactive-owner `RwSignal` — a reactive signal would be
  disposed on route-leave and expose the leaked `window.__editorSelection` bridge to disposed reads. This
  satisfies S4's intent (selection is app-side, not in the Y.Doc); a reactive signal can wrap it once an
  inspector consumes selection.

## Next
Ready for Cursor → **T-159.19** (per the hub: save/export, or marquee/drag-move). Return: SHA + tag
`T-159.18` + this log.
