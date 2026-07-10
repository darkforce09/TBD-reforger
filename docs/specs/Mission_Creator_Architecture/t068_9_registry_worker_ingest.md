# T-068.9 — Registry worker + compat ingest

**Ticket:** T-068 · **Slice:** T-068.9  
**Status:** **shipped** @ `d41418e5` (tag **T-068.9**) · **Executor:** claude-code ·
**Verify:** [`.ai/artifacts/t068_9_verify_log.md`](../../../.ai/artifacts/t068_9_verify_log.md) ·
**Next:** **T-068.10** · **Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)
· **Upstream:** [`t150_universal_registry_export.md`](t150_universal_registry_export.md)

---

## In one sentence

Ingest T-150 **items + compat** envelopes into Postgres, expose them on the API, and ship a
Comlink **registry worker** with IDB cache + `canEquip` / `canAttach` over the live edge graph.

---

## Problem

T-150 committed **1,880 items** + **4,012 edges** under `packages/tbd-schema/registry/`, but the
website still serves the Phase 1 thin seed / flat items only. Smart arsenal and vehicle ammo
questions need DB ingest + a worker that answers adjacency queries without blocking the UI
thread.

---

## Goal

1. **Ingest path** for both envelopes (extend `import-registry-items` or add
   `import-registry-compat` / unified importer) — idempotent upsert by `resource_name` /
   edge key, scoped by `modpackId`.
2. **Postgres:** migrate `registry_items` for new kinds if needed; add `registry_compat`
   (from_node, to_node, edge_type, modpack_id, …) matching
   `registry-compat.schema.json` (**already shipped by T-150** — do not recreate).
3. **API:** extend `GET /api/v1/registry` (or sibling routes) so FE can fetch items + edges
   (ETag/304 preserved where applicable). Contract stays snake_case.
4. **Worker:** `registry.worker.ts` + thin client — Comlink; IDB cache keyed by modpack /
   content hash; indices for adjacency; expose `canEquip` / `canAttach` (and a generic
   `hasEdge(from, to, type?)` if cleaner).
5. **Tests:** ingest IT and/or worker unit tests proving STANAG→M16A2 and at least one
   `mag_in_vehicle_weapon` edge from the committed T-150 sample.
6. Verify log + tag **T-068.9**. Cursor doc-sync after.

---

## Out of scope

- Smart Forge UI (**T-068.10**)
- Asset Browser map wiring (**T-146**) / vehicle place (**T-070**)
- Compiler / player equip (**T-068.11+**)
- Inventing `ammo_in_mag` edges (T-150 OPEN — AmmoConfig `.conf`; leave empty)
- Re-running Workbench export unless needed to refresh committed samples

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Compat schema | **Already at** `packages/tbd-schema/schema/registry-compat.schema.json` (T-150) |
| Sample data | `packages/tbd-schema/registry/registry-items.workbench.json` + `registry-compat.workbench.json` |
| Graph identity | Nodes = `resource_name` strings |
| OPEN edges | `ammo_in_mag` / shell-style `ammo_in_vehicle_weapon` stay empty until engine path exists |
| Docs/registry | Claude does **not** edit (verify log OK) |

---

## Tasks

1. Read T-150 schemas + committed envelopes; design migration + import CLI.
2. Implement migration + GORM model(s) + import.
3. Extend registry API (document `@route` / types).
4. Implement worker + client + IDB cache.
5. Tests with real T-150 sample edges.
6. `.ai/artifacts/t068_9_verify_log.md` + tag **T-068.9**.

---

## Verify

```bash
make db-up
# import both envelopes (exact CLI as implemented — document in verify log)
make test-it
cd apps/website/frontend && npm test && npm run build && npm run lint
cd packages/tbd-schema && npm run validate
```

Worker smoke: `canAttach` / `canEquip` true for a known `mag_in_weapon` pair from the sample;
false for a cross-well negative (e.g. wrong mag family).

---

## Acceptance

- [ ] Both T-150 envelopes ingest cleanly (idempotent re-run).
- [ ] API serves items + compat (or documented split routes).
- [ ] Worker answers adjacency without main-thread full-graph scan each call.
- [ ] Tests cover ≥1 infantry + ≥1 vehicle-weapon edge from sample.
- [ ] Tag **T-068.9**.

---

## Claude Code prompt — T-068.9 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.9** — Registry worker + compat ingest (T-150 data).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git pull && git lfs pull
  git rev-parse T-150   # expect 9107bf4e
  make db-up

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t068_9_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t068_9_registry_worker_ingest.md
  3. docs/specs/Mission_Creator_Architecture/t150_universal_registry_export.md
  4. .ai/artifacts/t150_verify_log.md
  5. packages/tbd-schema/schema/registry-items.schema.json
  6. packages/tbd-schema/schema/registry-compat.schema.json
  7. packages/tbd-schema/registry/registry-items.workbench.json
  8. packages/tbd-schema/registry/registry-compat.workbench.json
  9. apps/website/internal/handlers (registry) + cmd/import-registry-items
  10. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  T-150 shipped 1,880 items + 4,012 edges on disk. Website still lacks Postgres ingest of
  compat + a worker canEquip/canAttach over that graph.

═══ SHIPPED (do not reopen) ═══
  T-150 @ 9107bf4e (schemas + Workbench export + sample envelopes).
  T-068.1–.6 Phase 1 flat registry API/UI/NPC equip.

═══ DO ═══
  - Migrate + ingest items + compat (idempotent; modpack-scoped)
  - API surface for items + edges (ETag OK)
  - registry.worker.ts + client: IDB cache, canEquip/canAttach
  - Tests using committed T-150 sample (STANAG→M16A2; one mag_in_vehicle_weapon)
  - .ai/artifacts/t068_9_verify_log.md + tag T-068.9

═══ DO NOT ═══
  - Recreate registry-compat.schema.json (exists)
  - Invent ammo_in_mag edges
  - Forge UI (T-068.10), T-146 browser, T-070 place, markers
  - Edit registry / CLAUDE / hub (Cursor)

═══ VERIFY ═══
  (commands in spec §Verify — paste into verify log)

═══ RETURN ═══
  SHA + tag T-068.9
  Import CLI usage + row/edge counts in DB
  Ready for Cursor: T-068.10 / T-146 queue
```
