# T-068.15.2 — Arsenal capacity + cargo UI

**Ticket:** T-068 · **Slice:** T-068.15.2 · **Status:** **SHIPPED** @ `4fb156b7` (tag **T-068.15.2**) ·
**Executor:** claude-code / Fable 5 ·
**Verify:** [`.ai/artifacts/t068_15_2_verify_log.md`](../../../.ai/artifacts/t068_15_2_verify_log.md) ·
**Authority:** [`t068_15_cargo_program.md`](t068_15_cargo_program.md) ·
**Depends on:** **T-068.15.1** (shipped)

---

## In one sentence

Show per-garment **max kg** + **grid W×H** in Leptos Arsenal (screenshot parity) and
edit/seed `SlotLoadoutV2.cargo` from `character_default_cargo` registry edges.

## Problem

T-068.10 Arsenal shows wear/weapons with weight totals but not **per-container capacity**
(max kg + inventory grid). `loadout-export` v2 has a forward `cargo[]` skeleton
(`{container, item, qty}`) but the editor does not surface it. Mission makers cannot see
whether a mag fits in the vest or seed default rifleman contents.

## Goal

1. When vest/pants/jacket/backpack is selected in Arsenal: show **max kg** + **grid W×H**
   from `GET /registry` (`max_weight_kg`, `cargo_grid_w`, `cargo_grid_h`).
2. Cargo list/grid editor bound to `SlotLoadoutV2.cargo[]` (container keys:
   `vest` / `pants` / `jacket` / `backpack` from TargetStorage first path segment).
3. **Seed on place / apply kit:** copy `character_default_cargo` edges for the character
   into slot cargo (≥1 mag + ≥1 medical for US rifleman when data exists).
4. Persist cargo through Save Version, mission Export, and undo (same doc path as T-068.10).

## Out of scope

- Compile flatten (**T-068.11**)
- Mod equip / `InsertItem` (**T-068.12**)
- `ammo_in_mag` round-count UI
- Cargo budget solver beyond display warnings (warn-only OK)

## Locked decisions

| # | Decision |
|---|----------|
| 1 | Capacity numbers come **only** from registry export (T-068.15.1) — never hard-coded |
| 2 | Cargo shape matches `loadout-export.schema.json` v2 `cargo[]` |
| 3 | Container keys align with wear map keys (`vest`, `pants`, `jacket`, `backpack`) |
| 4 | Seed is idempotent on first open — user edits after seed are preserved |
| 5 | UI lives in **Leptos** `apps/website/frontend` — not the deleted React tree |

## Tasks

1. Extend registry DTO / client to surface `cargo_grid_w`, `cargo_grid_h`.
2. Arsenal context: capacity readout when a container garment is active (`arsenal.rs`).
3. Cargo editor UI (list minimum; grid optional if parity needs cells).
4. `seedCargoFromCharacterDefaults(characterRn, loadout)` using `/registry/compat` edges.
5. Wire place-slot + apply-kit paths to seed cargo.
6. Tests: seed produces expected cargo rows for golden character fixture.
7. `.ai/artifacts/t068_15_2_verify_log.md` + tag **T-068.15.2**.

## Verify

```bash
cargo xtask ci schema-validate
cargo xtask mk ci-local-leptos
# Manual: dev-login → Arsenal → select vest → see max kg + grid; place rifleman → cargo seeded
```

## Acceptance

- [ ] Capacity visible per container garment (max kg + W×H)
- [ ] Cargo editable + seeded from `character_default_cargo` defaults
- [ ] Save / Export round-trip `cargo[]` on slot loadout
- [ ] Tag **T-068.15.2**

## Depends on / Unblocks

- **Depends on:** T-068.15.1 (export fields + edges in DB)
- **Unblocks:** T-068.11 (compiler includes `cargo[]`), T-068.12 (equip cargo)

---

## Claude Code prompt — T-068.15.2 (copy-paste)

Authority: this spec + program handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.15.2** — Arsenal capacity + cargo UI + seed (Leptos).

═══ PREFLIGHT ═══
  cd /run/media/system/Disk_2/Projects/TBD-Reforger
  git rev-parse T-068.15.1   # must exist
  ./scripts/ticket brief T-068

═══ READ ═══
  1. .ai/artifacts/t068_15_fable_program_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t068_15_2_arsenal_cargo_ui.md
  3. .ai/artifacts/t068_15_1_verify_log.md
  4. apps/website/frontend/src/arsenal.rs
  5. apps/website/frontend/src/arsenal_rules.rs
  6. apps/website/frontend/src/dto.rs
  7. packages/tbd-schema/schema/loadout-export.schema.json  (cargo[])
  8. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Registry now has capacity + character_default_cargo; Arsenal does not show grids or
  let makers edit/seed cargo into SlotLoadoutV2.

═══ LOCKED ═══
  - Capacity only from registry — never hard-code kg/grid
  - Container keys: vest/pants/jacket/backpack
  - Seed idempotent; preserve user edits after seed
  - Leptos only (apps/website/frontend)

═══ DO ═══
  1. Surface cargo_grid_* on registry DTO/client
  2. Capacity readout in Arsenal for container garments
  3. Cargo editor bound to SlotLoadoutV2.cargo[]
  4. Seed from character_default_cargo on place/apply-kit
  5. Persist via existing loadout save/undo path
  6. Tests + verify log + tag T-068.15.2
  7. Continue to T-068.11 per program handoff

═══ DO NOT ═══
  - Re-export registry / invent capacity numbers
  - Compile flatten (T-068.11) or mod equip (T-068.12) before 15.2 gates
  - Edit docs/registry
  - Touch deleted React apps/website/frontend npm tree patterns

═══ VERIFY ═══
  cargo xtask ci schema-validate
  cargo xtask mk ci-local-leptos
  .ai/artifacts/t068_15_2_verify_log.md

═══ RETURN ═══
  SHA + tag T-068.15.2 · then start T-068.11
```
