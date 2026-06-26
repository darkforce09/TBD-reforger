# T-068.2 — Registry API + dev seed + import

**Ticket:** T-068 · **Slice:** T-068.2  
**Status:** Spec ready — code pending  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Ship `registry_items` Postgres table, dev seed, `GET /api/v1/registry`, admin import command, and integration tests.

---

## Problem

No backend registry route; Factions palette uses [`assetCatalogMock.ts`](../../../apps/website/frontend/src/features/mission-creator/layout/RightInspector/assetCatalogMock.ts).

---

## Goal

1. GORM model `RegistryItem` — `modpack_id`, `resource_name`, `display_name`, `category`, `icon_url`, `kind`, `sort_order`; unique `(modpack_id, resource_name)`.
2. Migration `internal/db/migrations/03_registry_items.sql` (idempotent).
3. Dev seed `internal/db/seeds/registry_dev.sql` — **≥20 rows**, all five **`kind`** values (`character` + four `gear_*`); **`resource_name`** = real `{GUID}Prefabs/...` strings (seed from [`registry.json`](../../../apps/mod/tbd-framework/Data/registry.json) POC + gear from **T-068.1** or MCP — **never** mock catalog ids like `a-nato-rifleman`); FK **`modpack_id`** = current modpack (`00000000-0000-4000-a000-000000000001` in [`mock_data.sql`](../../../apps/website/internal/db/seeds/mock_data.sql) when mock modpack loaded); wire into `make seed` or document apply in DEV_RUNBOOK.
4. `GET /api/v1/registry?modpack=<uuid>` — mission_maker+ JWT; resolve current modpack when omitted; response `{ data, etag, modpack_id, modpack_version }`; weak ETag + **304** on `If-None-Match`.
5. `cmd/import-registry-items` — read `registry-items` JSON file; upsert rows for modpack (admin/dev use for T-068.1 export landing).
6. Integration test: 200 + etag; 304 repeat; 404 bad modpack.

---

## Out of scope

- Frontend palette (T-068.3)
- Worker / IndexedDB (T-068.9)
- POST import HTTP route (CLI import sufficient for Phase 1)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| API field | `resource_name` (not legacy `classname`) |
| Auth | `RequireMinRole("mission_maker")` on `mm` group |
| Dev path without Workbench | `registry_dev.sql` unblocks **T-068.3 API smoke** only — must still include **gear_* kinds** |
| Modpack FK | Seed rows reference `modpacks.is_current` UUID (document in paste) |
| Post-T-068.1 ingest | `import-registry-items --file registry-items.workbench.json` documented in DEV_RUNBOOK |
| ETag | Weak; modpack_id + count + max(updated_at) |

---

## Tasks

1. `internal/models/registry.go`
2. `internal/db/migrations/03_registry_items.sql`
3. `internal/db/seeds/registry_dev.sql`
4. `internal/handlers/registry.go` + route in `handlers.go`
5. `cmd/import-registry-items/main.go`
6. `internal/handlers/registry_integration_test.go`
7. `frontend/src/types/models/registry.ts` — `RegistryItem`, `RegistryResponse`

---

## Verify

```bash
make db-up && make seed
PATH="$HOME/.local/go/bin:$PATH" make test-it
cd apps/website/frontend && npm run build && npm run lint
```

---

## Verification gate (mandatory)

**Advance when ALL PASS.**

### Automated (exit 0)

```bash
make db-up && make seed
PATH="$HOME/.local/go/bin:$PATH" make test-it
# Registry integration tests must pass (names may vary — grep output):
make test-it 2>&1 | tee /tmp/t068-2-test-it.log
grep -E 'PASS.*[Rr]egistry|ok.*registry' /tmp/t068-2-test-it.log
cd apps/website/frontend && npm run build && npm run lint
```

### API smoke (JWT required)

```bash
# 1) Dev-login in browser; copy access_token from callback fragment OR use session cookie flow
TOKEN="<paste>"
API=http://localhost:8080/api/v1
# 200 + shape
curl -sf -H "Authorization: Bearer $TOKEN" "$API/registry" | tee /tmp/registry.json | jq -e '.data | length >= 10'
curl -sf -H "Authorization: Bearer $TOKEN" "$API/registry" | jq -e '.etag | length >= 3'
curl -sf -H "Authorization: Bearer $TOKEN" "$API/registry" | jq -e '.data[0].resource_name | test("^\\{[0-9A-F]{16}\\}")'
ETAG=$(jq -r '.etag' /tmp/registry.json)
# 304 Not Modified
curl -sf -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $TOKEN" -H "If-None-Match: $ETAG" "$API/registry" | grep -x 304
# 404 unknown modpack
curl -sf -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $TOKEN" "$API/registry?modpack=00000000-0000-0000-0000-000000000000" | grep -x 404
```

### Import CLI (optional but recommended before marking PASS)

```bash
PATH="$HOME/.local/go/bin:$PATH" go run ./cmd/import-registry-items --file packages/tbd-schema/registry/registry-items.sample.json
# Re-curl: row count unchanged or increased; no duplicate resource_name errors in output
```

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Integration tests | `make test-it` exit 0; registry test file exists and runs |
| A2 | Seed data | `GET /registry` returns ≥10 rows for current modpack |
| A3 | Field contract | Each row has `resource_name`, `display_name`, `category`, `kind` |
| A4 | ResourceName | First row (and spot-check 3) match GUID regex |
| A5 | ETag | Response includes `etag`; second request with `If-None-Match` → **304** |
| A6 | 404 | Random UUID modpack query → **404** |
| A7 | Auth | Unauthenticated request → **401** |
| A8 | Import CLI | `import-registry-items` runs exit 0 on sample JSON (if implemented) |
| A9 | FE types | `registry.ts` compiles; build/lint clean |
| A10 | Gear kinds in seed | `jq` proof: ≥1 row per `gear_primary`, `gear_uniform`, `gear_vest`, `gear_helmet` in DB after seed |

### Verify paste (required)

Full `make test-it` tail + curl/jq outputs for A2–A7.

---

## Depends on / Unblocks

- **Depends on:** T-068.0.1
- **Unblocks:** T-068.3, T-068.4

---

## Documentation sync (Cursor)

After merge: `docs/backend/ROADMAP.md` registry row → in progress (ship @ T-068.3+).

---

## Claude Code prompt — T-068.2

```
Read CLAUDE.md §Status. Active slice: T-068.2.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_2_registry_api.md
Do not edit documentation. Branch: ticket/T-068
LOCKED: resource_name field; ETag/304; registry_dev.sql seed; import-registry-items CLI.
Verify: make db-up && make test-it; run ALL §Verification gate curl/jq checks; FE build/lint
Return: Verify paste block with A1–A9 table + command outputs.
```
