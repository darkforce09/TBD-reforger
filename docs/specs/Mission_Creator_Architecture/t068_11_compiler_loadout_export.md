# T-068.11 — Compiled mod document loadout block

**Ticket:** T-068 · **Slice:** T-068.11  
**Status:** **SHIPPED** @ `c66494c6` (tag **T-068.11**) · **Executor:** claude-code / Fable 5 ·
**Verify:** [`.ai/artifacts/t068_11_verify_log.md`](../../../.ai/artifacts/t068_11_verify_log.md) ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
**Feeds:** **T-068.12** (shipped)

---

## In one sentence

Put each slot’s Smart Forge **gear ResourceNames** and **cargo[]** onto the **compiled mod mission document**
(`GET /missions/:id/compiled` / `flattenEditorToModDocument`) so the mod can dress **human
players** and fill container storages in **T-068.12**.

---

## Problem

**T-068.10** already persists per-slot `loadout` in the **editor** doc (Save Version,
mission Export `editor.slots[].loadout`, ORBAT display summary, IDB, undo). The **mod-native
compiled** document (`flattenEditorToModDocument` / Go flatten) still emits slots with only
`kit` alias + transform — **no structured gear block**. T-068.12 cannot equip players from
compiled JSON until that block exists.

---

## Goal

1. Extend mod mission / slot schema (packages/tbd-schema) with optional per-slot loadout gear
   (ResourceName strings: primary, optic, magazine, uniform, vest, helmet — align with
   `loadout-export` / T-068.10 slot loadout shape).
2. Extend compiled slot loadout with optional **`cargo[]`** array (`{container, item, qty}` per
   `loadout-export.schema.json` v2) — populated from editor `SlotLoadoutV2.cargo` after
   **T-068.15.2**.
3. **TS** `flattenEditorToModDocument` — copy slot loadout (gear + cargo) into compiled `slots[]`.
4. **Go** `FlattenToModDocument` (or equivalent) — same shape; keep TS↔Go parity tests.
5. Hydrate path already has editor loadout (T-068.10) — do **not** re-litigate editor embed.
6. Golden / unit tests: slot with loadout → compiled JSON contains gear **and cargo** when set;
   empty loadout → omit or null per locked decision.
7. Tag **T-068.11**. Cursor advances to **T-068.12**.

---

## Out of scope

- Mod player equip / `InsertItem` cargo (**T-068.12**)
- Slot picker UI (**T-068.13**)
- Re-doing Arsenal / editor.slots loadout (done in **T-068.10**)
- Cargo export plugin (**T-068.15.1**) or Arsenal cargo UI (**T-068.15.2**) — must ship first
- Inventing `ammo_in_mag` edges

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Editor embed | **Already shipped** (T-068.10) — out of scope here |
| Compiled identity | Full Enfusion `resource_name` strings in gear fields |
| Empty gear | Omit empty/null fields (or explicit nulls — pick one, document, keep TS/Go identical) |
| Kit alias | Keep existing `kit` field; loadout **layers** on top (T-068.12) |
| Cargo | `cargo[]` copied verbatim from editor; empty array omitted |
| Prereq | **T-068.15.1** + **T-068.15.2** must ship before this slice |
| Docs/registry | Claude does **not** edit (verify log OK) |

---

## Tasks

1. Schema bump for mod compiled slot loadout (codegen if required).
2. TS flatten + tests.
3. Go flatten + IT / parity.
4. `.ai/artifacts/t068_11_verify_log.md` + tag **T-068.11**.

---

## Verify

```bash
cd packages/tbd-schema && npm run validate
cd apps/website/frontend && npm test && npm run build && npm run lint
make test-it
# Optional: curl compiled mission after Save — slots include loadout gear when set
```

---

## Acceptance

- [ ] Compiled mod document includes per-slot gear when editor loadout is set.
- [ ] Compiled mod document includes per-slot `cargo[]` when editor cargo is set (T-068.15.2).
- [ ] TS and Go flatten agree (gear + cargo).
- [ ] Empty loadout does not invent gear or cargo.
- [ ] Tag **T-068.11**.

---

## Claude Code prompt — T-068.11 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.11** — Compiled mod document loadout block incl. cargo (for T-068.12).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git rev-parse T-068.15.1
  git rev-parse T-068.15.2
  ./scripts/ticket brief T-068

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t068_11_claude_code_handoff.md
  2. .ai/artifacts/t068_15_fable_program_handoff.md
  3. docs/specs/Mission_Creator_Architecture/t068_11_compiler_loadout_export.md
  4. crates/map-engine-core/src/mission/flatten.rs
  5. apps/website/api/src/services/mission_compile.rs
  6. packages/tbd-schema schema for mod/compiled mission + loadout-export cargo[]
  7. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Editor slots carry wear/weapons (T-068.10) and cargo (T-068.15.2) but compiled
  /missions/:id/compiled may still omit structured gear+cargo for the mod.

═══ SHIPPED (do not reopen) ═══
  T-068.10 — Arsenal + per-slot loadout in editor
  T-068.15.1 / T-068.15.2 — capacity export + Arsenal cargo UI
  T-092.2 — compiled flatten route (Rust)

═══ DO ═══
  - Schema: optional per-slot loadout gear + cargo[] on compiled mod slots
  - Rust flatten emits gear + cargo from editor slot.loadout
  - Tests + verify log
  - Tag T-068.11
  - Continue to T-068.12

═══ DO NOT ═══
  - Re-implement Arsenal UI
  - Mod player equip (T-068.12) before 11 gates
  - Edit docs/registry
  - Invent ammo edges
  - Revive Go/TS flatten paths

═══ VERIFY ═══
  make schema-validate
  make test-it
  make ci-local-leptos
  .ai/artifacts/t068_11_verify_log.md

═══ RETURN ═══
  SHA + tag T-068.11
  Example compiled slot JSON with gear + cargo
  Then start T-068.12
```
