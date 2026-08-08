# Wave 131 — adversarial verify (T-762 · T-767 · T-746)

**Verified HEAD:** `bcab27c8f27ccaf8c36f4fe74706ba3205d7951d`  
(`bcab27c8` wave 131 fixup: rustfmt world_assets::named_locations)

**Wave base:** `10124eed` (wave 107 CLOSED — editor wave 130) — ancestor of HEAD.  
**Merges in wave:** `ddb38718` (T-762), `2d294d9e` (T-767), `add1e5ac` (T-746), `bcab27c8` (rustfmt fixup).

**Method.** HOST cargo only. Private targets under `~/.cache/tbd-target-wave131-{verify,perturb,base}` (deleted before this report). Mutations only in detached worktrees at HEAD / base (`~/.cache/tbd-verify131-{perturb,base}` — deleted). Main checkout left byte-clean (`git status` empty of tracked dirt; HEAD unchanged). Pre-existing untracked `tbd-target-T-767/` left alone.

**NO-DEFERRAL note.** Orchestrator will fix all findings — reported honestly; no self-authored deferrals.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **971** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **971 passed / 0 failed** | same private dir; `--list` == run |
| base (`10124eed`) `--list` | **967** | isolated worktree + private dir |
| New frontend pins this wave | **4** | all T-746 (`a_get_that_races…`, `shape_mirror_wires…`, `t746_row_id_predicate…`, `t746_row_hydrate…`) |
| T-767 re-export pin | **1** in `map-engine-core` | `doc::reexport_pins::connection_and_formation_api_is_crate_public_via_doc` (needs `--features doc,mission`) |

967 + 4 = **971**. T-762 updated an existing dock pin (no new frontend test). T-767 lives in `map-engine-core`, not the frontend suite count.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | `apps/website/frontend/src/world_assets/mod.rs:139` + `:156` — **T-762 RENDER_CTX / LabelHost bodies are unpinned (hollow Class-R)**

**Evidence.**  
(a) Live code: `world_assets::fly_to` runs `RENDER_CTX` → `set_view` → `on_camera_changed` → `flush_viewport` (`mod.rs:139-149`). `named_locations` reads `mh.labels.towns()` (`mod.rs:156-162`). Dock forwards to those seams (`eden_dock_left.rs:1230`, `:1240`).  
(b) The only T-762 Class-R needle is `eden_dock_left::tests::the_index_and_the_fly_to_reuse_the_shipped_paths` — it greps dock `live_code` for `named_locations`, `world_assets::fly_to`, and absent `__editorCamSet`. **No pin reads `world_assets/mod.rs`.**  
(c) Perturbation (HEAD worktree, private target): replace `fly_to` body with `let _ = (x,y,zoom);` (no `RENDER_CTX` / no `set_view`) → dock pin still **ok**. Replace `named_locations` body with `Vec::new()` (never calls `towns()`) → dock pin still **ok**.  
(d) Contrast: restore `__editorCamSet` in the dock `fly_to` and drop the `world_assets::fly_to` call → dock pin **FAILED** (`fly-to must call the world_assets::fly_to RENDER_CTX seam`). Dock *wiring* is pinned; seam *bodies* are not.

**Impact.** Today’s main *has* the correct seams, but a one-line gut of `world_assets::fly_to` / `named_locations` ships green while Places fly-to silently no-ops and the named-places index is permanently empty — the ticket’s critical claim (“RENDER_CTX, not `__editorCamSet`”; “boot-parsed towns”) is gate-invisible at the implementation site. Standing hollow-pin family (wave 127–130 #2).

**Disposition — fix this wave.** Add Class-R / `live_code` pins on `world_assets/mod.rs` that require `RENDER_CTX` + `set_view` + `on_camera_changed` + `flush_viewport` inside `fly_to`, and `labels.towns()` (or `towns()`) inside `named_locations`. Perturbation must go RED when either body is emptied while the dock still calls the symbol.

---

### F2 — MAJOR | `apps/website/frontend/src/mission_commands.rs:1582` (`t746_row_hydrate_keeps_game_mode_beside_meta`) — **ROW_HYDRATE `game_mode` fill is a hollow source pin**

**Evidence.**  
(a) Production `set_row_meta` does fill `HydratedRow { game_mode: detail.game_mode.clone(), … }` into `ROW_HYDRATE` (`mission_commands.rs:529-536`).  
(b) The pin only asserts `only_body(…, "pub fn set_row_meta").contains("ROW_HYDRATE") && …contains("game_mode")` plus getter name strings.  
(c) Perturbation: keep `ROW_HYDRATE.with` but set `game_mode: String::new()` (no `detail.game_mode`) → pin still **ok**. Earlier attack clearing the cell to `None` while leaving `detail.game_mode.clone()` as an unused binding also stayed **ok** (identifiers satisfy the greps).  
(d) Contrast: T-746’s `shape_mirror_wires_the_sequencer` **does** go RED when `begin_patch` is stripped from `set_game_mode` (see verified-clean). Race machine `a_get_that_races_a_patch_cannot_apply` is pure and solid.

**Impact.** Boot hydrate can stop retaining the real `game_mode` while the suite stays green; ShapeMirror then seeds an empty/wrong mode until a later GET (or never, if a PATCH window skips GET). Ticket claim “ROW_HYDRATE game_mode” is not actually gated.

**Disposition — fix this wave.** Tighten the pin to require `detail.game_mode` (or equivalent) flowing into `HydratedRow` / `ROW_HYDRATE` — e.g. `only_body` must contain both `HydratedRow {` and `game_mode: detail.game_mode` (split needles so the assertion string cannot satisfy itself). Prefer a tiny behavioural unit test that calls `set_row_meta` with a fixture `MissionDetail` and asserts `hydrated_row().unwrap().game_mode`.

---

### F3 — NIT | `packages/tbd-schema/schema/mission.schema.json:286` — **formation prose refresh has no pin**

**Evidence.** Description now mentions `force_to_formation` + `formation_offsets` (correct vs T-672). Reverting the description to the stale “no mod reader / prose only” wording in a worktree caused **zero** test failures (`map-engine-core` formation filters still green; no schema-description Class-R).  
**Impact.** Doc lie can return without a red gate; enum values themselves remain pinned elsewhere.  
**Disposition — fix this wave** (one needle in an existing schema/formation pin, or accept as doc-only risk). Not behavioural.

---

## Claim attacks (by ticket)

### T-762 — `world_assets::fly_to` + `named_locations`

| claim | result |
|---|---|
| Dock fly_to uses `world_assets::fly_to`, not `window.__editorCamSet` | **HELD** at call site — live forward + pin RED on `__editorCamSet` restore |
| `fly_to` body uses `RENDER_CTX` / `set_view` / flush | **CODE PRESENT, UNPINNED** → **F1 MAJOR** |
| `named_locations` from boot-parsed `LabelHost` towns | **CODE PRESENT, UNPINNED** → **F1 MAJOR** |
| Class-R pins | **PARTIAL** — dock reuse pin solid for wiring; hollow for seam bodies |
| `labels.rs` `towns()` getter outside owns | **HELD / justified** — required for `named_locations`; `pub(super)` only |

### T-767 — connection API re-export + formation prose

| claim | result |
|---|---|
| Schema prose mentions `force_to_formation` | **HELD** in source; **unpinned** → **F3 NIT** |
| `doc/mod.rs` re-exports `ConnectionKind` / `Row` / `Finding` / `validate_connection_rows` / `formation_offsets` | **HELD** — `pub use store::{…}` at `doc/mod.rs:22-25` |
| Pin on re-export | **HELD** — drop `ConnectionKind` from `pub use` → compile error `unresolved import super::ConnectionKind` in `reexport_pins` |
| Re-export usable | **HELD** at crate boundary (`map_engine_core::doc::…` with `doc` feature; wasm frontend already enables `doc`) |
| Frontend `ConnKind` copy NOT retired | **HELD** — `context_menu::ConnKind` still present; production still parses via `ConnKind::parse` |

### T-746 — ShapeMirror GET↔PATCH single-flight + shared row id

| claim | result |
|---|---|
| `ShapeSeq` single-flight between GET and PATCH | **HELD** — pure race test green; production `begin_patch`/`end_patch`/`may_apply_load` + skip-GET when `patch_inflight > 0` |
| Wire pin on ShapeMirror | **HELD** — strip `begin_patch` from `set_game_mode` → `shape_mirror_wires_the_sequencer` RED; race test alone stays green (expected) |
| `is_mission_row_id` `pub(crate)` shared | **HELD** — definition in `eden_top_strip.rs:357`; ShapeMirror calls it; no private twin in `eden_settings`; visibility pin green |
| `ROW_HYDRATE` / `game_mode` | **CODE PRESENT, UNPINNED fill** → **F2 MAJOR** |
| Race test | **HELD** — `a_get_that_races_a_patch_cannot_apply` covers pre-PATCH GET, mid-flight reopen, post-`end_patch` staleness |

---

## Standing attack surfaces (wave 127–130)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten / `keep_z_rows` | Replace `keep_z_rows` body with `None` → `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` **FAILED** (pin still load-bearing). Early-`return None` while leaving the condition string in dead code stayed green — source pin, not runtime. | **FAILED to break** (fix still holds when body truly gutted) |
| 2. Hollow source pins | T-762 seam bodies hollow → **F1**; T-746 hydrate fill hollow → **F2**; T-746 ShapeSeq wire solid; T-762 dock wire solid | **F1 + F2** |
| 3. Stale thread_local / `Rc::ptr_eq` | No new hook in this wave’s owns; `t754` suite **14/14** green | **FAILED to break** |
| 4. Affordance clickable IFF | `a_finding_row_is_clickable_iff_the_router_resolves_its_subject` + t754 affordance pins green | **FAILED to break** |
| 5. Shared `CARGO_TARGET_DIR` / list vs run | Private `~/.cache/tbd-target-wave131-*` only; list **971** == run **971**; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**yes** — with **F1 and F2 fixed in-wave** (orchestrator NO-DEFERRAL). Main is not broken today; open risk is regression-blind seam/hydrate wiring, not a live data-loss bug on HEAD. F3 is prose-only.

---

## Verified-clean register (claims re-proved)

- T-762: dock no longer calls `__editorCamSet`; forwards to `world_assets::fly_to`; index reads `named_locations`; `towns()` getter present; dock wiring pin RED on smoke-hook regress; **bodies present in source**.  
- T-767: `pub use` lists all five API names; re-export pin compile-fails when `ConnectionKind` dropped; schema description contains `force_to_formation`; `ConnKind` copy still shipped.  
- T-746: `ShapeSeq` race machine; ShapeMirror calls sequencer (`begin_patch`/`end_patch`/`may_apply_load`); shared `is_mission_row_id`; hydrate cell + notes exist in source.  
- Suite: 971 list == 971 run; base 967; +4 T-746.  
- Main: `bcab27c8`, clean tracked tree after cleanup.

---

## Attacked and FAILED to break

1. **Dock fly_to / named_locations *call sites*** — pin catches `__editorCamSet` restore and missing `world_assets::fly_to`.  
2. **ShapeSeq race semantics** — pure unit test encodes GET-across-PATCH / reopen / post-settle rules.  
3. **ShapeMirror sequencer wiring** — `live_code` + `only_body` goes RED without `begin_patch` in `set_game_mode`.  
4. **Shared `is_mission_row_id`** — `pub(crate)` in `eden_top_strip`; ShapeMirror uses it; no eden_settings twin.  
5. **T-767 re-export load-bearing** — removing `ConnectionKind` from `pub use` fails the pin at compile time.  
6. **ConnKind non-retirement** — enum + parse path still present (as claimed).  
7. **`keep_z_rows` sticky-z pin** — full body→`None` goes RED.  
8. **Affordance / t754 / subject clickable-IFF** — green.  
9. **Private-target list/run mismatch** — none (971==971).  
10. **`labels::towns` justification** — required accessor for the promoted seam; not a scope violation finding.

---

## Focused re-verify

**HEAD:** `de92548a` (F3 tip; includes F1 `4daa1551` + F2 `f0a61d66`).  
**Method.** HOST cargo only. Detached worktree `~/.cache/tbd-verify131-reverify` @ HEAD. Private `CARGO_TARGET_DIR=~/.cache/tbd-target-w131-reverify`. Perturb → expect RED → `git checkout --` restore. Main left byte-identical (only this artifact append). Worktree + target dir deleted after report.

### Suite reconciliation (post-fix)

| measurement | value |
|---|---|
| `--list` | **972** |
| run | **972 passed / 0 failed** |
| prior wave verify | 971 (pre-F1 pin) |
| delta | **+1** = `eden_dock_left::tests::fly_to_and_named_locations_bodies_are_live` (F1) |

list == run. Confirmed.

### Attack matrix (claimed fixes only)

| claim | perturbation | pin under test | result |
|---|---|---|---|
| **F1** fly_to / named_locations bodies pinned | Gut both: `fly_to` → `let _ = (x,y,zoom);`; `named_locations` → `Vec::new()` | `fly_to_and_named_locations_bodies_are_live` | **RED** — `fly_to must reach RENDER_CTX` (exit 101) |
| **F1** towns() needle | Gut `named_locations` only → `Vec::new()` (fly_to intact) | same | **RED** — `named_locations must read LabelHost towns()` |
| **F1** set_view / flush needles | Keep `RENDER_CTX` token; drop `set_view` / `on_camera_changed` / `flush_viewport` | same | **RED** — `fly_to must call set_view` |
| **F1** dock wiring still hollow alone | Same dual-gut as row 1 | `the_index_and_the_fly_to_reuse_the_shipped_paths` | **GREEN** (expected) — new body pin is the load-bearing gate |
| **F2** `detail.game_mode` fill | `game_mode: detail.game_mode.clone()` → `game_mode: String::new()` in `set_row_meta` | `t746_row_hydrate_keeps_game_mode_beside_meta` | **RED** — `HydratedRow.game_mode must come from detail.game_mode` (exit 101) |
| **F3** schema prose | Revert `$defs/group.formation` description to pre-T-767 “no mod reader / prose only” (drops `force_to_formation` + `formation_offsets`) | `doc::store::tests::formation_offsets_are_distinct_per_schema_token_and_fall_back_to_column` (`--features doc,mission`) | **RED** — `must mention force_to_formation` (exit 101) |

Post-restore smoke: F1 body pin + F2 hydrate pin + F3 formation pin all **GREEN**. Worktree clean after restores.

### Findings

| severity | count |
|---|---|
| CRITICAL | **0** |
| MAJOR | **0** |
| NIT | **0** |

All three prior findings (**F1 MAJOR, F2 MAJOR, F3 NIT**) are **CLOSED** — original hollow attacks now go RED; restores green.

### Main safe to build the next wave on?

**yes** — F1/F2/F3 pins load-bearing; suite 972==972; main checkout unchanged aside from this artifact.
