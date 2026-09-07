# Wave 253 adversarial verify

Base `51c50be54` (wave 252 CLOSED). Dispatch HEADs `9f4a8c7d1` / `8c4669ca9`. Verify ran read-only
against `96030fd76`; the fixes below landed after it. Host cargo through
`/home/Samuel/.cache/tbd-bin/hcargo`, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`.

This was a **custom 10-wide pack**, not lock row `n = 253` (`T-936.6 T-941.4 T-941.8`). The other
seven sat in lock rows 254/255/257/258. Shipping all ten with `ticket ship --no-repack` then one
`wave repack` emptied two whole lock waves, so the repack froze `[[emptied]] n = 253`
(`T-936.6 T-941.4 T-941.8`) and `n = 254` (`T-938.2 T-938.3 T-938.4`); open waves start at 255.
`--reserve` was not needed. Close uses `--tickets` of all ten — the gated span since `51c50be54`.

| Ticket | Merge | Agent |
|---|---|---|
| T-936.6 | `1fdbbc555176e8f6cf7a16f7257552ed26c5c1b5` | authored spawnModules waves and garrisons |
| T-941.4 | `28a7ce04adef64219557e685e6053c87e9d2247c` | objective HUD replaces the chat pump |
| T-941.8 | `9d72811be23c6133ff494f96ac0134915a224bc2` | full fuel and per-class cargo on spawn |
| T-139   | `5616e105cd10e45b5204bfbe8bd8540d54824d5d` | lobby kit icon-grid preview |
| T-937.2 | `bc816976435f92fa42a8a551b5def49cf6827d72` | undo grouping per gesture window |
| T-937.4 | `0d08ea76d8a7efd1ca224e900053b3b22f911870` | persist save failures on chip and toast |
| T-930   | `f412df38e12c85836c5e0f76ceccb08cbb0e0d41` | first vehicle paint uses silhouette |
| T-938.2 | `8c5457178bb9d0e4c6f9ffd3925ad2fdfd9fe983` | chunk-crossing allocation counter |
| T-938.3 | `d70d9b2863859dcb797188897237a9191413d2b2` | GPU cull every icon lane |
| T-938.4 | `f264710155a26bf733cce41ecd4f72110ed275d7` | BVH section cuts, sparse HeightField |

## Fixed in-wave

### 1. BLOCKER — one `cull_params` uniform shared by all nine cull lanes (`1d514ea1b`)

`icon_cull_gpu.rs` created a single 32-byte `cull_params` on `IconComputeCull`, rewrote it per lane
in `encode_lane`, and bound it at binding 3 from **every** lane's bind group. `encode_cull` encodes
all lanes into ONE `CommandEncoder` and the frame is one submit; `Queue::write_buffer` is staged and
applied before any command buffer in that submit runs. So every lane read whichever `src_count` was
written last, and the loop iterates `self.lanes.keys()` of a `HashMap`, so which lane won was
nondeterministic per run.

With one lane this was invisible. T-938.3 made nine lanes normal. 380 slots + 12 tree glyphs: trees
last ⇒ `count = 12` and 368 of 380 slot icons vanish; slots last ⇒ the 1-workgroup trees dispatch
runs 64 threads over a 12-icon buffer and inflates `cull_counter`, so `draw_indirect` draws up to 64
instances from 12. Fires on essentially every WebGPU frame with a mission loaded.

`params_buf` moved into `LaneGpu`; the shared field is gone, so no accidental rebind is possible.
`icon_cull_gpu.rs` is `cfg(target_arch = "wasm32")`, so the native pin is the Class-R probe this
crate already uses for the module — `cull_params_is_per_lane`. **Perturbation:** reverting the write
to `self.params_buf` turns `per_lane_cull_params_not_shared` RED verbatim; restored, green.
`cargo test -p map-engine-render` 87/0; `cargo check --target wasm32-unknown-unknown` clean (the
native suite never compiles that file, so the wasm check is what proves the move builds).

### 2. Two frontend tests neither slice gate could run (`36d98499b`)

The first full gate was 30/31 with `test frontend` the only red — two deterministic failures.
`platform wave gate --slice` does not run the frontend suite, so neither slice could have seen them.

- **T-937.4's own Class-R probe was dead.** `unreadable_lockout_offers_retry` splits
  `save_status.rs` at the FIRST `#[cfg(test)]` and asserts the production half offers Retry — but
  the test-only `last_toast()` sat at line 47, above the `invoke_retry` / `"Retry"` it searched for.
  The haystack was lines 1–46 and the assertion could only ever fail. This is wave 252's T-937.1
  haystack trap (`edb7e69f8`) repeated one wave later in the same class of test. `last_toast` moved
  below every production item, with a comment saying why it must stay there.
- **T-938.4 regressed an unowned consumer.** The brief specified `f32` + NaN sparse storage, so
  heights read back f32-rounded (6.2 → 6.199999809). The slice widened five of its own goldens from
  `1e-9` to `1e-5`/`1e-4` and left the identical assertions in `building_viewer.rs` alone, because
  that file was not in its owns. The two HeightField-derived assertions there are now `1e-5` and say
  which change made them inexact; the other `1e-9` assertions in that file are 2D projection math
  and stay exact. `owns` gained `building_viewer.rs`, same shape as `a18efcc18`.

`wave.lock` carries an **owns snapshot**, so that widen made `check::tests::tip_registry_full_check_ok`
fail with `wave.lock owns snapshot disagrees with the ticket files on ["T-938.4"]` until a repack.
Worth knowing: `a18efcc18` had the same latent staleness and was masked only because the ship's
repack happened to fix it later.

## Filed, not fixed — T-946.51 … T-946.63 (`d0bf5a3b3`)

MAJOR: `.51` SAVE_IN_FLIGHT write-only and its test passes on the identifier · `.52` y-interval
index rebuilt per cut so total work went up · `.53` `authored_blocks_root` is a hand match with no
coverage guard · `.54` T-374 unreadable latch now set ~560 ms late · `.55` objective HUD replicates
to every player at 1 Hz with no dirty check · `.56` WebGPU slot upload does a pooled write it
discards · `.57` roster vehicles resolved by position only, 3 m neighbours collide · `.58` T-139 kit
preview structurally empty on a dedicated server · `.59` T-938.2 narrowed the brief's ship threshold
to one sub-metric.

NIT: `.60` cull equality test scrapes shader text · `.61` yrs `u64` clock underflow under `RealClock`
· `.62` chat-pump removal left a dead field and a stale glyph list · `.63` spawnModules exclusivity
accepts `x` without `z`.

`.53` and `.58` are the two worth reading first: the flatten match is the same defect class as
T-946.36/41/43/44/47 and already has its next victim named (T-936.7 `tacticalGraphics`), and `.58`
means a mounted, logging, always-empty widget everywhere the client is not also the server.

## Leftovers — unchanged, not re-filed

Flatten still drops authored blocks (T-946.36/41/43/44/47). Panels still unmounted
(T-946.33/39/45/49) — T-936.6 mounted `spawn_modules` only. `SpectatorHost` still treats 0 as
unlimited (T-946.50). `dem::peaks::tests::everon_peaks_max_above_350` fails in every slice worktree
on the LFS-pointer DEM: environmental, never a finding.

## Attacked and FAILED to break

- **T-936.6** — `TBD_DynamicSpawner` is not dead code: wired at `:566-599` via `modded class
  SCR_BaseGameMode` `OnGameStart` + self-rearming `CallLater`, the pattern `TBD_AudioEmitter`,
  `TBD_WeatherRuntime` and `TBD_TriggerRuntime` already use. The flatten test runs through
  `flatten_to_mod_document` on real JSON, so deleting the serde field fails it. The panel-mount
  needle is assembled from fragments so it cannot match itself, and `spawn_modules_panel(ctrl)` is
  genuinely in `MissionSettingsDialog` — the T-936.5 registered-but-unmounted mistake was not repeated.
- **T-941.4** — layout binding, `modded class` collision with T-139, and Rpc arity all hold: HUD-local
  `ResourceName` with the `TBD_UILayouts.Create` fallback, multiple `modded class` blocks are the
  established pattern, 7 payload params, send server-gated on `RplMode.Client`. `HideAllHuds` fires
  on `OnDelete` and the LIVE edge behind a `m_bLive` guard, so it cannot spam.
- **T-941.8** — best shot was `Apply(body, false, 1.0, ABSENT)` clearing an authored `lock: true`.
  It cannot: `Apply` is `if (lock) ApplyLock(...)`, so `false` is a no-op. Alias table walked against
  the real `veh:` vocabulary — `m151_mg`→jeep, `brdm2`/`btr70`/`lav25`→apc, `campaignradiotruckwest`
  →truck, `cinematic_flying_uh1`→helo, with helo tested first so no ordering trap.
- **T-139** — the grid cannot null-deref: `HandlerAttached` fills all three arrays to `CELL_COUNT = 13`
  and `Refresh(null, -1)` runs immediately; `RefreshPreviewFromTag` bounds-checks both ends;
  `MountLoadoutPreview` is idempotent and null-guards workspace, parent and handler cast. The defect
  is architectural (T-946.58), not defensive.
- **T-937.2** — `end_group()` → `UndoManager::reset()` is yrs's `stopCapturing` (sets
  `last_change = 0`), not a stack wipe. Two `#[wasm_bindgen(start)]` in one crate both run, so
  `install_wasm_now` fires and `real_now()` never sticks at its fallback. The façade is live:
  `editor_ops::delete_selection` / `paste_at_cursor` / `align_selection` all resolve to `batch::`
  because the glob re-exports were replaced by explicit lists omitting exactly those three names.
  201 groups → 200 undos → 200 redos → new edits: the cap stays sticky and never un-forgets.
- **T-937.4** — the chip is reachable: `register_flush_on_hide` is called from `mission_editor.rs`
  and calls `bind_runtime()` + `set_retry_handler`; `report()` calls `ensure_chip_mounted()` on every
  transition, so the overlay self-mounts without `top_strip.rs`.
- **T-930** — sharpest attack was the `atlas_ready` gate: `vehicles_bind_symbology` early-returns and
  caches nothing, so a mount-time call before the atlas would be a permanent silent no-op. It is not:
  `ensure_slot_atlas` runs synchronously from `build_marker_slot_atlas()` *before* the first bind.
- **T-938.2** — the counter does not miss the cited span: `push_landcover` runs before `commit_warm()`,
  and `commit_warm` is gated on `fetched && had_resident` sampled before `set_viewport`. Its claim
  that T-938.3 left its uploads alone holds — that diff touches no `upload_world_buildings`, outline
  or fence-strip code.
- **T-938.3** — `pub mod compute_cull;` already existed pre-slice, so the new native tests compile.
  Both `workgroupBarrier()`s are in uniform control flow, `wg_vis[lid]` is written unconditionally,
  and the atlas bind-group choice matches `draw_batches`' `IconInstanced` arm role-for-role. The
  T-151.11.1 in-order guarantee survives `emit_due`. The shared-uniform bug above is the one that got through.
- **T-938.4** — the f32 root-bounds early-out does not reject a cut at the exact mesh top: `Bvh::build`
  pads outward before the f32 cast, so the cull is conservative, pinned on all six goldens.
  `build_into` cannot recurse infinitely, and `owner.get(ti)` still indexes by true triangle id.

## Twins

Line counts identical in every pair; both trees normalised (em-dash, `§`, `→`, box-drawing → ASCII)
and the added/removed line sets diffed separately.

`TBD_DynamicSpawner.c`, `TBD_ObjectiveHud.layout`, `TBD_LoadoutPreview.c` and
`TBD_LoadoutPreview.layout` are **byte-for-byte identical**. `TBD_ObjectivesComponent.c`,
`TBD_ObjectiveHud.c`, `TBD_SpawnManager.c` and `TBD_LobbyScreen.c` are **identical in logic** —
residual lines are entirely comment folds and user-facing display strings, and every added or removed
*code* line matches. No logic divergence in any pair.

## Owns

`files_outside_owns` is `[]` for all ten; the only non-owned path per merge is that slice's own
`REPORT-T-XXX.md`, which the brief mandates. Both `a18efcc18` exceptions confirmed present
(`compute_cull.rs` on T-938.3, `building_section_tests.rs` on T-938.4). The in-wave fix added
`building_viewer.rs` to T-938.4.

## main_left_clean

Command-centre bookkeeping only on `main` (ticket TOMLs, `wave.lock`, T-946.51–.63, this file) plus
the two in-wave fix commits. Application code landed via `--no-ff` merges of `slice/T-*`.
