# Backend — ROADMAP

**Start here.** Planning view for the Go API — what is **shipped**, what is **deferred**, and links to all backend documentation and code.

**Queue:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) · **Full registry:** [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md)

**Code:** [`cmd/api/`](../../../apps/website/cmd/api) · **Contract:** [`internal/models/`](../../../apps/website/internal/models) (GORM JSON tags = API shape)

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`docs/backend/architecture.md`](architecture.md)** | Target schema + design (verify vs live models post T-008) |
| **[`DEV_RUNBOOK.md`](../DEV_RUNBOOK.md)** | db-up, api, web, dev-login, test-it, seeds |
| **[`docs/platform/registration_flow.md`](../platform/registration_flow.md)** | ORBAT registration design (**implemented** T-008–T-010) |
| **[`docs/platform/context_handoff.md`](../platform/context_handoff.md)** | Original product blueprint (§3 partially stale) |
| **[`CLAUDE.md`](../../../apps/website/CLAUDE.md)** | T-0xx milestones, auth tiers, doc-on-commit rule |
| **[`docs/TAGS.md`](../TAGS.md)** | T-0xx naming contract |

---

## DONE — shipped API areas

| Area | Models | Handlers | Notes |
|------|--------|----------|-------|
| **Auth** | `user.go` | `auth.go`, `dev.go` | Discord OAuth2, JWT + rotating refresh |
| **Missions** | `mission.go` | `missions.go` | Library, versions, export, armory, approvals |
| **Events / ORBAT** | `event.go` | `events.go` | Campaign refactor T-008; `event_missions`, auto-ORBAT |
| **Registrations** | `event.go` | `events.go` | Per-mission slots, squad reserve T-010 |
| **Telemetry** | `telemetry.go` | `telemetry.go` | Service-token ingest |
| **Leaderboards** | — | `leaderboards.go` | Materialized view |
| **Wiki / CMS** | `content.go` | `wiki.go`, `content.go` | Doctrine + admin CMS |
| **Vehicles** | — | `vehicles.go` | Vehicle database |
| **Admin** | `admin.go` | `admin.go` | Personnel, audit logs |
| **Realtime** | — | SSE hub in `internal/realtime/` | |

**Migrations:** [`internal/db/migrations/`](../../../apps/website/internal/db/migrations) · **Seeds:** [`internal/db/seeds/`](../../../apps/website/internal/db/seeds) (`make seed` = Discord roles only)

---

## SHIPPED (T-068.2–T-068.4 @ main)

| T-ID | Item | Spec | Notes |
|------|------|------|-------|
| **T-068.2** | **`GET /api/v1/registry`** | [`t068_2_registry_api.md`](../../specs/Mission_Creator_Architecture/t068_2_registry_api.md) | `resource_name` rows, weak ETag/304, `registry_dev.sql` + `import-registry-items` CLI |
| **T-068.3** | **Factions palette → registry** | [`t068_3_palette_wire.md`](../../specs/Mission_Creator_Architecture/t068_3_palette_wire.md) | `useRegistry()` + `buildCatalogTree`; mock deleted @ `da78452` |
| **T-068.4** | **Arsenal dumb loadout UI** | [`t068_4_dumb_loadout_ui.md`](../../specs/Mission_Creator_Architecture/t068_4_dumb_loadout_ui.md) | Frontend-only; reuses `GET /registry` gear rows + `modpack_id` on export @ `a85f16b` |

## IN PROGRESS — mod equip (T-068.5)

| T-ID | Item | Spec | Notes |
|------|------|------|-------|
| **T-068.5** | **Mod equip from loadout-export JSON** | [`t068_5_mod_equip_loadout.md`](../../specs/Mission_Creator_Architecture/t068_5_mod_equip_loadout.md) | Reads `$profile:TBD_LoadoutTest.json`; no backend change |

---

## NOT DONE — deferred (T-IDs)

| T-ID | Item | Blocked by | Notes |
|------|------|------------|-------|
| **T-086** | **Server control / RCON API** | Game server bridge | Frontend `/admin/server` stub |
| **T-095** | **Per-handler API reference doc** | — | Future `docs/backend/api.md` |
| **T-096** | **Live game-server telemetry bridge** | Service deployment | Ingest endpoints exist; no bridge wired |

Full deferred table: [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md) (`program: backend` + related platform rows).

---

## Recommended next work

1. **T-068.5** — mod equip from web download JSON (mod + MCP; backend unchanged)
2. **T-086** — when RCON/game-server integration is scoped
3. Keep **`internal/models/`** as source of truth — update TS types in [`frontend/src/types/`](../../../apps/website/frontend/src/types) when models change

---

## Verify changes

```bash
make db-up
PATH="/var/home/Samuel/.local/go/bin:$PATH" make api
# no /health route — confirm API is up via the dev-login 302:
curl -si "http://localhost:8080/api/v1/auth/dev-login?role=admin" | head -1
make test-it
```

API contract smoke: hit endpoint, confirm JSON matches GORM tags + frontend types.
