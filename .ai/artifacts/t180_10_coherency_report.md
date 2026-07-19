# T-180.10 — Program coherency report

**Date:** 2026-07-19  
**Auditor:** Cursor (cursor-docs)  
**Code tip through:** T-180.9 @ `cba837b3`  
**Spec:** [`t180_10_program_coherency.md`](../../docs/specs/Mission_Creator_Architecture/t180_10_program_coherency.md)

## Verdict

**Code locks L1–L10 / Class-R A–I: PASS.** No P0 code remediations → **no T-180.10.1**.  
**Doc/queue drift: P1** — remediations are in-scope for this slice (hub/pins/ROADMAP/agent_execution/t071 + absorb T-071/074/147).  
**Residual (not FAIL):** operator manuals M-C1…M-I3; L8 Standardization (operator deferred).

---

## GPU bind (closes pre-read UNKNOWN)

| Path | Evidence | Status |
|------|----------|--------|
| Squad links upload | [`mission_history.rs`](../../apps/website/frontend/src/mission_history.rs) `upload_squad_links` → `e.upload_hairline_segments(ROLE_SQUAD_LINKS=9, …)` on rebind + `after_doc_change` | **OK** |
| Vehicles bind | same file: `e.vehicles_bind(&doc.vehicle_xy_flat())` after slots/tints | **OK** |
| Lane order | `LaneRole::SquadLinks` / `MissionVehicles` in `map-engine-render` draw_order; test `mission_vehicles_sit_between_squad_links_and_slots` PASS | **OK** |

---

## L1–L10 lock matrix

| Lock | Claim | Evidence | Status |
|------|-------|----------|--------|
| L1 | BLUFOR/OPFOR/INDFOR only | `EDEN_SIDE_CHIPS` exact; no CIV chip; template filter excludes CIV | **OK** |
| L2 | Named factions = T-153 templates | Apply/Save via Faction Library (T-180.8) | **OK** |
| L3 | SL = `leaderSlotId` not tag | `set_leader` + SL badge tests | **OK** |
| L4 | Place → mint new squad | `place_character_under_side`; `two_places_two_squads_same_side` | **OK** |
| L5 | Empty squad GC | `empty_squad_garbage_collected` | **OK** |
| L6 | Objects stub | `OBJECTS_COMING_SOON`; objects_mode gates place | **OK** |
| L7 | Leader→member lines only | `squad_link_*` tests; no peer | **OK** |
| L8 | Standardization deferred | `rg` Standardization/IFAK → 0 in orbat_manager/eden_chrome | **OK** (omit) |
| L9 | No F1–F6 | Dock chips only; FE tests | **OK** |
| L10 | T-080 sync out | No Eden module/trigger sync in T-180 scope | **OK** |

---

## Class-R A–I (re-run 2026-07-19)

| Slice | Gates | Result |
|-------|-------|--------|
| .1 A | `place_*` / `slot_callsign` / no `ensure_default_squad` | **PASS** |
| .2 B | `set_leader` / GC / `move_slot` / `leader_invariant` / `attach_vehicle` | **PASS** |
| .3 C | `side_tint_three_distinct` RGBA pins | **PASS** |
| .4 D | `squad_link_*` (6) | **PASS** |
| .5 E | `eden_side_chips_labels_no_civ` + chip wiring | **PASS** |
| .6 F | refile→`move_slot_to_squad`; `orbat_includes_two_squads_*`; SL badge | **PASS** |
| .7 G | `g1_dialog_class_near_fullscreen`; `format_slot_line_*`; no Standardization | **PASS** |
| .8 H | `apply_faction_*` (5); `pack_vehicle_instances`; FE Apply cancel / CIV filter / add vehicle | **PASS** |
| .9 I | `derive_fills_*` / `compile_export_orbat_loadout` / `open_arsenal_selects_arsenal_tab`; no `String::new()` hardcode | **PASS** |

### Real paths (vs older “mission/…” prose)

| Concept | Live path |
|---------|-----------|
| Place | `crates/map-engine-core/src/doc/place_orbat.rs` |
| Apply | `crates/map-engine-core/src/doc/apply_faction.rs` |
| Links | `crates/map-engine-core/src/squad_links.rs` |
| Slot line | `crates/map-engine-core/src/slot_line.rs` |
| Derive | `crates/map-engine-core/src/mission/orbat.rs` |
| FE package | **`website-frontend`** (bin tests; not `website-leptos`) |

---

## Findings

### P0 — code lock breaks

*None.*

### P1 — doc / queue drift (**REMEDIATED** in T-180.10)

| ID | Issue | Fix |
|----|-------|-----|
| P1-1 | Hub Gap table still pre-.1 | **Done** — post-ship SoT table |
| P1-2 | Pins “ABSENT must be added” | **Done** — Pre-ship baseline + Post-ship SoT |
| P1-3 | T-071 `ready`; T-074/T-147 `queued` | **Done** — T-071 `shipped`; T-074/T-147 `deferred` |
| P1-4 | ROADMAP ORBAT section ignores T-180 | **Done** |
| P1-5 | agent_execution Decisions stale | **Done** — T-180 COMPLETE decision |
| P1-6 | t071 hub still READY .1+ | **Done** — SUPERSEDED by T-180 |

### P2 — nits (no 10.1)

| ID | Issue | Disposition |
|----|-------|-------------|
| P2-1 | Spec prose sometimes said `mission/place_orbat.rs` | Paths corrected in pins/hub; historical verify logs OK |
| P2-2 | `attributes.rs` module header said Arsenal stub | **Fixed** (header reflects live Arsenal + `open_arsenal` tab 3) |
| P2-3 | Operator manuals M-* pending | Residual — not FAIL |
| P2-4 | T-168 summary pointed management at T-071 | **Fixed** (registry summary → T-180) |

---

## Cross-slice seams

| Seam | Status |
|------|--------|
| place → orbat_nodes + lines | OK (`after_local_edit` / `after_doc_change`) |
| refile → GC → lines | OK (`refile_slot` → core move) |
| Apply REPLACE → vehicles lane | OK (`apply_faction_library` + `vehicles_bind`) |
| Arsenal → derive → Export | OK (`loadout_summary_from_value` + compile test) |

---

## Permanent gate

`make verify-t180` → [`scripts/verify-t180-coherency.sh`](../../scripts/verify-t180-coherency.sh)
