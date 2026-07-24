# T-068.11 — compiled mod document loadout block · verify log

**Date:** 2026-07-24 · **Executor:** Fable 5 / Claude Code · **Branch:** `main` ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_11_compiler_loadout_export.md` ·
**Handoff:** `.ai/artifacts/t068_11_claude_code_handoff.md` (Rust flatten only — no Go/TS/React) ·
**Prereqs:** tags **T-068.15.1** @ `85acbb13` · **T-068.15.2** @ `4fb156b7`

## Result

**PASS.** `GET /missions/:id/compiled` now carries an optional per-slot `loadout`
block — `gear` (six fixed ResourceName fields) + `cargo[]` (verbatim) — flattened
from the editor `SlotLoadoutV2`. Proven end-to-end against the live route.

## What shipped

- **`mission.schema.json` `$defs/slot`** += optional `loadout {gear{primary,optic,magazine,
  uniform,vest,helmet — minLength 1, additionalProperties:false}, cargo[{container,item,qty}]}`.
  **No schemaVersion bump** (locked reasoning recorded in the schema description): the mod
  loader allowlists `"1.1"/"1.2"` (`TBD_MissionLoader.c:378`) and JsonLoadContext ignores
  unknown keys, so the block is additive for deployed mods; the kit alias stays the base
  character — loadout layers on top.
- **`crates/map-engine-core/src/mission/flatten.rs`**: `ModSlotLoadout` / `ModSlotGear` /
  `ModSlotCargo` output types; `SlotIn` += `loadout: Option<Value>`; `mod_slot_loadout`
  mapper with the locked derivation (jacket→uniform; **armoredVest else vest**→vest;
  headCover→helmet; `weapons[]` slotIndex 0 + slotType "primary" → primary/optic/magazine).
  Empty policy = **omit** (the `ModSlot.y` precedent): empty strings drop, `gear` omitted when
  all six empty, `cargo` omitted when empty, whole `loadout` omitted when both absent;
  malformed cargo rows (empty strings / qty < 1) drop — same tolerance as the editor.
- The API adapter (`mission_compile.rs`) needed no code change — the block flows through the
  shared core flatten; its G6 test fixture + assertions extended.

## Automated gates (all exit 0)

```bash
make schema-validate                         # golden missions PASS with the new optional block
cargo test -p map-engine-core --all-features # 3/3 flatten tests (mission mod is feature-gated):
                                             #   flatten_matches_locked_contract (gear+cargo,
                                             #   omission on the wire, qty 40 verbatim)
                                             #   slot_loadout_mapper_edge_cases (vest fallback,
                                             #   non-primary ignored, qty<1 drop, cargo-only)
                                             #   empty_editor_is_no_slots
cargo test --lib services::mission_compile   # 2/2 — G6 validate_mission_document PASSES with
                                             #   a loadout-bearing compiled doc
make test-it                                 # full IT suite PASS (fresh rust_it DB)
make ci-local-leptos                         # wasm rebuild of map-engine-core + 94 tests + trunk PASS
```

## Live E2E receipt (real route, service token)

`POST /missions` → `POST …/versions` (editor payload with a loadout slot; editor schema's
`slots` is untyped — Save Version accepts) → `GET /api/v1/missions/6d291619-…/compiled`
with `X-Service-Token` returned:

```json
{ "schemaVersion": "1.1",
  "slots": [ { "id": "blufor:Alpha:RFL:0", "kit": "kit:us_rifleman",
    "x": 4839.2, "z": 6620.8, "headingDeg": 270.0,
    "loadout": {
      "gear": { "primary": "res://m16", "optic": "res://acog", "magazine": "res://stanag",
                "uniform": "res://bdu", "vest": "res://pasgt", "helmet": "res://helmet" },
      "cargo": [ { "container": "vest",  "item": "res://stanag",   "qty": 4 },
                 { "container": "pants", "item": "res://morphine", "qty": 2 } ] } } ] }
```

armoredVest (`res://pasgt`) correctly beat the chest rig; schemaVersion stayed 1.1;
loadout-less slots serve no `loadout` key (wire-asserted in tests).

## Known limitations (explicit)

- Gear carries the six locked v1 fields (spec sentence + loadout-export derivation);
  pants/boots/backpack/handwear wear picks reach the mod later via cargo containers only —
  widening the gear block is a future spec decision, not this slice.

## Ready for Cursor

Registry/status/doc sync per this log. Tag: **T-068.11**.
