# REPORT — T-938.6 · Wasm memory budget guard with HUD readout

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.6
slice/T-938.6
```

`git status` was **clean on arrival** — no `t946_64_gate_probe` residue in `canvas/viewport.rs`,
nothing to `git checkout --`.

## defect_verified

Verified **on `main`'s own code**, twice, in a real browser. The dist was built from the unmodified
main worktree and proved unmodified before it was used:

```
strings dist_main/*_bg.wasm  | grep -cE 'memBudgetMb|__t9386|memory budget'  ->  0
strings dist_slice/*_bg.wasm | grep -cE 'memBudgetMb|__t9386|memory budget'  ->  3
```

Rig: `gate render-check --dir <dist> --map-assets packages/map-assets --inject-js <probe>
--no-freeze --path '/missions/aad0a12a-.../edit?force=webgl[&memBudgetMb=512]' --assert-js <probe>`
against a live API on `:8080` (rebuilt — `apps/website/api` had moved since the cached binary) and a
real `dev-login?role=admin` session in `localStorage['tbd-auth']`. The probe hooks
`WebAssembly.instantiateStreaming` / `instantiate` / `Instance` **before the SPA boots** and samples
the module's own exported `WebAssembly.Memory.buffer.byteLength`, so it reads the real wasm heap on a
tree that carries no instrumentation of its own.

### (a) There is no budget, and nothing can ask for one

| | `main` default | `main` `?memBudgetMb=512` |
|---|---|---|
| `satW x satH` | 6400 x 6400 | **6400 x 6400** |
| `satMips` | 13 | **13** |
| `window.__t9386` | `null` | `null` |
| debug HUD | `z -2.00 · c0 · glyph 0 · 42 FPS · rf 15.25ms (66 eq) · occl 133ch/1416bvh 115MB` | same shape — **no memory cell** |
| wasm heap peak | 613,875,712 B (585.44 MiB) | 645,332,992 B (615.44 MiB) |

A budget query param is *inert on main*: same island, same mip count, same absent readout.
Structurally, `grep -rnE "memory_budget|fn reserve\(|MemoryBudget|Decision::"` over
`apps/website/frontend/src/editor/` **exits 1 — no match anywhere**. Every existing `Budget` in
`world_assets` is `BootEvent::Budget`, the *network* progress bar (`dem_load.rs:104`, `fetch.rs:50`,
`satellite.rs:533`) — bytes on the wire, never bytes in the heap.

### (b) Each loader allocates independently, and the steps are visible

Wasm linear memory on `main`, default budget (bytes @ ms):

```
4,456,448 @332 -> 81,657,856 @569 -> 85,917,696 @609 -> 102,957,056 @657
-> 122,093,568 @702 -> 449,970,176 @1161 -> 613,875,712 @5417
```

The two steps that matter are uncoordinated and self-sized:

* **+327,876,608 B @ ~1.2 s** — the terrain lane. That is `2 x 163,840,000` to the byte: the DEM's
  6400^2 `Vec<f32>` of metres and the 6400^2 RGBA hillshade built from it, plus the 71,911,548 B PNG
  alive across the decode.
* **+163,905,536 B @ ~5.4 s** — the satellite chain from level 1 down.

Neither consulted the other, or anything else. On this rig `maxTextureDimension2D` is 8192, so the
GPU limit alone picks level 1 (248.53 MiB). **A 12800-capable GPU takes level 0 instead — 978.97 MiB
resident** (873,813,260 B of RGBA + 152,710,470 B of bodies), because `load_unified_full` decodes
every tile it will upload into one `Vec` *before* it takes the engine borrow. On wasm32 the failure
mode for an allocation that cannot be served is not an error a caller can handle: the alloc-error
handler calls `abort()` and the instance traps.

**What I did not do:** I did not force a real allocation abort. Doing so needs the wasm heap driven
to its 4 GiB ceiling, which would commit that much host RAM, and I judged that not worth the machine.
The tree carries its own record of the failure shape at `canvas/viewport.rs:67-75` — an observed
`createBuffer size too large` -> wasm `unreachable`, whose *second* panic buried the first. I cite
that as an in-tree record, not as a measurement of mine.

## changes

Four files, all in owns; **nothing outside owns was touched**.

**`world_assets/memory_budget.rs` (new, 878 lines incl. 12 tests).** A `thread_local` `Ledger`: byte
budget (default `DEFAULT_BUDGET_MB = 1536`, overridden per boot by `?memBudgetMb=NNN` via
`UrlSearchParams` — a value, not a substring flag — else `window.__memBudgetMb`), per-asset
`held`/`peak`/`growth` over a closed `Asset` enum, and the gate:

```rust
pub fn decide(&self, bytes: u64) -> Decision {
    if self.held_total().saturating_add(bytes) <= self.budget { Decision::Ok }
    else if bytes <= self.budget { Decision::Degrade }
    else { Decision::Refuse }
}
```

`Degrade` ("ask smaller") and `Refuse` ("larger than the whole budget, no release helps") are kept
apart because they call for opposite responses. `Ledger::reserve` records **only** on `Ok` — a
refused request that recorded bytes would refuse the next caller from a fiction. `floor_for_budget`
is pure and walks the mip ladder one level per rejection; `claim_satellite_floor` does the walk and
the reservation in one borrow so they cannot drift. `hud_suffix` formats the readout; `publish`
mirrors the ledger to `window.__t9386`.

**Two currencies, deliberately never summed.** `held`/`peak` are *declared* bytes (computed from the
asset) and are the only figures the budget arithmetic reads. `growth` is *measured* wasm linear
memory across a phase — the only handle on the four loaders outside this slice's owns — and is a
documented **lower bound**, since the heap never shrinks. Adding them would double-count.

**`satellite.rs`.** The budget sits in a **separate statement after `let base = base as usize;`**,
never folded into the `pick_base_level_for_limit(&index, limit.map(|l| l.device))` line (t629 pin).
It builds `LevelBytes` from the index — exact figures, every tile's `length` is in there — calls
`claim_satellite_floor`, warns once per rejected rung with the level, its cost and the decision, then
rebinds `base`. `mip_count`, `base_mip`, the `.skip(base)` upload plan and `report_chosen_level` all
follow the raised floor for free. `report_chosen_level` now also names the budget as a cause, because
with a budget in the path a level-above-0 basemap has two explanations that call for opposite
responses, and naming only the GPU would make the other unfalsifiable.

**`world_assets/mod.rs`.** `mod memory_budget;` + one narrow re-export (`memory_hud_suffix`).
`bootstrap` **forecasts the terrain lane from the manifest before the DEM/satellite join** — they
race and the satellite decides first (its index is two small Range requests; the DEM's is a 71.9 MB
body), so a budget that only knew about assets already loaded would hand the ceiling to whoever asked
first. `DemInfo` gained optional `widthPx`/`heightPx` (optional on purpose: `ManifestDem` is parsed
strictly, and a required field would cost a terrain its whole basemap over a missing forecast).
`load_dem_and_hillshade` then replaces the forecast with the figures actually allocated and releases
the hillshade after upload. The four unowned loaders are bracketed with `heap_mark`/`observe_since`.

**`canvas/viewport.rs`.** One extra `{}` on the single `debug_hud.set(format!(...))` at :120-126,
beside `los_world_wasm::hud_suffix()`. The rAF loop is otherwise untouched — `tick_object_wash` at
:88 and the T-670 scale publish are exactly where T-938.5 and t670 need them. It also mounts
`memory_budget.rs` a second time under `#[cfg(all(test, not(target_arch = "wasm32")))]`, at the very
end of the file (below `registry_session`'s `#[cfg(test)]`, so `class_r_scrub`'s whole-file cut never
sees it). **Without that mount the tests would compile for nobody**: `world_assets` is
`#![cfg(target_arch = "wasm32")]`, so a `#[cfg(test)] mod` living only there would be a green check
over an input it never examined. Same device as `mission_editor::tbd_sat_pure`, for the same reason.

## perturbation (RED verbatim)

Perturbation: the `Degrade` arm of `Ledger::decide` -> `Decision::Ok`, i.e. **`reserve` can never
return `Degrade`** (`reserve` delegates to `decide`; that arm is the only place `Degrade` is made).

```
test editor::canvas::viewport::memory_budget_pure::t938_6::decide_separates_shrink_from_never ... FAILED
test editor::canvas::viewport::memory_budget_pure::t938_6::reserve_records_only_on_ok ... FAILED
test editor::canvas::viewport::memory_budget_pure::t938_6::the_floor_rises_one_level_per_degrade ... FAILED

---- editor::canvas::viewport::memory_budget_pure::t938_6::the_floor_rises_one_level_per_degrade stdout ----

thread 'editor::canvas::viewport::memory_budget_pure::t938_6::the_floor_rises_one_level_per_degrade' (667093) panicked at apps/website/frontend/src/editor/canvas/../world_assets/memory_budget.rs:732:9:
assertion `left == right` failed: level 0 costs 1,026,523,730 B, which fits a 1024 MiB budget on its own but NOT beside the 163,840,000 B DEM raster — so the floor must rise by exactly one
  left: 0
 right: 1

---- editor::canvas::viewport::memory_budget_pure::t938_6::decide_separates_shrink_from_never stdout ----

thread 'editor::canvas::viewport::memory_budget_pure::t938_6::decide_separates_shrink_from_never' (667085) panicked at apps/website/frontend/src/editor/canvas/../world_assets/memory_budget.rs:662:9:
assertion `left == right` failed: 500 does not fit the 400 that is left but would fit an empty budget — that is the `ask smaller` answer, and collapsing it into Refuse is what would leave the satellite with no ladder to walk
  left: Ok
 right: Degrade

---- editor::canvas::viewport::memory_budget_pure::t938_6::reserve_records_only_on_ok stdout ----

thread 'editor::canvas::viewport::memory_budget_pure::t938_6::reserve_records_only_on_ok' (667089) panicked at apps/website/frontend/src/editor/canvas/../world_assets/memory_budget.rs:678:9:
assertion `left == right` failed: reserve and decide answer with the same arm
  left: Ok
 right: Degrade

test result: FAILED. 1353 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.60s
```

Restored with `git checkout --`, then **`touch`ed** the file (a restore alone does not re-trigger
cargo — the T-421 trap), re-ran:

```
   Compiling website-frontend v0.1.0 (.../worktrees/T-938.6/apps/website/frontend)
test result: ok. 1356 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.90s
```

Both halves show a real `Compiling website-frontend` line from **this worktree's path**, so neither
verdict is a T-596 replay. Baseline before the slice was 1344 passed; the slice adds 12.

## test / verify output

`hcargo xtask platform wave test --slice T-938.6 -p website-frontend` (private target dir):

```
   Compiling website-frontend v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.6/apps/website/frontend)
test result: ok. 1356 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.90s
```

`hcargo fmt -p website-frontend --check` -> clean.
`hcargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets` (CI's exact flags)
-> **zero warnings from `memory_budget.rs`**; the three touching my other files (`satellite.rs:218`,
`satellite.rs:388`, `viewport.rs:60`) are all pre-existing, on lines this slice did not change.

### Live acceptance — gate-checkable boolean

`gate render-check ... ?force=webgl&memBudgetMb=512 --assert-js <boolean probe>` -> **`assertOk =
True`, `assertValue = True`**. The probe asserts, in one expression: the Degrade log AND the
budget-cause log are on the console; `satMode==='unified' && satW===3200 && satH===3200 &&
satMips===12`; `__t9386.budget === 512*1048576 && satFloor === 2 && satRaised === 1`; and the HUD text
matches `/· mem \d+\/512MB · sat L2 \(\+1\)/`.

Console, verbatim, from that boot:

```
warn: satellite: memory budget Degraded level 1 (248 MiB resident — RGBA plus tile bodies, held together across the decode) — raising the mip floor by one. Budget 512 MiB, 312 MiB already held by the other world assets.
warn: satellite: DOWNSCALED basemap — showing level 2 (3200x3200) instead of level 0 (12800x12800). GPU maxTextureDimension2D = 8192 (adapter 8192); this GPU cannot hold the full-resolution basemap as a single texture — and 1 of those level(s) were taken by the MEMORY BUDGET, not by the GPU: see the `memory budget` warnings above.
log: satellite: basemap up — level 2, 3200x3200 with 12 mips (11286430 B over 13 Range requests); GPU maxTextureDimension2D = 8192
```

### Before / after, same rig, same mission

| | main default | **slice default (1536 MiB)** | **slice `?memBudgetMb=512`** |
|---|---|---|---|
| `satW x satH` | 6400^2 | 6400^2 | **3200^2** |
| `satMips` | 13 | 13 | **12** |
| satellite floor | 1 (GPU) | 1 (GPU), budget raised 0 | **2 (GPU 1, budget +1)** |
| wasm heap peak | 613,875,712 B | 614,465,536 B | **425,525,248 B** |
| satellite transfer | 42,152,810 B | 42,152,810 B | **11,286,430 B** |
| debug HUD tail | *(no memory cell)* | `· mem 404/1536MB · sat L1` | `· mem 219/512MB · sat L2 (+1)` |

**The default is byte-for-byte the old behaviour** (same level, same mip count, heap within 0.1 %) —
the budget only bites the case that was already fatal. Under 512 MiB the boot **completes with a
softer basemap**, 188,940,288 B (180.2 MiB) less wasm heap, instead of allocating into a wall.

## measured_peaks (per asset, everon)

Read off `window.__t9386` after a full boot of `/missions/aad0a12a-.../edit` on everon (6400^2 DEM,
12800^2 satellite), slice dist, default 1536 MiB budget.

| asset | peak (B) | MiB | kind | what it is |
|---|---|---|---|---|
| `dem` | **235,751,548** | 224.83 | declared | 71,911,548 B PNG + 163,840,000 B `Vec<f32>`, alive together across `decode_png_to_meters` |
| `hillshade` | **163,840,000** | 156.25 | declared | 6400^2 RGBA, released after `tex_layer_commit` |
| `satellite` | **260,606,070** | 248.53 | declared | level-1 chain: 218,453,260 B RGBA + 42,152,810 B bodies |
| `world` | **0** | 0 | measured growth | no new linear memory claimed |
| `forest` | **0** | 0 | measured growth | no new linear memory claimed |
| `labels` | **0** | 0 | measured growth | no new linear memory claimed |
| `water` | **0** | 0 | measured growth | everon declares no `water` block — the loader never runs |

Ledger totals: `peak` (declared high-water of the sum) **660,197,618 B = 629.62 MiB**; `held` at
hand-over 424,446,070 B. Measured wasm heap peak the same boot: **614,465,536 B = 586.02 MiB**.

The satellite figure is floor-dependent, and that is the whole point of the slice:

| satellite base level | resident (B) | MiB |
|---|---|---|
| 0 (a >=12800 px GPU) | 1,026,523,730 | 978.97 |
| 1 (this rig, 8192 px GPU) | 260,606,070 | 248.53 |
| 2 (`?memBudgetMb=512`) | 65,899,690 | 62.85 |

**Read the four zeros correctly.** They are a real measurement, not a broken probe — `heap` is
published from the same `heap_bytes()` and reads 614,465,536 B in the same object. Those four loaders
ran (the HUD shows `occl 133ch/1416bvh 115MB` from world-derived data) but claimed **no new linear
memory**, because the terrain lane had just freed 235,751,548 B back to the allocator (the
hillshade's 163,840,000 B plus the PNG's 71,911,548 B) and they fit in those pages. Growth is a lower
bound by construction — the wasm heap never shrinks — so a phase that reuses freed pages measures
zero. Two things follow, and both are worth having: the audit's framing is right (the terrain and
satellite lanes *are* the memory story on everon), and a byte-exact figure for the other four needs a
`GlobalAlloc` wrapper in `main.rs`, which is outside this slice's owns — see found_not_fixed.

Also note the ledger's declared total peak (629.62 MiB) **exceeds** the measured heap peak
(586.02 MiB): the per-asset high-water marks do not all coincide, so the ledger over-estimates. That
is the right direction for a guard — it refuses slightly early rather than slightly late.

## files_outside_owns

`[]` — none. `git diff --stat main...HEAD` is exactly the four owned files:

```
 apps/website/frontend/src/editor/canvas/viewport.rs            |  28 +-
 apps/website/frontend/src/editor/world_assets/memory_budget.rs | 878 +++++
 apps/website/frontend/src/editor/world_assets/mod.rs           |  75 +-
 apps/website/frontend/src/editor/world_assets/satellite.rs     |  55 +-
 4 files changed, 1031 insertions(+), 5 deletions(-)
```

Unowned pins checked and left intact: `t629_satellite_resolution.rs` (the literal
`pick_base_level_for_limit(&index, limit.map(|l| l.device))` survives verbatim;
`max_texture_dimension_2d()` still appears **exactly twice in the scrubbed view the pin reads**
(`satellite.rs:57` and `:58`, both inside `texture_limit`; a raw grep also hits a doc comment at
`:48`, and `unwrap_or(8192)` survives only in comments at `:48`/`:705` — `live_code` blanks both, and
my diff adds neither token, verified with `git diff main...HEAD | grep '^+'`);
`report_chosen_level(&index, base, limit)`, the `logging::error!` + `return false;`, and the
post-`tex_layer_commit` `logging::log!` all still present), `t628_boot_progress.rs` (no new
`BootEvent::Budget`, no new early return in `load_unified_full`, `planned_density_bins()` still
before `world.init(`, both `Finish` blocks still two `async {` deep), `t635_debug_hud.rs` /
`t670_scale_signal.rs` (HUD is still one `RwSignal<String>` set in one place, still after the scale
publish and inside the 1 Hz gate), `dock_left.rs:2147` and `ruler_tool.rs:1465` (their `only_body`
markers in `world_assets/mod.rs` are untouched and still unique). All pass in the 1356.

No `#[cfg(test)]` was added to `satellite.rs` or `world_assets/mod.rs` — both remain whole-file
haystack for Class-R scrubs.

## found_not_fixed

1. **Byte-exact peaks for `world` / `forest` / `labels` / `water` are not reachable from this slice's
   owns.** Their loaders live in `world_host.rs`, `forest_mass.rs`, `labels.rs`, `water.rs`
   (siblings' or unowned), and wasm linear-memory growth cannot see an allocation served from freed
   pages. The honest instrument is a `GlobalAlloc` wrapper installed in
   `apps/website/frontend/src/main.rs` that tracks live bytes; that is a crate-wide change and a
   different ticket. **Not deferred by me** — outside owns, reported as the brief requires.
2. **Only the satellite can degrade.** That is the ticket's own `LOCKED` clause, not a decision of
   mine. The DEM/hillshade lane is `hold`-only: by the time its size is known the memory exists, and
   it has no coarser variant to fall back to. A DEM that could load at half resolution would need a
   mip pyramid the `.dem`/PNG formats do not carry.
3. **`bridge.rs` publishes `satMips`, and a budget raise changes it** (13 -> 12 under 512 MiB). No
   consumer in the tree asserts a fixed `satMips`, and the gate is green, but any future golden that
   pins it must pin the budget too. `bridge.rs` is unowned; I changed nothing there.
4. **The `?memBudgetMb` override applies at first touch of the ledger and is never re-read.** A boot
   is the unit. Changing it needs a reload, which is stated in the module docs.

## deviations

1. **`reserve(bytes) -> Decision` is `Ledger::reserve(&mut self, asset, bytes)` plus
   `Ledger::decide(&self, bytes)`, not a free function.** The same requirement also asks for a
   *per-asset* registered peak, which a bare `reserve(bytes)` cannot record. I first added
   free-function wrappers as well; clippy showed them to be genuinely uncalled (the only caller that
   must ask before allocating is the satellite, and it asks through `claim_satellite_floor`, which
   walks and commits in one borrow), so I removed them rather than ship dead code. Both methods are
   exercised by the tests.
2. **`FloorWalk::raised()` is derived from the floor that moved, not from `rejected.len()`.** My own
   test caught the difference: when *no* level fits, the coarsest is rejected and then loaded anyway,
   so the rejection count is one higher than the resolution lost. Reporting that on the HUD would put
   up a number no rung corresponds to. I changed the code, not the expectation.
3. **The DEM's cost is forecast before the join.** Not spelled out in the ticket, but without it the
   satellite — which reaches its decision first, every time — sees an empty ledger and the budget is
   decorative. Four lines in `bootstrap`, two optional serde fields, inside owns.
4. **The `?memBudgetMb=512` demonstration raises the floor 1 -> 2, not 0 -> 1**, because the headless
   GPU reports `maxTextureDimension2D = 8192` and level 0 is never a candidate there. The mechanism
   is identical and the decision recorded is a genuine `Degrade`; level 0's 978.97 MiB is covered by
   `the_everon_ladder_costs_what_the_audit_says` and `the_floor_rises_one_level_per_degrade` against
   the real committed everon index.
5. **No real allocation abort was forced** — see defect_verified for why, and for what I cited
   instead. Stated rather than implied.

## commits

| sha | subject |
|---|---|
| `98d60cf32` | T-938.6: wasm memory budget with a satellite mip floor and a HUD readout |

Branch `slice/T-938.6`, committed and clean. **Not shipped, not merged, not pushed.** No `docs/` or
`.ai/tickets/` edits. Every `git add` named explicit paths; no `git add -A`, no `git stash`.

## gate_verdict_tail

```
gate: lock acquired after ~60s.
touch_workspace: invalidated 669 workspace .rs file(s) and 133 include_str!/include_bytes! input(s) across 9 member(s)
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
  T-278 catalogue drift    PASS
  db_migrate claim body    PASS
  db_migrate persist       PASS
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 98d60cf320c4 recorded: .ai/artifacts/verdicts/T-938.6.json
SLICE GATE: PASS
```

## manual_checklist

The brief's MANUAL step — *"Load everon with `?memBudgetMb=512`: satellite floor rises, no abort, HUD
shows the numbers"* — was executed headlessly and asserted, not eyeballed:

- [x] everon loads at `?memBudgetMb=512` — `satMode === 'unified'`, boot overlay reaches 100 %, no abort
- [x] satellite floor rises — 6400^2 / 13 mips -> **3200^2 / 12 mips**, `satFloor 2`, `satRaised 1`
- [x] HUD shows reserved-vs-budget and the floor — `· mem 219/512MB · sat L2 (+1)`, read from the
      live `[data-status-hud]` element after the real Ctrl+Alt+D arm toggled it
- [x] the raise is logged, and names the budget rather than blaming the GPU (console above)
- [x] the default budget changes nothing — 6400^2 / 13 mips, identical to `main`
- [x] `main` with the same URL is unaffected — the param is inert there
- [x] screenshot captured (the boot overlay is mid-dismiss in it; the DOM assertions above are the
      load-bearing evidence, not the picture)

Left running for the operator, as instructed: **API on `:8080`** (rebuilt, fresh). `:3000` was not
running on arrival and was not started — the browser runs used `gate render-check`'s own static
server on ports 5311-5317, all exited.
