# Wave 139 — adversarial verify (T-732 · T-747 · T-726 + cite retarget)

**Verified HEAD:** `d2787ba533d075f1160028153819651ba48f1e16`  
(`d2787ba5` T-739 retarget set_loadout cites after T-732 line shift)

**Wave base:** `57f7f970` (wave 115 CLOSED — editor wave 138) — ancestor of HEAD.  
**Merges in wave:** `4f40bc14`/`97a66c8a`/`981f8044` (T-732), `fa991de8`/`85f787fd` (T-747),
`5d19629e`/`56f3919f` (T-726), `d2787ba5` (post-merge cite retarget).

**Gate (orchestrator):** PASS (re-gated after T-739 cite fix for T-732 line shift).  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave139-{verify,perturb,base}` (deleted after this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify139-{perturb,base}` — deleted). Main
checkout left byte-clean at HEAD aside from this report (`git status` → only pre-existing
`?? .ai/artifacts/t732_stage/` + `?? tbd-target-T-*`; HEAD unchanged). No fix / commit / ticket
filing.

**PRIMARY ATTACK.** T-732 undo-depth Class-R pins (per-patch / multi-txn hollow). T-747 tripwire
without features + neutralize hollow + wave-gate feature-set honesty. T-726 Esc stack topmost /
`any_open` hollow + census of listeners outside the widened list. Cite pins after line shift
(stale number + blank-line shift). Hollow-pin attacks on every new pin.

**NO-DEFERRAL note.** Every severity reported honestly — including found_not_fixed residues the
ticket already named. Process note (duplicate T-732 agents, cleaned, solo gate PASS) is not a
finding.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **1014** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD frontend run | **1014 passed / 0 failed** | same private dir; `--list` == run |
| base (`57f7f970`) frontend `--list` | **1006** | isolated worktree + private dir |
| Net frontend delta | **+8** | all T-726 pins (cite retarget adds no tests; T-732/T-747 add none on frontend list) |
| HEAD map-engine-core bare `--list` | **140** | no features |
| HEAD map-engine-core `--features doc,mission` `--list` / run | **502** / **501 pass + 1 ignored** | wave.sh gate feature set |
| HEAD map-engine-core `--all-features` `--list` / run | **635** / **634 pass + 1 ignored** | Makefile / T-747 stated target |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `attributes::t726_attributes_esc_stack::attributes_modal_gates_escape_on_modal_stack` | T-726 |
| `context_menu::t726_context_menu_esc_stack::context_menu_gates_escape_on_modal_stack` | T-726 |
| `eden_settings::t726_settings_esc_stack::settings_dialogs_gate_escape_on_modal_stack` | T-726 |
| `eden_settings::t726_settings_esc_stack::prefs_and_all_settings_mount_after_settings_body` | T-726 |
| `mission_editor::t726_window_esc_stack::editor_overlay_esc_listeners_gate_on_modal_stack` | T-726 |
| `mission_editor::t726_window_esc_stack::measure_tool_escape_arm_yields_when_modal_stack_has_open` | T-726 |
| `mission_editor::t726_window_esc_stack::measure_any_open_guard_is_load_bearing` | T-726 |
| `ui::tests::stacked_dialogs_only_topmost_answers_escape` | T-726 |

1006 + 8 = **1014**. Confirmed. List == run.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | T-732 — `attrs_update_position_multi` still N undo steps; rustdoc falsely claims no atomic API

**Evidence.** Live host path still loops per-id mutators:

```text
apps/website/frontend/src/editor_ops.rs:1454–1493
  rustdoc: "exposes no atomic multi-slot position API, so an N-slot commit is N undo steps"
  body: for id in ids { core.update_slot_position(id, …); }
```

Meanwhile T-732 shipped `MissionDocCore::update_entity_transforms` (one `begin()` over the patch
list) and migrated rotate / T-645 `commit_positions` / unmanned place onto it. Attributes multi
commit (`attributes.rs` → `attrs_update_position_multi`) was **not** migrated.

Executable probe (detached worktree, deleted after): injected
`probe_per_id_update_slot_position_is_n_undo_steps` mirroring that host loop →
`undo_depth() == 2` for two slots (PASS as defect demonstration). Contrast Class-R pins
`update_entity_transforms_is_one_undo_step_for_mixed_batch` / `rotate_entities_is_one_undo_step…`
which go RED when atomicity is broken (H1 below).

**Impact.** Ticket claim “migrate the … call sites; pin undo depth 1 per op” is incomplete for
Attributes multi-edit — still N Ctrl+Z for an N-slot stamp. Rustdoc actively lies after the API
exists (teaches the next agent the wall is still there). Same family wart T-732 was filed to close.

**Disposition — fix this wave.** Route `attrs_update_position_multi` through
`update_entity_transforms` (one LOCAL txn); rewrite the undo-granularity rustdoc to match; add a
Class-R / host scrub pin that fails if the per-id `update_slot_position` loop returns.

---

### F2 — MAJOR | T-747 — wave gate still skips ~133 tests; docs/assert claim `--all-features` soundness

**Evidence.** Measured feature-set coverage on HEAD:

| invocation | tests listed | notes |
|---|---|---|
| bare `cargo test -p map-engine-core --lib` | **140** | tripwire RED (good) |
| `--features doc,mission` | **502** | **what `wave.sh` gate runs** (`scripts/platform/wave.sh` ~2731) |
| `--all-features` | **635** | Makefile `ci-local` / T-747 stated target |
| delta gate vs all-features | **133 missing** | mostly `world::*` (+ some `dem::*`) |

T-747 tripwire message + `EDITOR_FACTORY_START.md` say the wave gate / Makefile are sound with
`--all-features`. Makefile:183 is correct. **`wave.sh` is not** — it still uses
`--features doc,mission` and never compiles/runs the world suite.

Tripwire itself only asserts `cfg!(feature = "doc")`, so `--features doc,mission` greens the
tripwire while still omitting world. Ad-hoc
`bash scripts/platform/wave.sh test --slice T-747 -p map-engine-core …` without features correctly
RED (tripwire load-bearing for the bare hole).

**Impact.** T-747 closed the *bare* vacuous pass (agents can no longer mistake 140 green for the
real suite). It did **not** close the residual class for the orchestrator gate: world/dem tests
remain invisible to `wave.sh` while prose claims `--all-features` soundness. Same signature defect
(“success over code never examined”), pointed at the gate rather than at ad-hoc cargo.

**Disposition — fix this wave.** Align `wave.sh` `test map-engine` (and the stale comment’s census
numbers) with `cargo test -p map-engine-core --all-features` (or at least
`--features doc,mission,world`); fix EDITOR_FACTORY_START + tripwire message so they do not claim
the wave gate already uses `--all-features`. Optionally harden the tripwire to require `world` as
well if that is the intended floor.

---

### F3 — MAJOR | T-726 found_not_fixed — Esc listeners outside the widened list still pile up

**Evidence.** Ticket already named “some Esc listeners outside widened list.” Live census of
`ev.key() == "Escape"` / `"Escape" if` window consumers on HEAD:

| surface | modal_stack gate? | in T-726 pin set? |
|---|---|---|
| `eden_settings` Mission/Prefs/All Settings | `is_topmost_open` | yes |
| `attributes` AttributesModal | `is_topmost_open` | yes |
| `context_menu` | `is_topmost_open` | yes |
| `mission_editor` Asset/Comment/Connections overlays | `is_topmost_open` | yes |
| `mission_editor` measure Esc arm | `any_open()` yield | yes |
| `ui` Dialog / FormModal | `is_topmost_open` | pre-existing T-333 |
| **`orbat_manager.rs:250`** | **bare `open && Escape`** | **no** |
| **`faction_manager.rs:71`** | **bare `open && Escape`** | **no** |
| **`eden_top_strip.rs:773`** | **bare Escape (menus/export/save/hint)** | **no** |
| `layout.rs` mobile/nav | SPA chrome | out of editor T-726 owns |

ORBAT Manager + Faction Manager are mounted from `mission_editor.rs` (~5086/5089) alongside the
dialogs T-726 fixed — they **can** be open in the same editor session. One Esc still closes an
unguarded manager **and** a stacked topmost dialog/menu that also answers Escape.

Widened pins are load-bearing for the surfaces they cover (H4/H6 RED when gates stripped) but
**do not** census-fail on orbat/faction/top-strip.

**Impact.** Same defect class T-726 was filed to eliminate (one Esc → multiple consumers). Partial
ship: settings/attributes/context/measure fixed; ORBAT/Faction/top-strip still unguarded.

**Disposition — fix this wave (NO-DEFERRAL; ticket already flagged found_not_fixed).** Register
ORBAT + Faction with `modal_stack` and gate on `is_topmost_open`; make `eden_top_strip` Esc yield
on `any_open()` (or register its transient menus). Extend the T-726 scrub pins so deleting those
gates goes RED.

---

### NIT | T-747 tripwire message — “at least `--features doc`” is not enough to compile the suite

**Evidence.** `cargo test -p map-engine-core --features doc --lib` fails to compile (doc tests
reference `crate::mission::…` which is behind `mission`). Tripwire / EDITOR_FACTORY_START still
say “or at least `--features doc`.”

**Impact.** Agent who follows the literal minimum hits a compile error, not a green vacuous suite —
less dangerous than silent skip, but the written floor is wrong.

**Disposition — fix this wave (nit):** state `--features doc,mission,world` / `--all-features` as
the minimum; drop “doc alone” wording.

---

## Main safe to build the next wave on?

**no** — not until F1 (T-732 attrs multi still N-step + false doc), F2 (wave.sh still skips ~133
world tests while claiming `--all-features` soundness), and F3 (T-726 found_not_fixed ORBAT /
Faction / top-strip Esc) are fixed. NIT in-wave under NO-DEFERRAL.

T-732 store atomic APIs + migrated rotate/place/T-645 paths held under hollow attack. T-747
tripwire **does** fail bare runs (including via `wave.sh test --slice` without features). T-726
widened surfaces + pins held. Cite retarget `2002`/`2016` matches live `set_loadout` /
`after_local_edit` and pin goes RED on drift.

---

## Verified-clean register (claims re-proved)

- **T-732 store APIs:** `place_vehicle_with_crew_stamp`, `rotate_entities`,
  `update_entity_transforms` each assert `undo_depth() == 1` on multi-entity shapes (4 Class-R
  pins GREEN on HEAD with `--all-features`).
- **T-732 host migrations that held:** `editor_ops` rotate / `commit_positions` /
  `place_vehicle_with_crew_stamp` call sites; >10 confirm copy states one-step undo
  (`confirm_bulk_one_step` / “Ctrl+Z undoes the whole op”).
- **T-747 tripwire:** bare suite RED on `feature_gate_tripwire::…`; `--all-features` tripwire
  GREEN; neutralizing tripwire → **140/140 green vacuous** (H5d).
- **T-747 docs:** `EDITOR_FACTORY_START.md` records the bare hazard + `wave.sh test` does not
  inject features (accurate for the ad-hoc path).
- **T-726 widened surfaces:** settings / attributes / context / editor overlays / measure
  `any_open` present; stacked prefs-over-settings behavioral pin GREEN; scrub pins RED when gates
  stripped (H4/H6).
- **Cite retarget:** live `pub fn set_loadout` @ **2002**, `after_local_edit` @ **2016**; arsenal
  production cites match; `arsenal_cites_live_set_loadout_lines` GREEN on HEAD, RED on stale cite
  (H3) and blank-line shift (H7).
- Suites: frontend **1014 == 1014**; map-engine `--all-features` **635 == 634+1 ignored**.
- Main: `d2787ba5`, tracked tree untouched aside from this report.

---

## Attacked and FAILED to break

1. **T-732 rotate/update undo-depth pins (H1)** — per-patch `begin()` → both Class-R pins RED
   (`undo_depth` 3 and 2 vs 1). Pins are load-bearing.
2. **T-732 place stamp undo-depth pin (H2b)** — three LOCAL txns → pin RED (`undo_depth` 3 vs 1).
3. **T-747 bare tripwire** — `cargo test -p map-engine-core --lib` FAIL; `wave.sh test --slice
   T-747 -p map-engine-core` without features FAIL.
4. **T-747 tripwire with `--all-features`** — PASS.
5. **T-747 neutralize hollow (H5d)** — without assert, bare suite **140 passed** (proves tripwire
   is the only barrier to vacuous green).
6. **T-726 attributes Esc gate strip (H4)** — pin RED (`must gate Escape on is_topmost_open`).
7. **T-726 measure `any_open` strip (H6)** — pin RED.
8. **Cite stale 2002/2016 → 1999 (H3)** — cite pin RED (demands live `editor_ops.rs:2002`).
9. **Cite line-shift blank before `set_loadout` (H7)** — cite pin RED (demands `:2003`).
10. **HEAD cite + T-726 pins GREEN** — arsenal cite + 6 T-726 named tests PASS.
11. **Frontend list↔run mismatch** — none (1014 == 1014).
12. **map-engine `--all-features` list↔run** — none (635 accounted).

*(F1/F2/F3 are the misses — residual defects, not held attacks.)*

---

## Safe? / register summary

| question | answer |
|---|---|
| **Main safe to build the next wave on?** | **no** (F1 T-732 attrs multi; F2 wave.sh feature gap vs `--all-features` claim; F3 T-726 found_not_fixed Esc listeners; NIT tripwire wording) |
| **Attacked-and-FAILED-to-break** | See numbered register above (12 held attacks; F1/F2/F3 are the live defects) |

---

## Focused re-verify

**Re-verify HEAD:** `2256030ad8e69cb762bb49069ac60bde42cbccd3`  
(fixes since prior verify: `bd8e742d` T-732 · `71118a86` T-747 · `ea82f26b` T-726 · `e95f7d71` rustfmt · `2256030a` T-739 cite 2005/2019)

**Method.** Documents only — no fix / commit / push / registry. HOST cargo
(`PATH=$HOME/.cargo/bin:$PATH`, `TBD_WAVE_GENERATION_FLOOR=100`). Private targets
`~/.cache/tbd-target-w139-reverify{,-pert}` + detached worktree
`~/.cache/tbd-verify139-repert` at HEAD (mutated only for RED; deleted after). Main
checkout HEAD commit unchanged; working tree dirty only this report.

**Orchestrator re-gate:** PASS (30/30) — accepted as process evidence; focused probes below
are independent.

### Suite delta since prior verify (`d2787ba5`)

| measurement | prior | now | note |
|---|---|---|---|
| frontend `--list` | 1014 | **1018** | +4 F3 pins (orbat / faction / top-strip ×2) |
| map-engine bare / doc,mission / `--all-features` | 140 / 502 / 635 | **140 / 502 / 635** | unchanged census |

---

### F1 — T-732 attrs multi → `update_entity_transforms` — **CLOSED**

**Evidence of fix on HEAD**

- `editor_ops.rs:1454–1501`: rustdoc claims one LOCAL txn via `update_entity_transforms`; body
  builds `EntityTransformPatch` list and calls `core.update_entity_transforms(&patches, …)` (no
  per-id `update_slot_position` loop).
- Prior false rustdoc (“exposes no atomic multi-slot position API”) is gone from this function.
- GREEN pins (HEAD, private target):
  - `attributes::tests::an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` → **ok**
  - `mission_editor::t649_select_all_and_multi_edit::multi_edit_commits_fan_out_to_every_selected_id` → **ok**

**Attack (would go RED if fix reverted)**

In detached worktree: replace `attrs_update_position_multi` body with per-id
`core.update_slot_position(...)` loop (pre-`bd8e742d` shape).

- `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` → **FAILED** —
  `must commit via update_entity_transforms (T-732 atomic batch)`
- `multi_edit_commits_fan_out_to_every_selected_id` → **FAILED** —
  `must commit via update_entity_transforms (one LOCAL txn)`

**Verdict: CLOSED.**

*(Note: `apply_loadout_buffer_to_selection` rustdoc still documents N undo steps for multi-loadout
writes — different API, not the attrs-position defect F1 named.)*

---

### F2 — T-747 `wave.sh` `--all-features` + tripwire harden — **CLOSED**

**Evidence of fix on HEAD**

- `scripts/platform/wave.sh:2730`: gate runs
  `cargo test -p map-engine-core --all-features -p map-engine-render --quiet`
  (comment at 2723–2726 explicitly bans bare and `doc,mission`-only).
- Makefile:183 already `--all-features` (aligned).
- Tripwire `feature_gate_tripwire::map_engine_core_tests_require_doc_feature` asserts
  `cfg!(doc) && cfg!(mission) && cfg!(world)` (`lib.rs:57`).
- Feature matrix (executable):

| invocation | tripwire | listed |
|---|---|---|
| `--all-features` | **PASS** | 635 |
| bare | **FAIL** | 140 |
| `--features doc,mission` | **FAIL** (world missing) | 502 |

**Attack (would go RED / hollow if fix reverted)**

1. **Gate honesty:** mutate worktree `wave.sh` gate line back to `--features doc,mission` →
   file no longer matches Makefile/`--all-features` contract (static ACCEPT that the F2 defect
   returns). HEAD line remains `--all-features`.
2. **Tripwire harden load-bearing:** weaken tripwire to `cfg!(feature = "doc")` only →
   `cargo test -p map-engine-core --features doc,mission --lib feature_gate_tripwire` → **ok**
   (vacuous GREEN over the 502-suite hole). On HEAD the same `doc,mission` run is **FAIL**.

**NIT (prior):** “at least `--features doc`” wording — **CLOSED** on HEAD; tripwire +
`EDITOR_FACTORY_START.md` now say `--all-features` / `doc,mission,world` and that `doc` alone
does not compile.

**Verdict: CLOSED.**

---

### F3 — T-726 ORBAT / Faction / top-strip Esc — **CLOSED**

**Evidence of fix on HEAD**

| surface | gate | pin GREEN |
|---|---|---|
| `orbat_manager.rs:249–255` | `register` + `is_topmost_open(modal_id)` | `orbat_manager_gates_escape_on_modal_stack` **ok** |
| `faction_manager.rs:68–76` | `register` + `is_topmost_open(modal_id)` | `faction_manager_gates_escape_on_modal_stack` **ok** |
| `eden_top_strip.rs:773–777` | yield on `modal_stack::any_open()` | `top_command_strip_escape_yields…` + `top_strip_any_open_guard_is_load_bearing` **ok** |

**Attack (would go RED if fix reverted)**

Worktree: strip `is_topmost_open` from ORBAT/Faction Esc arms; strip `any_open()` yield from
top-strip.

- `orbat_manager_gates_escape_on_modal_stack` → **FAILED** (`must gate Escape on is_topmost_open`)
- `faction_manager_gates_escape_on_modal_stack` → **FAILED** (same)
- `top_command_strip_escape_yields_when_modal_stack_has_open` → **FAILED** (`must consult … any_open()`)

**Verdict: CLOSED.** (SPA `layout.rs` chrome Esc remains out of editor T-726 owns — unchanged.)

---

### Cite pin — `set_loadout` / `after_local_edit` — **CONFIRMED 2005 / 2019**

**Re-measured on HEAD (do not trust prior):**

```text
editor_ops.rs:2005  pub fn set_loadout(...)
editor_ops.rs:2019  crate::mission_history::after_local_edit();
```

Production arsenal cites: `arsenal.rs:26` → `editor_ops.rs:2005`; `arsenal.rs:1316` →
`editor_ops.rs:2019`.

- GREEN: `arsenal::tests::t739::arsenal_cites_live_set_loadout_lines` → **ok**
- RED (cite-only attack): replace production cites with `editor_ops.rs:1999` while live lines stay
  2005/2019 → **FAILED** — `must cite live set_loadout at editor_ops.rs:2005`

---

## Focused re-verify — safe?

| Main safe to build the next wave on? | **yes** |
|---|---|

All three MAJOR findings (F1/F2/F3) CLOSED under ACCEPT/REFUSE probes; cite pin live at
**2005/2019**; prior T-747 NIT wording CLOSED. No remaining open item from the prior NOT-SAFE
verdict.
