# T-068.11 — Compiled mod document loadout block

**Ticket:** T-068 · **Slice:** T-068.11  
**Status:** **ready** · **Executor:** claude-code · **Baseline:** tag **T-068.10** @ `3bc0bd24` ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
**Feeds:** **T-068.12** (mod player equip on spawn)

---

## In one sentence

Put each slot’s Smart Forge **gear ResourceNames** onto the **compiled mod mission document**
(`GET /missions/:id/compiled` / `flattenEditorToModDocument`) so the mod can dress **human
players** in **T-068.12**.

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
2. **TS** `flattenEditorToModDocument` — copy slot loadout into compiled `slots[]`.
3. **Go** `FlattenToModDocument` (or equivalent) — same shape; keep TS↔Go parity tests.
4. Hydrate path already has editor loadout (T-068.10) — do **not** re-litigate editor embed.
5. Golden / unit tests: slot with loadout → compiled JSON contains gear; empty loadout → omit
   or null per locked decision.
6. Tag **T-068.11**. Cursor advances to **T-068.12**.

---

## Out of scope

- Mod player equip (**T-068.12**)
- Slot picker UI (**T-068.13**)
- Re-doing Arsenal / editor.slots loadout (done in **T-068.10**)
- Inventing `ammo_in_mag` edges

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Editor embed | **Already shipped** (T-068.10) — out of scope here |
| Compiled identity | Full Enfusion `resource_name` strings in gear fields |
| Empty gear | Omit empty/null fields (or explicit nulls — pick one, document, keep TS/Go identical) |
| Kit alias | Keep existing `kit` field; loadout **layers** on top (T-068.12) |
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
- [ ] TS and Go flatten agree.
- [ ] Empty loadout does not invent gear.
- [ ] Tag **T-068.11**.

---

## Claude Code prompt — T-068.11 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.11** — Compiled mod document loadout block (for T-068.12).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git pull && git lfs pull
  git rev-parse T-068.10   # expect 3bc0bd24

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t068_11_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t068_11_compiler_loadout_export.md
  3. .ai/artifacts/t068_10_verify_log.md  (editor embed already done — do not redo)
  4. apps/website/frontend/src/features/mission-creator/compiler/flattenModDocument.ts
  5. Go FlattenToModDocument / compiled mission handlers (T-092.2)
  6. packages/tbd-schema schema for mod/compiled mission
  7. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Editor slots carry loadout (T-068.10) but the mod-native compiled document still has no
  structured gear — T-068.12 cannot equip players from /compiled.

═══ SHIPPED (do not reopen) ═══
  T-068.10 — Arsenal + per-slot loadout in editor doc / Save / Export / ORBAT summary.
  T-092.2 — compiled flatten + GET /missions/:id/compiled.

═══ DO ═══
  - Schema: optional per-slot loadout gear on compiled mod slots
  - TS + Go flatten emit gear from editor slot.loadout
  - Parity tests + verify log
  - Tag T-068.11

═══ DO NOT ═══
  - Re-implement editor embed / Arsenal UI
  - Mod player equip (T-068.12)
  - Edit registry / CLAUDE / hub (Cursor)
  - Invent ammo edges

═══ VERIFY ═══
  cd packages/tbd-schema && npm run validate
  cd apps/website/frontend && npm test && npm run build && npm run lint
  make test-it

═══ RETURN ═══
  SHA + tag T-068.11
  Example compiled slot JSON snippet with gear
  Ready for Cursor: T-068.12
```
