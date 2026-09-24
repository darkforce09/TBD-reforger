# T-180.1 — Foundation schema + place→new squad

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) · **Executor:** claude-code  
**Verify log:** [`.ai/artifacts/t180_1_verify_log.md`](../../../.ai/artifacts/t180_1_verify_log.md)

## Problem (measured)

| Fact | Evidence |
|------|----------|
| Place dumps into one squad | [`editor_ops.rs:903-960`](../../../apps/website/frontend/src/editor_ops.rs) `ensure_default_squad` → `squad-1` |
| No leader | [`store.rs:324-336`](../../../crates/map-engine-core/src/doc/store.rs) — no `leaderSlotId` |
| No slot callsign/rank | [`store.rs:275-307`](../../../crates/map-engine-core/src/doc/store.rs) |
| FE drops faction `key` | [`editor_ops.rs:668-676`](../../../apps/website/frontend/src/editor_ops.rs) / [`outliner.rs:169-174`](../../../apps/website/frontend/src/outliner.rs) |
| No `active_side` | `OpsCtx` [`editor_ops.rs:50-71`](../../../apps/website/frontend/src/editor_ops.rs) |

## Locked

| ID | Decision |
|----|----------|
| A-L1 | Stable faction ids: `faction-BLUFOR` / `faction-OPFOR` / `faction-INDFOR` with `key` equal to side |
| A-L2 | Each `place_at` mints a **new** squad under that faction; never append to a random first squad |
| A-L3 | New sole member → `leaderSlotId = that slot` |
| A-L4 | Slot fields `callsign`/`rank` optional strings (omit when empty, same as `tag`) |
| A-L5 | Pure helper in **`map-engine-core`** (testable without wasm); `place_at` calls it |
| A-L6 | `FactionRow.key` plumbed; `faction_rows` preserves `key` |
| A-L7 | `active_side: RwSignal<&'static str or String>` default `"BLUFOR"` in OpsCtx |
| A-L8 | Empty-squad GC deferred to **T-180.2** |
| A-L9 | Delete/stop using `ensure_default_squad` as place path; `DEFAULT_SQUAD_ID` must not appear in `place_at` |

## File map

| File | Change |
|------|--------|
| `crates/map-engine-core/src/doc/store.rs` | `leaderSlotId` on `add_squad` / `set_leader`; `update_slot_identity` or extend update for callsign/rank |
| **NEW** `crates/map-engine-core/src/mission/place_orbat.rs` (name OK if equivalent) | `ensure_side_faction` + `mint_squad_with_leader_slot` OR single `place_slot_under_side(...)` used by tests |
| `crates/map-engine-core/src/lib.rs` | export module |
| `apps/website/frontend/src/editor_ops.rs` | `active_side` on OpsCtx; `place_at` → core helper; remove place use of `ensure_default_squad` |
| `apps/website/frontend/src/mission_editor.rs` | create/pass `active_side` |
| `apps/website/frontend/src/outliner.rs` | `FactionRow { key, … }` |
| `apps/website/frontend/src/editor_ops.rs` `faction_rows` | read `key` |

## Helper contract (Class-R)

```rust
/// Ensures faction-{SIDE} exists (id+key+name=SIDE). Mints unique squad id under it.
/// Adds slot; sets leaderSlotId = slot_id. Returns (faction_id, squad_id, slot_id).
pub fn place_character_under_side(
    doc: &MissionDocCore,
    side: &str,           // "BLUFOR"|"OPFOR"|"INDFOR" only — Err otherwise
    slot_id: &str,
    layer_id: &str,
    role: &str,
    tag: Option<String>,
    asset_id: Option<String>,
    x: f64, y: f64, z: f64, rotation: f64,
) -> Result<(String, String, String), PlaceOrbatError>;
```

`place_at` mints `slot_id`, resolves layer, reads `active_side`, calls this, then selection/after_local_edit.

## Class-R gates (must fail if stubbed)

| ID | Assert | Command |
|----|--------|---------|
| **A1** | `place_character_under_side(…, "OPFOR", …)` ⇒ faction id `faction-OPFOR`, key `OPFOR`, squad id ≠ `squad-1`, `slotIds==[slot]`, `leaderSlotId==slot` | `cargo test -p map-engine-core place_character_under_side_opfor` |
| **A2** | Write callsign+rank; read back via slots_json / get | `cargo test -p map-engine-core slot_callsign_rank_roundtrip` |
| **A3** | `rg -n 'ensure_default_squad' apps/website/frontend/src/editor_ops.rs` exits **1** (no matches) | shell |
| **A4** | Two calls same side ⇒ two distinct squad ids; faction.squadIds.len()==2 | `cargo test -p map-engine-core two_places_two_squads_same_side` |
| **A5** | `side="CIV"` or `"nope"` ⇒ Err; no mutation | `cargo test -p map-engine-core place_rejects_invalid_side` |
| **A6** | `faction_rows` / FactionRow includes key `"BLUFOR"` after mint | `cargo test -p website-frontend faction_rows_preserves_key` **or** core JSON assert in A1 |
| **A7** | `rg -n 'DEFAULT_SQUAD_ID' apps/website/frontend/src/editor_ops.rs` — if present, must **not** be referenced from `place_at` (verify log quotes `place_at` body) | manual+rg in verify log |

## Verify / Rebuild

```bash
cargo test -p map-engine-core place_character_under_side_opfor
cargo test -p map-engine-core slot_callsign_rank_roundtrip
cargo test -p map-engine-core two_places_two_squads_same_side
cargo test -p map-engine-core place_rejects_invalid_side
# A3 — must find ZERO matches:
if rg -n 'ensure_default_squad' apps/website/frontend/src/editor_ops.rs; then echo 'A3 FAIL'; exit 1; fi
cargo test -p website-frontend --lib
cargo xtask mk ci-local-leptos
```

## Acceptance

- [ ] A1–A7 PASS with pasted command output in verify log  
- [ ] Tag **T-180.1**

## Claude Code prompt — T-180.1 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.1** — Foundation schema + place→new squad.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md
  4. docs/specs/Mission_Creator_Architecture/t180_1_foundation_schema.md
  5. crates/map-engine-core/src/doc/store.rs:275-336
  6. apps/website/frontend/src/editor_ops.rs:41-48,668-676,903-960

═══ PROBLEM ═══
  place_at → ensure_default_squad (squad-1). No leaderSlotId. No callsign/rank.
  FE faction_rows drops key. Extract core helper so gates are cargo-testable.

═══ LOCKED ═══
  - faction-BLUFOR|OPFOR|INDFOR stable ids + key
  - place mints NEW squad; sole member = leader
  - helper in map-engine-core; place_at calls it
  - FactionRow.key required
  - Package website-frontend NOT website-leptos
  - No Stitch/map lines/chips UI (active_side default BLUFOR OK)

═══ DO ═══
  1. store leaderSlotId + set_leader + callsign/rank
  2. place_character_under_side (+ tests A1,A2,A4,A5)
  3. Wire place_at + active_side; delete ensure_default_squad place path (A3)
  4. Plumb FactionRow.key (A6)
  5. .ai/artifacts/t180_1_verify_log.md · tag T-180.1

═══ DO NOT ═══
  Docs/registry · Stitch chrome · keep ensure_default_squad as place path
  · put place rules only in Leptos without core tests

═══ VERIFY ═══
  cargo test -p map-engine-core place_character_under_side_opfor
  cargo test -p map-engine-core slot_callsign_rank_roundtrip
  cargo test -p map-engine-core two_places_two_squads_same_side
  cargo test -p map-engine-core place_rejects_invalid_side
  if rg -n 'ensure_default_squad' apps/website/frontend/src/editor_ops.rs; then exit 1; fi
  cargo test -p website-frontend --lib
  cargo xtask mk ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.1 · verify log · Ready for T-180.2
```
