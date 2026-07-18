# Backend — ROADMAP

> **SUPERSEDED (T-145 / T-171):** Live API is **Rust Axum + sqlx** at [`apps/website/api/`](../../../apps/website/api/) (pkg `website-api`). Go/`internal/`/`cmd/api` / GORM / golangci rows below are historical. Run: `make api` · SPA: `make leptos` · seeds: `apps/website/api/seeds/` · migrations: `apps/website/api/migrations/`. Conventions: [`WHERE_DOES_X_GO.md`](../../platform/WHERE_DOES_X_GO.md). Canonical status: root [`CLAUDE.md`](../../../CLAUDE.md).

**Start here for planning history.** What was **shipped** under the Go era; verify paths against live `api/src/`.

**Queue:** [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md) · **Full registry:** [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md)

**Live code:** [`apps/website/api/src/`](../../../apps/website/api/src) · **Contract:** `api/src/models/` (serde snake_case) + generated `api/src/contract/generated/`

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`architecture.md`](architecture.md)** | ARCHIVE design plan — verify vs live `api/src/models/` |
| **[`DEV_RUNBOOK.md`](../DEV_RUNBOOK.md)** | db-up, api, leptos, dev-login, test-it, seeds, map-assets |
| **[`docs/website/platform/registration_flow.md`](../platform/registration_flow.md)** | ORBAT registration design (**implemented** T-008–T-010) |
| **[`docs/website/platform/context_handoff.md`](../platform/context_handoff.md)** | Original product blueprint (§3 partially stale) |
| **Root [`CLAUDE.md`](../../../CLAUDE.md)** | T-0xx milestones, auth tiers, doc-on-commit rule |
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
| **Realtime** | — | SSE hub in `api/src/realtime/` | |

**Migrations (live):** [`apps/website/api/migrations/`](../../../apps/website/api/migrations) · **Seeds:** [`apps/website/api/seeds/`](../../../apps/website/api/seeds) (`make seed` = Discord roles + registry_dev)

---

## SHIPPED — platform audit (T-122 @ `f131770`)

| T-ID | Item | Notes |
|------|------|-------|
| **T-122** | **Codebase audit hotfix** | [`CODEBASE_AUDIT_2026.md`](../../platform/CODEBASE_AUDIT_2026.md) — C1 leader PATCH; C2 attendance ingest; T2 mission visibility; T4 inject admin-only; T5 body limit on `/missions/:id/versions`; 37/41 shipped |

## SHIPPED — documentation standards (T-123 @ `169e47d`)

| T-ID | Item | Notes |
|------|------|-------|
| **T-123** | **Documentation standards rollout** | [`DOCUMENTATION_STANDARDS.md`](../../platform/DOCUMENTATION_STANDARDS.md) — `@contract`/`@route`/`@authority` tags; codegen → `apps/website/api/src/contract/generated/` via `make schema-codegen`; `CreateVersion` validates payload schema; `contracts.yml` CI |

## SHIPPED (T-068.2–T-068.4 @ main)

| T-ID | Item | Spec | Notes |
|------|------|------|-------|
| **T-068.2** | **`GET /api/v1/registry`** | [`t068_2_registry_api.md`](../../specs/Mission_Creator_Architecture/t068_2_registry_api.md) | `resource_name` rows, weak ETag/304, `registry_dev.sql` + `import-registry-items` CLI |
| **T-068.3** | **Factions palette → registry** | [`t068_3_palette_wire.md`](../../specs/Mission_Creator_Architecture/t068_3_palette_wire.md) | `useRegistry()` + `buildCatalogTree`; mock deleted @ `da78452` |
| **T-068.4** | **Arsenal dumb loadout UI** | [`t068_4_dumb_loadout_ui.md`](../../specs/Mission_Creator_Architecture/t068_4_dumb_loadout_ui.md) | Frontend-only; reuses `GET /registry` gear rows + `modpack_id` on export @ `a85f16b` |

## SHIPPED — mod (T-068.5, no backend change)

| T-ID | Item | Spec | Notes |
|------|------|------|-------|
| **T-068.5** | **Mod equip from loadout-export JSON** | [`t068_5_mod_equip_loadout.md`](../../specs/Mission_Creator_Architecture/t068_5_mod_equip_loadout.md) | `TBD_LoadoutEquipComponent.c` @ `21ec91e`; reads `$profile:TBD_LoadoutTest.json` |

## IN PROGRESS — Phase 1 E2E (T-068.6)

| T-ID | Item | Spec | Notes |
|------|------|------|-------|
| **T-068.6** | **Human Phase 1 E2E gate** | [`t068_6_phase1_e2e_gate.md`](../../specs/Mission_Creator_Architecture/t068_6_phase1_e2e_gate.md) | E1–E12 checklist; executor **human** |

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

1. **T-068.6** — human Phase 1 E2E sign-off (mod equip shipped @ T-068.5; backend unchanged)
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

API contract smoke: hit endpoint, confirm JSON matches `api/src/models/` serde + `frontend/src/dto.rs`.
