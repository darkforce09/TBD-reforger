# Wave 210 adversarial verification — T-818 / T-819 / T-836

Verifier: Cursor Grok 4.5, 2026-08-11. Verified MERGED MAIN at **00eea875** (`git rev-parse HEAD` = `00eea8758840b95fdcd31019574b78e58dda76cb`).

| Pin | Sha / note |
|---|---|
| Wave base (last close) | `c24c7e8a` — wave 131 CLOSED — editor wave 208 |
| Merge T-818 | `22f3b137` (slice `ccd26a62`) |
| Merge T-819 | `3c895070` (slice `82007ed6` + rustfmt `af251a68`) |
| Merge T-836 | `dbacc087` (slice `c4b7e0e6`) |
| Completion | `ddbdcddd` (split `MissionEditorPage` scrub anchor) · `00eea875` (align T-802 hover cache pin to `map_render_slot_soa`) |
| HEAD re-check at start + exit | `00eea875` — **nothing landed after dispatch** |
| Wave gate | PASS claimed (frontend 1204/1204). Per §5: `wave.sh` runs **zero** editor smokes — GATE PASS ≠ editor-suite green |

**Environment left as found:** no repo files mutated except this report; no commits; no tickets filed. Kit-aliases perturbation restored byte-exact (`cmp` vs `/tmp/kit-aliases.wave210.bak`) + `touch` on `kit.rs`. Ephemeral Chromium profiles / CDP JSON under `/tmp/w210-verify/` (not in repo). Pre-existing `:8080` API and `:3000` trunk left running. `git status` at exit: clean working tree aside from this untracked report path under `.ai/artifacts/`.

**Surface:** Class-R via `cargo xtask ai run` / `cargo test -p map-engine-core --features mission`; headless Chromium 1228 (`--headless=new`, SwiftShader, `--remote-allow-origins=*`), fresh profile; live trunk dist wasm mtime `2026-08-11 05:00:53` (post-`00eea875`) contains `vehicle_attrs_view`, `map_render_slot_soa`, `veh:m1025_m2`.

---

## FINDINGS

### F1 — Orphaned Placed-strip UI remains in `eden_vehicles_panel.rs`
`NIT | apps/website/frontend/src/eden_vehicles_panel.rs:1-98 (+ wasm/native stubs) | strip deleted from DockRight as claimed, but the old panel fn + seat/cargo constants remain with \`#![allow(dead_code)]\` and stale module prose that still says cargo lives under the Vehicles tab | source audit`

- Evidence: `rg 'placed_vehicles_panel\('` — **no call sites** outside the function definitions and the T-818 negative Class-R assert in `eden_dock_right.rs`. Module header still claims the Placed section is “where authored vehicle cargo is entered” / “lives in the Vehicles tab rather than in the Attributes modal.” `attributes.rs` duplicated `FIXED_SEATS` / `DEFAULT_CARGO_SEATS` / `VEHICLE_CARGO_KINDS` (byte-equal to the dead panel today).
- Impact: No live strip; operator claim “Placed strip dies” holds for DockRight. Dead code + stale comments are a drift hazard if someone later re-wires the panel without noticing Attributes owns the editor.
- Disposition: **NIT** — do not block; optional cleanup of the orphan module / shared seat-model helper. Not filed (verifier does not file tickets).

### F2 — Attributes (incl. new vehicle body) still hard-codes `z-50`, not `modal_stack::z_class`
`NIT | apps/website/frontend/src/attributes.rs:~434,~775 | Esc is modal_stack-gated (T-726); overlay paint still literal z-50 | source audit`

- Evidence: `AttributesModal` registers + `is_topmost_open` + unregister (Class-R `attributes_modal_gates_escape_on_modal_stack` green). Both slot `modal_view` and T-818 `vehicle_attrs_view` overlays use `z-50` classes. OrbatManager is the surface pinned to `modal_stack::z_class` (`ui.rs` O-3). This is **pre-existing Attributes pattern**, inherited by the moved vehicle editor — not a new Esc break.
- Impact: Ticket trap named z_class; Esc consumption is correct. Stacked paint order vs Arsenal/ORBAT still relies on hardcoded tiers for Attributes.
- Disposition: **NIT** — pre-existing; not a wave-210 functional miss of Heading/Cargo/Crew move.

No BLOCKER. No MAJOR.

---

## Safe-line

**Yes — `main` at `00eea875` is safe to build the next wave on.**

T-818 moved the vehicle editor into Attributes and removed the DockRight Placed strip (Class-R + served wasm symbols). T-819 derived map-render hide is real (keep-filter, selection/materialize separation, history/page binds, intentional RED perturbation, T-802 pin retargeted). T-836 aliases resolve all four seed ResourceNames and the flatten pin goes RED when one is removed. Residual NIT dead-panel / Attributes `z-50` are not wave-stopping.

---

## VERIFIED-CLEAN REGISTER

### T-818 — Vehicle Attributes gets Heading/Cargo/Crew; Placed strip dies

| Claim | Result | Evidence |
|---|---|---|
| Dblclick / Attributes routes vehicles to Heading/Cargo/Crew body | **PASS** | `attributes_modal_routes_vehicles_to_the_vehicle_editor` — `is_vehicle_id` → `vehicle_attrs_view`; labels `"Heading"`, `"Add cargo"`, `"Crew"`, `"Cargo"` on `live_source` |
| Same mutators as old strip (`set_vehicle_heading` / `set_vehicle_cargo` / `assign_crew_seat` / `clear_crew_seat`) | **PASS** | `vehicle_attrs_view_wires_heading_cargo_crew_through_existing_mutators`; heading via `number_field` + `Gate::open()` (T-785) |
| Vehicles tab catalog-only — no Placed strip | **PASS** | `vehicles_tab_is_catalog_only_without_the_placed_strip` — no `placed_vehicles_panel(` in `DockRight`, no `"Placed"` heading in dock live_source; DockRight comment @~2047 documents deletion |
| modal_stack Esc | **PASS** | `attributes_modal_gates_escape_on_modal_stack` green; register / topmost / unregister present |
| Orphan Add-cargo path / strip remnant reachable | **REFUTED as live path** | Zero call sites to `placed_vehicles_panel(`; F1 notes dead module only |
| Served wasm carries vehicle editor | **PASS** | `strings` on `dist/...822dbe99..._bg.wasm` → `attributes::vehicle_attrs_view` |
| Live CDP: whole-editor body has no Placed strip heading | **PASS (partial)** | Auth’d smoke editor `/missions/smoke/edit`: `/\bPlaced\b/` false on `document.body` (outliner “Placed vehicles” string exists in wasm from `eden_tree`, not the dock strip). Vehicles tab button not found in this DOM snapshot (Asset Browser h3 visible) — Class-R owns the DockRight contract |

### T-819 — Crewed slots leave map render (derived)

| Claim | Result | Evidence |
|---|---|---|
| Assign Driver+Gunner drops two map-render rows | **PASS** | `assign_driver_and_gunner_drops_two_map_render_rows` |
| Selection / slots_json universe keeps boarded ids (not T-701 materialize drop) | **PASS** | `crewed_slots_remain_in_slots_json_universe_and_selection` |
| Unassign restores figure; stored z exact f64 untouched | **PASS** | `unassign_restores_figure_at_stored_z_exact_f64` (`12.345678901234567`) |
| Delete vehicle / undo restore visibility | **PASS** | `delete_vehicle_restores_both_figures`, `undo_assignment_round_trips_visibility` |
| No `editorHidden` / `set_slots_editor_hidden` on assign | **PASS** | `assign_crew_seat_does_not_write_editor_hidden` on scrubbed `editor_ops` |
| Map binds use `map_render_slot_soa` (history + page) | **PASS** | `map_binds_feed_map_render_slot_soa`; `mission_history::{rebind_engine_from_doc,after_doc_change}` feed filtered SoA + authored `slot_count` |
| Filter copies `zs` (no f32 squash in filter path) | **PASS** | `filter_slot_soa_excluding` pushes `soa.zs[i]` |
| Owns scope: no core `materialize()` change | **PASS** | T-819 merge touches only `mission_editor.rs` / `editor_ops.rs` / `mission_history.rs` |
| Hollow-pin attack (ignore crewed keep) | **PASS (anti-hollow)** | `perturbing_the_keep_filter_makes_the_assign_pin_fail` — catch_unwind RED with “Driver+Gunner must leave…”; suite **8 passed** |
| Completion scrub-anchor landmine | **PASS** | `ddbdcddd` splits `MissionEditorPage` anchor; unbroken full signature count in file = **1**; t819 uses `format!("{}{}", "pub fn Mission", "EditorPage()…")` |
| T-802 hover cache pin vs crewed hide | **PASS** | `00eea875` retargets pin to `map_render_slot_soa`; `hover_hit` body uses it (not bare `materialize()`); `the_point_sets_are_cached_against_the_lane_binding_tick` green — pin would fail if hover reverted to `materialize()` |

`cargo xtask ai run -- 'cargo test -p website-frontend t819_crewed'`: **8 passed**.

### T-836 — Seed veh: aliases

| Claim | Result | Evidence |
|---|---|---|
| Four seed ResourceNames → `veh:` aliases | **PASS** | Exact `registry_dev.sql` rows map to `veh:m1025_m2`, `veh:m998`, `veh:m923a1`, `veh:m113_m2` in `kit-aliases.json` (format matches existing `{alias,resourceName}` rows) |
| Compile clean for mission with all four | **PASS** | `seeded_vehicle_resource_names_resolve_through_kit_aliases` places all four and asserts wire aliases |
| Anti-vacuity missing alias | **PASS** | Asserts `vehicle_for_resource(DEADBEEF…)` is `None` before seed checks |
| Test RED without alias (hollow attack) | **PASS (not hollow)** | Removed `veh:m1025_m2` block → panic `lacks kit-aliases veh: row (T-836)` left `None` right `Some("veh:m1025_m2")`; restored byte-exact + re-green |

`cargo test -p map-engine-core --features mission seeded_vehicle_resource_names_resolve_through_kit_aliases`: green after restore.

---

## Attacked and FAILED to break

1. **T-818 DockRight Placed strip still mounted** — Class-R negative needles + zero call sites; live smoke body has no `\bPlaced\b` strip heading.
2. **T-818 mutator / digest path drift** — Attributes still calls the same four `editor_ops` mutators; heading commits through `number_field` as specified.
3. **T-818 Esc / modal_stack** — register / topmost / unregister still present; Class-R green.
4. **T-818 orphan Add-cargo as a second live editor** — dead `placed_vehicles_panel` is unreachable (F1 NIT only).
5. **T-819 reuse of T-701 `editorHidden` / materialize drops for crew** — assign body forbids those needles; filter is post-`materialize()` view; selection stays on `slots_json`.
6. **T-819 id-universe / selection prune of invisible crew** — selectable_ids still contains boarded ids; OBJ uses `slot_count` not filtered SoA len.
7. **T-819 z rewrite on board/unboard** — exact f64 pin holds; filter copies `zs` unchanged.
8. **T-819 hollow keep-filter pin** — intentional perturbation REDs; green path still drops two rows.
9. **T-819 completion only papering Class-R** — scrub split removes double-match landmine; T-802 pin now requires `map_render_slot_soa` and fails the hollow “still materialize()” story.
10. **T-836 wrong / missing seed alias** — all four SQL ResourceNames resolve; format matches prior `veh:` rows.
11. **T-836 hollow flatten test** — removing one alias makes the pin FAIL; restore returns green.
12. **HEAD drift after dispatch** — remained `00eea875` for the whole verification.
13. **Gate success without examining these surfaces** — standing §5 note applies to editor smokes, but these tickets’ Class-R / alias pins **did** examine the claimed code (not “green only because wave.sh never looked”).

---

## Environment left as found

- Repo: HEAD `00eea875`, no verifier commits, no ticket registry edits, no app source edits left behind.
- `packages/tbd-schema/registry/kit-aliases.json` restored byte-exact after hollow attack; `crates/map-engine-core/src/mission/kit.rs` touched only to force rebuild (mtime), content unchanged vs git.
- Ephemeral probe debris: `/tmp/w210-verify/**` — not in the repo.
- Pre-existing `:8080` API and `:3000` trunk left as found; verifier Chromium on `:9333` killed.
