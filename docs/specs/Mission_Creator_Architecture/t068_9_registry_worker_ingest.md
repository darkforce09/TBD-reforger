# T-068.9 — Registry worker + compat ingest

**Ticket:** T-068 · **Slice:** T-068.9  
**Status:** Spec ready — Phase 2  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Add `registry-compat.schema.json`, Postgres ingest, Comlink `registry.worker.ts`, and IDB cache with `canEquip` / `canAttach`.

---

## Problem

Dumb loadout cannot validate attachments; [`engineering_plan.md`](engineering_plan.md) Phase 5 worker not built.

---

## Goal

1. `packages/tbd-schema/schema/registry-compat.schema.json`
2. `registry_compat` migration + model + import path for T-068.8 export
3. `registry/registry.worker.ts` + `registryClient.ts` — Comlink; IDB cache by modpack version
4. Indices: compat adjacency for slot/attach queries
5. Expose `canEquip(classname, slotType)` / `canAttach(weapon, attachment)` to main thread
6. Integration test for ingest + worker init (unit/worker test as feasible)

---

## Out of scope

- Smart UI (T-068.10)
- Compiler export (T-068.11)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Schema file | Created here (not T-068.7) |
| Cache | IndexedDB keyed by modpack version |

---

## Tasks

1. Schema + golden compat fixture
2. Backend compat table + import
3. Worker + client + types under `frontend/src/features/mission-creator/registry/`
4. Tests

---

## Verify

```bash
cd packages/tbd-schema && npm run validate
PATH="$HOME/.local/go/bin:$PATH" make test-it
cd apps/website/frontend && npm run build && npm run lint
```

---

## Verification gate (mandatory)

### Automated (exit 0)

```bash
cd packages/tbd-schema && npm run validate   # includes registry-compat schema + fixture
PATH="$HOME/.local/go/bin:$PATH" make test-it
cd apps/website/frontend && npm run build && npm run lint
```

### Worker smoke (browser console)

| ID | Check | Pass condition |
|----|-------|----------------|
| W1 | Init | No worker crash on editor load |
| W2 | `canEquip` | Call with known-valid pair → `true` (paste I/O) |
| W3 | `canEquip` | Call with known-invalid pair → `false` |
| W4 | IDB | IndexedDB entry for modpack version created (Application tab screenshot) |

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Schema file | `registry-compat.schema.json` in validate |
| A2 | Backend ingest | Compat rows import without error |
| A3 | Tests | `make test-it` green |
| A4 | Worker API | W1–W3 documented with pasted results |
| A5 | Build | FE build/lint clean |

---

## Depends on / Unblocks

- **Depends on:** T-068.8
- **Unblocks:** T-068.10

---

## Claude Code prompt — T-068.9

```
Read CLAUDE.md §Status. Active slice: T-068.9.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_9_registry_worker_ingest.md
Do not edit documentation. Branch: ticket/T-068
Deliver: registry-compat.schema.json, Postgres ingest, registry.worker.ts, IDB cache.
Verify: npm run validate; make test-it; FE build/lint; W1–W3 console I/O pasted
Return: Verify paste A1–A5 + worker smoke table.
```
