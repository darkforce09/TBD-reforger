# REPORT — T-938.5 · Viewsheds sliced across frames with caps

> Transcribed by the command centre from the slice agent's returned report: the agent harness
> refused its `Write` to this path ("Subagents should return findings as text, not write report
> files"). Content is the agent's, verbatim in substance. Independently re-verified by the command
> centre: branch clean, two commits, `files_outside_owns []`, no tolerance loosened, caps as
> decided. Merged to main as `e914301d12e28f86c6a6b982f4c2ef8798d0fa60`.

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.5
slice/T-938.5
```

## defect_verified
Measured before any change at branch point `c73d39668`. Every run showed `Compiling map-engine-core`,
so none is a replayed cached verdict (T-596).

Terrain viewshed at the shipped 2000 m / 8 m default:
```
T-644 viewshed perf: 56.82 ms | 501×501 raster (251001 cells) | visible 197253 hidden 53748 unknown 0
```
Building wash at the shipped 25 m default, and the absent radius clamp:
```
radius 25 m   -> 2 levels, 200x200 cells each   (80000 total)   in   56.5 ms BLOCKING (accepted)
radius 1000 m -> 2 levels, 2048x2048 cells each (8388608 total) in 5387.4 ms BLOCKING (accepted)
```
56.8 ms and 56.5 ms inside one pointer-up handler — three-plus dropped frames each. The clamp was
genuinely absent: `dem/sample.rs:420-424` only substituted the default for a non-finite or
non-positive radius, and `grid_rect` coarsened the *cell* to `MAX_WASH_DIM`, which bounds the raster
but not the work — 2048² rays per level however large the disc.

## changes
**`crates/map-engine-core/src/dem/sample.rs`** — `ViewshedCapRefused{cap,limit,measured}` + `Display`
(one refusal type for both subsystems); `MAX_VIEWSHED_CELLS = 300_000`; `ViewshedGrid` /
`viewshed_grid()` / `march_schedule()` / `March::sample()` extracted so `compute_viewshed` and the
new job share the lattice and the per-sample rule and differ **only in how they iterate**;
`ViewshedJob` with `cursor` + `generation` + `step(elev_at, budget_ms, now)` (the `ObjectPass`
idiom), plus `raster` / `into_raster` / `progress` / `cancel`. **Checkpoints at a whole RAY** — the
march is not pure over its index (`max_angle` is a running horizon, `Visible` is sticky across rays),
and a ray boundary needs no persisted horizon while preserving ray order. `step` always marches at
least one ray, so `budget_ms == 0.0` is finest slicing, not a spin. `compute_viewshed` refuses
over-cap with an empty (`cols == 0`) raster.

**`crates/map-engine-core/src/building_viewshed.rs`** — `MAX_WASH_RADIUS_M = 400.0` +
`wash_cap_check()`. **The cap is in `wash_band` / `WashJob::new`, never `grid_rect`** — `grid_rect`
still coarsens and `oversized_radius_coarsens_the_cell_to_the_cap` is untouched and green.
`LevelWash::cell_verdict()` + `wash_shell()` are the shared rule and geometry; `WashJob` carries
`cursor` / `generation` / `step` over a pre-sized raster written by index (this loop *is* pure over
its index, so any cell is a legal checkpoint), `WASH_BATCH_CELLS = 256` between clock reads (≈0.26 ms
at the crate's measured 1.03 µs/ray). Over-cap is refused on every surface: `wash_band` returns a 0×0
wash and **casts no ray** (asserted with a counting blocker), `level_wash*` inherit it,
`WashJob::new` returns the message-bearing `Err`.

**`apps/website/frontend/src/editor/tools/viewshed_scheduler.rs`** (new, 448 lines) —
`VIEWSHED_BUDGET_MS = 4.0`; `ViewshedTool{Terrain,BuildingWash}` keys the slots so there is one
active job per tool and a submit cancels only its own. `submit_terrain` builds the job from the same
manifest and params `compute_viewshed_for` uses, bumps the generation, drops the previous job before
any work, runs one budgeted batch, publishes and returns the snapshot; on wasm **this module's own
self-rescheduling rAF closure** finishes it, natively `submit_terrain` drains it so a native caller
still gets the synchronous result. Two cancel paths, neither touching a sibling file: a newer submit
replaces the slot, and the pump drops a job whose observer no longer matches the session
`ViewshedState` — which is exactly Esc (handled in sibling-owned `canvas/commands.rs`), arriving for
free and stopping a dismissed wash being resurrected. On completion the pump republishes the final
raster and calls `los_world_wasm::start_object_wash()` so the merged upload `viewport.rs` already
pumps is over the finished terrain, not the first batch. The wash lane
(`submit_wash` / `step_wash` / `take_wash`) is stepped by its owner — a `blocked` closure borrowing
the building's BVH cannot be parked in a `'static` thread-local. `last_refusal()` surfaces the cap
message.

**`apps/website/frontend/src/editor/tools/los_tool.rs`** (+13 net) — `place_viewshed` submits and
encodes, nothing more; the inline `ViewshedState` write is extracted **verbatim** as
`publish_viewshed_raster` so the scheduler can repeat it on completion; `everon_manifest` becomes
`pub(crate)`. `compute_viewshed_for` untouched. No banned Class-R token introduced; both RGBA
self-pins intact.

**`.../tools/mod.rs`** — `pub mod viewshed_scheduler;`.

**Tests added (11):** `sliced_viewshed_is_bit_identical_to_the_sync_path`,
`sliced_viewshed_matches_the_off_coverage_opening`, `viewshed_job_cancels_mid_disc`,
`over_cap_viewshed_is_refused_with_a_message`, `sliced_wash_is_bit_identical_to_the_sync_path`,
`wash_job_cancels_mid_disc`, `over_cap_wash_radius_is_refused_with_a_message`,
`one_active_job_per_tool_and_a_submit_cancels_its_own`,
`a_scheduled_wash_finishes_equal_to_the_sync_path`, `an_over_cap_submit_is_refused_with_a_message`,
`the_terrain_lane_declines_without_a_sampler`. Both equality fixtures first assert the fixture yields
**all three** `Visibility` classes (the terrain one runs off the coverage box, through a sampler hole
and past a ridge), and compare at three batch sizes under a monotone fake clock — no wall-clock
deadline anywhere.

## perturbation (RED verbatim)
Broke both iterators to skip their last row/ray.

**The first attempt was ineffective and that is worth recording.** Perturbing only `WashJob::step`'s
loop bound and `done` check left `end = (cursor + WASH_BATCH_CELLS).min(self.total)` clamping to the
*unperturbed* total, so the last batch overshot the perturbed bound and decided the final row anyway
— the wash test stayed **green** while the terrain test went red. The perturbation was corrected to
cap `end` as well. A green half of a perturbation loop is precisely the failure mode the habit
exists to catch.

```
running 4 tests
test dem::hillshade::tests::constants_are_bit_identical_to_js ... ok
test dem::sample::tests::sliced_viewshed_is_bit_identical_to_the_sync_path ... FAILED
test building_viewshed::tests::sliced_wash_is_bit_identical_to_the_sync_path ... FAILED
test geometry::tbdd::tests::everon_tiles_decode_bit_identically_to_the_old_loop ... ok

failures:

---- dem::sample::tests::sliced_viewshed_is_bit_identical_to_the_sync_path stdout ----

thread 'dem::sample::tests::sliced_viewshed_is_bit_identical_to_the_sync_path' (694748) panicked at crates/map-engine-core/src/dem/sample.rs:1595:13:
assertion `left == right` failed: budget 0: every ray marched
  left: 471
 right: 472

---- building_viewshed::tests::sliced_wash_is_bit_identical_to_the_sync_path stdout ----

thread 'building_viewshed::tests::sliced_wash_is_bit_identical_to_the_sync_path' (694746) panicked at crates/map-engine-core/src/building_viewshed_tests.rs:249:9:
assertion `left == right` failed: budget 0: the sliced wash must equal the synchronous one (16 batches)
  left: LevelWash { level_index: 1, eye_y: 4.0, obs: [1.5, 1.4, 1.5], radius_m: 8.0, min_x: -6.5, min_z: -6.5, max_x: 9.5, max_z: 9.5, cell_m: 0.25, cols: 64, rows: 64, cells: [Unknown, Unknown, … ] }
 right: LevelWash { … }

failures:
    building_viewshed::tests::sliced_wash_is_bit_identical_to_the_sync_path
    dem::sample::tests::sliced_viewshed_is_bit_identical_to_the_sync_path

test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 1065 filtered out; finished in 0.17s
```
Restored with `git checkout --` (never `git stash`) **and `touch`ed** both files; `Compiling
map-engine-core` appeared again, so the green is not a stale artifact:
```
test dem::sample::tests::sliced_viewshed_is_bit_identical_to_the_sync_path ... ok
test building_viewshed::tests::sliced_wash_is_bit_identical_to_the_sync_path ... ok
test result: FAILED. 1066 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 4.64s
```
The one failure is `dem::peaks::tests::everon_peaks_max_above_350` —
`decode: Decode("Invalid PNG signature.")` because `everon-dem-16bit.png` is a 133-byte unfetched LFS
pointer in a worktree. Pre-existing and environmental.

`cargo test -p website-frontend`: **1348 passed, 0 failed** — including `t644_viewshed_wiring` (the
pin requiring `place_viewshed(` to precede `.viewshed_upload(` in the untouched `gestures.rs`),
`t090_12_world_los_wiring`, `los_world_tests`' 58 `assert_eq!`, and `building_viewer`'s per-cell
`Visibility` + census-band + 1e-9 rect goldens. Clippy clean on both crates.

## gate_verdict_tail
```
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
  T-278 catalogue drift    PASS
  … 14 more PASS …
  no-python (T-620)        PASS

  gate verdict PASS @ 5749a92eae47 recorded: .ai/artifacts/verdicts/T-938.5.json
SLICE GATE: PASS
```
21 PASS, 0 FAIL, 0 SKIP. (Waited ~30 s on the gate lock held by sibling T-938.6 — serialisation, not
a hang. This is the first wave where the gate's new `test (frontend, changed)` step ran, T-946.64.)

## files_outside_owns
`[]` — confirmed independently by the command centre: the diff lists exactly the six owned paths.

## found_not_fixed
1. **The wash lane has no production consumer.** `pages/debug/building_viewer.rs` (**unowned**) still
   calls `level_wash` / `level_wash_compound` synchronously at `:1611-1612`. It is now radius-capped,
   but nothing routes it through `WashJob` / `submit_wash`. Wiring is mechanical: swap the
   `level_wash*` call for `submit_wash(level_index, eye_y, obs, &p)`, call `step_wash(&blocked)` from
   that page's existing `start_raf` (`:2134`) with the same closure `level_wash*` injects, and
   `take_wash()` into the `wash` signal on done. Not attempted — outside owns.
2. **No frontend surface displays `last_refusal()`.** Logged and readable; the natural home is a HUD
   cell, and the HUD suffix lives in `los_world_wasm::hud_suffix()` (unowned).
3. **`place_viewshed` drops the click-time observer Z** — `st.observer = Some((x, y, None))`
   overwrites the Z `gestures.rs` wrote via `ViewshedState::place` a moment earlier. Pre-existing,
   preserved byte-for-byte; flagged only because the extraction made it visible.
4. **`los_tool::compute_viewshed_for` now has no production caller.** Kept deliberately as the
   documented synchronous seam and the reference the sliced path is pinned against.

## deviations
1. **`dem/sample.rs` grew ~326 production lines (1185 → 1705)**, against "grow by call-site lines
   only". The ticket requirement is explicit that `dem/sample.rs` must *expose* the resumable
   iterator, and the only alternative — a new `dem/viewshed_job.rs` — is outside the owns list, which
   the brief forbids more strongly. This does not extend the allowlist: `dem/sample.rs` already
   carries a SIZE-3 row (`.coding-standards-allowlist.yaml:247`, expires 2026-11-13) and the gate's
   file-length step passed.
2. **`los_tool.rs` changed slightly more than `place_viewshed`'s body** — `+publish_viewshed_raster`
   (the state write extracted verbatim) and `everon_manifest` → `pub(crate)`. +13 net lines, no new
   logic. Without a writable seam the scheduler cannot publish a finished raster.
3. **The infallible surfaces refuse with an EMPTY raster, not a `Result`.** `compound_wash` /
   `wash_band` return `LevelWash` and `compound_wash` is consumed by the unowned
   `building_compound_tests.rs:649-693`; changing those signatures would require editing outside
   owns. Over-cap yields a `cols == rows == 0` wash (all `Unknown`, no ray cast) and the message
   comes from `wash_cap_check` / `WashJob::new`.
4. **`viewshed_scheduler.rs` carries a module-level `#![allow(dead_code)]`** — the `los_tool.rs:51` /
   `ruler_tool.rs:49` precedent in the same directory. On the wasm build the test module is not
   compiled and the wash lane's consumer is the unowned `building_viewer.rs`. All those items are
   exercised by this module's native tests.
5. **The scheduler calls `los_world_wasm::start_object_wash()`** on completion — a call into that
   module, not an edit. Required: otherwise the object pass `gestures.rs` starts right after
   `place_viewshed` runs over the *first batch's* raster and its merged upload shows a partial
   terrain wash forever.
6. **No mid-job GPU upload.** The scheduler cannot reach `&mut RenderEngine`: `world_assets`'
   `RENDER_CTX` is private and exposes no engine accessor, and `world_assets/mod.rs` belongs to
   T-938.6. The first batch is uploaded by the existing `gestures.rs` call site and the finished disc
   by the existing `tick_object_wash` merge, so nothing is left undrawn — but a per-batch progressive
   paint would need an engine seam in an unowned file. Flagged rather than taken.

## commits
```
5749a92ea T-938.5: the viewshed scheduler, and place_viewshed submits to it
b7af42b7a T-938.5: resumable, capped viewshed jobs in map-engine-core
```
Merged to main as `e914301d12e28f86c6a6b982f4c2ef8798d0fa60`.

## manual_checklist
Needs a live editor (`cargo xtask mk leptos` + API on `:8080`), dev-login, LoS tool → Viewshed:
1. Place one viewshed — the click returns immediately, a partial wash appears on the first frame and
   fills over the next handful; the HUD's `rf <ms>` should show no ~57 ms spike at the click.
2. Place two in quick succession — the first job is dropped unfinished; only the second disc
   completes and no stale wash from the first observer appears.
3. Press Esc mid-fill — the lane clears and the in-flight job does not republish afterwards.
4. Confirm the terrain default is unchanged — the disc still spans 2000 m (501×501 at 8 m cells).
5. Cap refusal is not reachable from the shipped UI (no radius control), so that path is
   test-covered only.
