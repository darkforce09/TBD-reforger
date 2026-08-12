# T-180 — ORBAT + Eden placement program (Class-R)

**Status:** SHIPPED (program complete + coherency) · **Last slice:** **T-180.10** · **Code tip:** **T-180.9** @ `cba837b3` · **Gate:** `cargo xtask verify t180`  


**Ticket:** T-180 · **Route:** `/missions/:id/edit` · **Branch:** `main`  
**Depends on:** T-177 / T-071.0 (ORBAT Manager shell) · T-153 (Faction Library) · T-151 (wgpu map)  
**Absorbs remaining intent of:** T-071.1+ · T-074 (side submode) · T-147 (squad leader lines) · vehicle bits of T-070  
**Stitch / Eden refs:** [`.ai/artifacts/t180_stitch_orbat_modal/`](../../../.ai/artifacts/t180_stitch_orbat_modal/)  
**Class-R pins (measured):** [`t180_class_r_pins.md`](t180_class_r_pins.md) — **wins on conflict** with older prose  
**Handoff:** [`.ai/artifacts/t180_claude_code_handoff.md`](../../../.ai/artifacts/t180_claude_code_handoff.md)  
**Plan authority (operator locks):** Cursor plan `orbat_manager_stitch_ux_5fd474fe`

**Shipped:**
- **T-180.1** @ `aeb51209` · **T-180.2** @ `83557768` · **T-180.3** @ `19acc593`
- **T-180.4** @ `63e7ef00` · **T-180.5** @ `1324799c` · **T-180.6** @ `056c9a1a`
- **T-180.7** @ `e9c2406d` · **T-180.8** @ `cce240a5` (tag **T-180.8**) — [verify](../../../.ai/artifacts/t180_8_verify_log.md)
- **T-180.9** @ `cba837b3` (tag **T-180.9**) — [verify](../../../.ai/artifacts/t180_9_verify_log.md) Open Arsenal + `derive_orbat` loadout
- **T-180.10** coherency — [report](../../../.ai/artifacts/t180_10_coherency_report.md) · [verify](../../../.ai/artifacts/t180_10_verify_log.md) · `cargo xtask verify t180`
- Placement pin (Apply/Add Vehicle): Everon `(6400, 6400)`
- Manuals pending (operator): M-C1 · M-D1 · M-E1/M-E2 · M-F1 · M-G1..M-G4 · M-H1..M-H4 · M-I1..M-I3

Doc-core tests: `cargo test -p map-engine-core --features doc …`.

---

## Stitch mock-up — yes, included

Operator Stitch pack was copied into the repo (not left only in `Downloads/`):

| File | Role |
|------|------|
| [`screen.png`](../../../.ai/artifacts/t180_stitch_orbat_modal/screen.png) | ORBAT Manager visual authority (tabs, tree, inspector, Apply, Open Arsenal) |
| [`code.html`](../../../.ai/artifacts/t180_stitch_orbat_modal/code.html) | Static structure / class names to port |
| [`DESIGN.md`](../../../.ai/artifacts/t180_stitch_orbat_modal/DESIGN.md) | Aegis tokens |
| [`eden_side_chips_ref.png`](../../../.ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png) | Eden right-panel chips ref (T-180.5) |

**Slice that implements the mock:** **T-180.7** — full region inventory (implement vs omit Standardization) lives in [`t180_7_orbat_manager_ui.md`](t180_7_orbat_manager_ui.md).  
Templates/vehicles chrome in the mock → **T-180.8**. Open Arsenal behavior → **T-180.9**.

---

## North star

One live mission graph — **Side → Squad → (Squad Leader + members + vehicles)** — authored from:

1. **Right dock** — Eden side chips (BLUFOR/OPFOR/INDFOR) + Objects stub + search → place  
2. **Map** — side-colored slots + lines **only** Squad Leader → each subordinate  
3. **ORBAT Manager** — Stitch modal; templates via T-153 Faction Library  

Save Version / Export / Event attach see the **same** structure. **No mock ORBAT.**

**Process:** one slice at a time. Class-R gates must **fail if the feature is stubbed**. Do not start slice N+1 until N verify log is PASS.

---

## Locked decisions (operator 2026-07-19)

| ID | Lock |
|----|------|
| L1 | Sides = **BLUFOR / OPFOR / INDFOR** only (no CIV in this UI) |
| L2 | Named factions (e.g. US 1980s) = T-153 templates under a side |
| L3 | Squad Leader = **user-chosen**; UX = **SL badge**; SoT = `squad.leaderSlotId` (does not overwrite `tag` MED/ENG) |
| L4 | Place with side chip active → **always mint a new squad** under that side; sole member = leader |
| L5 | Empty squad (0 slots) → **delete** |
| L6 | Objects yellow chip = **in** (stub until sandbags/props); not vehicles |
| L7 | Map lines = leader → members only; side-colored; no peer lines |
| L8 | Standardization (IFAK/grenades) = **deferred** — operator: arsenal inventory later |
| L9 | No F1–F6 Eden mode row |
| L10 | Classic T-080 Eden sync-to-module/trigger = **out** (different feature; not squad lines) |

---

## Target data model

```text
Faction { id, key/side: "BLUFOR"|"OPFOR"|"INDFOR", name, squadIds[] }
Squad   { id, factionId, name, callsign?, slotIds[], leaderSlotId, vehicleIds[] }
Slot    { id, squadId, index, role, tag?, callsign?, rank?, assetId?, loadout?, position{x,y,z,rotation}, stance }
Vehicle { id, squadId?, resource/assetId, label?, position? }
```

**Where it lives today (post-ship measured — T-180.1–.9):**

| Concept | Live path | Status |
|---------|-----------|--------|
| Faction | `faction-{SIDE}` rows; `FactionRow.key` preserved | **closed** (.1) |
| Squad | `leaderSlotId` + `vehicleIds` + mutators/GC | **closed** (.1/.2) |
| Slot | `callsign` / `rank` + embedded `loadout` | **closed** (.1/.9) |
| Place | [`doc/place_orbat.rs`](../../../crates/map-engine-core/src/doc/place_orbat.rs) → mint squad under `active_side`; `ensure_default_squad` **gone** | **closed** (.1/.5) |
| Ring tint | [`slots_gpu.rs`](../../../crates/map-engine-core/src/slots_gpu.rs) SIDE_* RGBA | **closed** (.3) |
| Map lines | [`squad_links.rs`](../../../crates/map-engine-core/src/squad_links.rs) + `mission_history` upload role 9 | **closed** (.4) |
| ORBAT UI | [`orbat_manager.rs`](../../../apps/website/frontend/src/orbat_manager.rs) Stitch shell + live mutators | **closed** (.7) |
| Dock | Eden chips BLUFOR/OPFOR/INDFOR/Objects | **closed** (.5) |
| Templates / vehicles | [`doc/apply_faction.rs`](../../../crates/map-engine-core/src/doc/apply_faction.rs) REPLACE + MissionVehicles lane | **closed** (.8) |
| Compile | [`orbat.rs`](../../../crates/map-engine-core/src/mission/orbat.rs) `loadout_summary_from_value` | **closed** (.9) |

**Residual (not code gaps):** operator manuals M-C1…M-I3 · L8 Standardization (operator deferred) · Event lobby polish **T-118** · faction logos (not in T-180).

---

## Slice ladder

| Slice | Title | Spec | Executor | Status |
|-------|-------|------|----------|--------|
| **T-180.0** | Program hub + Class-R pins + slice specs | this file + [`t180_class_r_pins.md`](t180_class_r_pins.md) | cursor-docs | **shipped (docs)** |
| **T-180.1** | Foundation: leaderSlotId, sides, callsign/rank, place→new squad | [`t180_1_foundation_schema.md`](t180_1_foundation_schema.md) | claude-code | **SHIPPED** @ `aeb51209` |
| **T-180.2** | Graph mutators + empty-squad GC | [`t180_2_graph_mutators.md`](t180_2_graph_mutators.md) | claude-code | **SHIPPED** @ `83557768` |
| **T-180.3** | Map side tint | [`t180_3_map_side_tint.md`](t180_3_map_side_tint.md) | claude-code | **SHIPPED** @ `19acc593` |
| **T-180.4** | Map leader→member lines | [`t180_4_squad_leader_lines.md`](t180_4_squad_leader_lines.md) | claude-code | **SHIPPED** @ `63e7ef00` |
| **T-180.5** | Right dock Eden chips + Objects stub | [`t180_5_right_dock_side_chips.md`](t180_5_right_dock_side_chips.md) | claude-code | **SHIPPED** @ `1324799c` |
| **T-180.6** | Place/refile ↔ ORBAT live sync | [`t180_6_place_orbat_sync.md`](t180_6_place_orbat_sync.md) | claude-code | **SHIPPED** @ `056c9a1a` |
| **T-180.7** | Stitch ORBAT Manager UI | [`t180_7_orbat_manager_ui.md`](t180_7_orbat_manager_ui.md) | claude-code | **SHIPPED** @ `e9c2406d` |
| **T-180.8** | Templates + vehicles | [`t180_8_templates_vehicles.md`](t180_8_templates_vehicles.md) | claude-code | **SHIPPED** @ `cce240a5` |
| **T-180.9** | Arsenal wire + compile truth | [`t180_9_arsenal_compile.md`](t180_9_arsenal_compile.md) | claude-code | **SHIPPED** @ `cba837b3` |
| **T-180.10** | Program coherency checker | [`t180_10_program_coherency.md`](t180_10_program_coherency.md) | cursor-docs | **SHIPPED** |

---

## Master Class-R gate table (fail if stubbed)

Full pin ledger: [`t180_class_r_pins.md`](t180_class_r_pins.md). Per-slice specs own the full ID lists; this is the **minimum** merge checklist.

| Slice | Gate IDs | Primary commands (package names exact) |
|-------|----------|----------------------------------------|
| .1 | A1–A7 | `cargo test -p map-engine-core place_character_under_side_opfor` · `two_places_two_squads_same_side` · `place_rejects_invalid_side` · `slot_callsign_rank_roundtrip` · `rg ensure_default_squad` → 0 · `cargo test -p website-frontend` |
| .2 | B1–B7 | `cargo test -p map-engine-core set_leader_exclusive empty_squad_garbage_collected move_slot_bidirectional leader_invariant_holds move_leader_promotes_next attach_vehicle_roundtrip slot_indices_dense_after_move` |
| .3 | C1–C4 | `cargo test -p map-engine-core side_tint_three_distinct` — assert RGBA **`[173,198,255,255]` / `[248,113,113,255]` / `[34,197,94,255]`** · selected `[250,204,21,255]` |
| .4 | D1–D6 | `cargo test -p map-engine-core squad_link_` — size N ⇒ N−1 segs; no peer; solo 0 |
| .5 | E1–E5 | `cargo test -p website-frontend` dock chip tests · `active_side` OPFOR · Objects empty · no F1–F6 · no CIV |
| .6 | F1–F5 | `cargo test -p map-engine-core refile_gc` · place×2 ⇒ 2 squads · line count after merge |
| .7 | G1–G8 | near-fullscreen ≠ max-w-xl-only · `format_slot_line_*` · `set_leader` · no Standardization · no hardcoded Stitch L85A3 as SoT |
| .8 | H1–H9 | `cargo test -p map-engine-core apply_faction_` · `add_vehicle` exists · replace-not-merge · `cargo xtask db test-it` |
| .9 | I1–I9 | `derive_fills_loadout_from_summary` · no `String::new()` hardcode · invert `orbat.rs:174` · `open_arsenal` → Attributes tab **3** · `compile.rs:248` fixtures updated |
| .10 | J1–J7 | `cargo xtask verify t180` · coherency report · hub/pins/ROADMAP absorb cleanup |

---

## Code ownership map (where to write)

### Rust — `map-engine-core` (graph SoT)

| Path | Owns |
|------|------|
| [`crates/map-engine-core/src/doc/store.rs`](../../../crates/map-engine-core/src/doc/store.rs) | `leaderSlotId`, callsign/rank on slots, vehicleIds, set_leader, move_slot, GC, rename/reorder |
| [`crates/map-engine-core/src/doc/soa.rs`](../../../crates/map-engine-core/src/doc/soa.rs) | SoA fields if needed for tint/lines |
| [`crates/map-engine-core/src/slots_gpu.rs`](../../../crates/map-engine-core/src/slots_gpu.rs) | Per-side RGBA pack |
| **NEW** `crates/map-engine-core/src/mission/squad_links.rs` (or equiv) | Pure: segments from (leader, members, xy) → `Vec<(x0,y0,x1,y1,rgba)>` |
| [`crates/map-engine-core/src/mission/orbat.rs`](../../../crates/map-engine-core/src/mission/orbat.rs) | Compile loadout/callsign fill |

### Rust — `map-engine-render` (GPU draw)

| Path | Owns |
|------|------|
| [`crates/map-engine-render/src/engine.rs`](../../../crates/map-engine-render/src/engine.rs) (~4399 hairline upload) | Upload squad-link LineList lane each frame / on dirty |

### Leptos — website frontend

| Path | Owns |
|------|------|
| [`apps/website/frontend/src/editor_ops.rs`](../../../apps/website/frontend/src/editor_ops.rs) | `place_at` / active side / mint squad; delete `ensure_default_squad` path |
| [`apps/website/frontend/src/eden_chrome.rs`](../../../apps/website/frontend/src/eden_chrome.rs) | `DockRight` chips; `OrbatManagerDialog` Stitch UI |
| [`apps/website/frontend/src/mission_editor.rs`](../../../apps/website/frontend/src/mission_editor.rs) | Wire active_side signal; mount dialog |
| [`apps/website/frontend/src/asset_catalog.rs`](../../../apps/website/frontend/src/asset_catalog.rs) | Filter catalog by side |
| [`apps/website/frontend/src/arsenal.rs`](../../../apps/website/frontend/src/arsenal.rs) | Open from ORBAT inspector; slot-line weapon text |
| [`apps/website/frontend/src/faction_manager.rs`](../../../apps/website/frontend/src/faction_manager.rs) / client | Apply/Save templates (T-153 API) |
| [`apps/website/frontend/src/outliner.rs`](../../../apps/website/frontend/src/outliner.rs) | `build_orbat` if needed for SL badge |

### Explicitly do not touch (unless slice says)

- `apps/mod/**` (until a later mod-facing slice)  
- Docs/registry from Claude Code  
- Standardization UI (L8)

---

## Deferred (operator-authorized only)

1. Standardization IFAK/grenade complements — until arsenal inventory  
2. Real Objects catalog content (sandbags) — chip shell only  
3. Eden module/trigger/waypoint sync (classic T-080 Syncing)

---

## Agent split

| Agent | Owns |
|-------|------|
| **Cursor** | Specs, registry, handoffs, verify-log templates, CLAUDE status sync |
| **Claude Code / Grok** | App + crate code for `executor: claude-code` slices only |

---

## Relationship to T-071 / T-074 / T-147

- **T-071.0** remains shipped via T-177.  
- **T-071.1–.4** intent is **superseded by T-180.1–.9** (do not implement thin T-071.1 in parallel).  
- **T-074** / **T-147** absorbed into T-180.5 / T-180.4.  
- Update those registry rows to point here when syncing.
