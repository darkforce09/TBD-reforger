# T-068.10 — Smart Loadout Forge UI

**Ticket:** T-068 · **Slice:** T-068.10  
**Status:** **shipped** @ `3bc0bd24` (tag **T-068.10**) · **Executor:** claude-code ·
**Verify:** [`.ai/artifacts/t068_10_verify_log.md`](../../../.ai/artifacts/t068_10_verify_log.md) ·
**Next:** **T-068.11** (compiled mod loadout) · **Authority:**
[`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) · **Worker:**
`registryCompatClient.ts` (`initRegistryCompat` / `canAttach` / `canEquip` / `itemsFor`)

---

## In one sentence

Wire the Attributes **Arsenal** tab to the T-068.9 registry worker so invalid weapon/optic/mag
(and related) combos are **blocked in the UI** before `loadout-export.json` download — and
persist per-slot loadout in the mission document (operator scope expansion).

---

## Problem

Phase 1 dumb dropdowns (T-068.4) let mission makers pick impossible kits. The compat graph and
worker exist but are **unwired** — nothing imports `registryCompatClient.ts` yet.

---

## Goal

1. On Arsenal open (character slot): `await initRegistryCompat()` (modpack from registry /
   mission context).
2. Pickers filter or disable options via `itemsFor(host, edgeType?)` / `canAttach` /
   `canEquip` — Aegis error chip when blocked.
3. Export still downloads `loadout-export.json`; refuse or strip invalid pairs (document
   choice in verify log — prefer **block before download**).
4. Graceful degrade: if init fails / empty graph, keep Phase 1 dumb dropdowns + toast
   (“compat unavailable”).
5. Vitest for the bridge; manual M1–M3 in verify log.
6. Tag **T-068.10**. Cursor doc-sync after.

---

## Out of scope

- Compiler mission envelope (**T-068.11**)
- Asset Browser / map place (**T-146** / **T-070**)
- Inventing `ammo_in_mag` edges
- Full paper-doll Eden Forge (ship **validated pickers** on the existing Arsenal tab)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Validation | Worker client only (`registryCompatClient`) before export commit |
| UI home | Same Attributes → Arsenal tab (smart mode) |
| Data | Live API / IDB via `initRegistryCompat` (dev DB after `make registry-import`) |
| Docs/registry | Claude does **not** edit (verify log OK) |

---

## Tasks

1. Bridge hook (e.g. `useArsenalValidation`) → `initRegistryCompat` / `canAttach` / `itemsFor`.
2. Upgrade T-068.4 Arsenal dropdowns to filtered/validated picks.
3. Block invalid export; preserve degrade path.
4. Tests + `.ai/artifacts/t068_10_verify_log.md` + tag **T-068.10**.

---

## Verify

```bash
make db-up && make registry-import && make api   # separate terminal
cd apps/website/frontend && npm test && npm run build && npm run lint
```

Manual: invalid optic/mag blocked; valid loadout exports; worker init visible in verify notes.

---

## Acceptance

- [ ] Arsenal uses worker validation on character slots.
- [ ] Invalid combo cannot export (or clearly blocked in UI).
- [ ] Valid combo exports; degrade path if compat missing.
- [ ] Tag **T-068.10**.

---

## Claude Code prompt — T-068.10 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.10** — Smart Loadout Forge UI (wire T-068.9 worker into Arsenal).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git pull && git lfs pull
  git rev-parse T-068.9   # expect d41418e5
  make db-up && make registry-import

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t068_10_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t068_10_smart_forge_ui.md
  3. .ai/artifacts/t068_9_verify_log.md
  4. apps/website/frontend/src/features/mission-creator/registry/registryCompatClient.ts
  5. Attributes Arsenal UI (T-068.4) under mission-creator
  6. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Compat worker exists but Arsenal still uses dumb dropdowns — invalid kits can export.

═══ SHIPPED (do not reopen) ═══
  T-068.9 @ d41418e5 — ingest, API, initRegistryCompat/canAttach/canEquip/itemsFor.
  T-150 envelopes in DB via make registry-import.

═══ DO ═══
  - Wire Arsenal tab to registryCompatClient
  - Filter/block invalid picks; block bad export
  - Graceful degrade if init fails
  - .ai/artifacts/t068_10_verify_log.md + tag T-068.10

═══ DO NOT ═══
  - Edit registry / CLAUDE / hub (Cursor)
  - T-068.11 compiler, T-146 browser, T-070 vehicles
  - Invent ammo_in_mag edges
  - Rebuild the whole Attributes modal

═══ VERIFY ═══
  make registry-import
  cd apps/website/frontend && npm test && npm run build && npm run lint
  Manual M1–M3 (invalid blocked, valid exports, worker used)

═══ RETURN ═══
  SHA + tag T-068.10
  Ready for Cursor: T-068.11 / T-146
```
