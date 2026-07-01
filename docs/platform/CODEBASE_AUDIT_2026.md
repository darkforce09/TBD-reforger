# CODEBASE_AUDIT_2026 — TBD Reforger monorepo

**Date:** 2026-06-29 · **Ticket:** T-122 (single bundle — all fixes)  
**Path map:** `website/`→`apps/website/`, `mod/`→`apps/mod/`, `shared/tbd-schema/`→`packages/tbd-schema/` (`2a51d66`).

---

## Executive summary

Read-only audit (frontend / backend / mod). **One ticket (T-122)** ships all code fixes in one Claude pass. R0 (Dashboard hub link) verified OK — not a defect.

| Severity | IDs | Count |
|----------|-----|------:|
| Critical | C1–C4 | 4 |
| Routing | R1–R3 | 3 |
| Tech debt | T1–T17 | 17 |
| Minor | M1–M15 | 15 |
| Doc drift | D1–D2 | 2 |

---

## Critical

| ID | File:line | Problem | Fix |
|----|-----------|---------|-----|
| **C1** | `admin.go:15-22` | `validRole` omits `leader` → PATCH role leader → 400 | Add `models.RoleLeader` |
| **C2** | `telemetry.go:206-208` | Filters `event_id` (dropped in `02_campaign_refactor.sql`) → ingest 500 | `event_mission_id IN (SELECT id FROM event_missions WHERE event_id = ?)`; return Update error |
| **C3** | `useMissionDoc.ts:32,267-271` | Boot catch → `ready`; no error UI on IDB failure | `DocStatus 'error'` + overlay + toast |
| **C4** | `TBD_LoadoutEquipComponent.c:60` (`TBD_GameMode.et:8`) | `Attribute("1")` → dev loadout test on live gamemode | Default `"0"` |

---

## Routing

| ID | File:line | Problem | Fix |
|----|-----------|---------|-----|
| **R1** | `operations.tsx:295-301`, `deployments.go:15-24` | MODIFY ASSIGNMENT → `/events/:id` only | BE `event_mission_id` on DTO; FE deep-link ORBAT |
| **R2** | `router.tsx:100`, `events.tsx` | Orphan `/events/:id/missions/:emid/orbat` | Link from R1 (or delete if inline-only) |
| **R3** | `TopNav.tsx:64-77` | Both menu items → `/settings` | Identity → `/settings#arma-link` or remove dup |

---

## Tech debt (backend)

| ID | Area | Fix summary |
|----|------|-------------|
| T1 | missions persist | Validate payload vs editor contract — **RESOLVED (T-123.5):** `mission-editor-payload.schema.json` in `CreateVersion` |
| T2 | missions read | Draft visibility — author/admin only when not live |
| T3 | CreateVersion | Guard live mission version swap |
| T4 | InjectMission | Move to admin tier |
| T5 | ClearSlot | Return tx errors |
| T6 | duplicate key | Use Postgres 23505 not string match |
| T7 | Withdraw | Propagate slot-free / waitlist errors |
| T8 | Export | `schemaVersion` int vs string enum collision — **RESOLVED (T-123.1):** renamed to `exportFormatVersion` on export envelope |
| T9 | ListAllLeave | Return total/limit/offset |

## Tech debt (frontend MC)

| ID | Area | Fix summary |
|----|------|-------------|
| T10 | useMissionEditor | Hydrate: 404 silent, 5xx toast |
| T11 | IDB persist | Surface write/quota failures |
| T12 | FpsCounter | Dev gate only |
| T13 | selectors | Remove dead exports or wire up |

## Tech debt (mod)

| ID | Area | Fix summary |
|----|------|-------------|
| T14 | samples vs plugin | Unify modpackId |
| T15 | ScenarioRouter | Real GUID + prod Everon conf |
| T16 | RegistryPoc | Dev gate default off |
| T17 | RadioBridge | Document stubs / Phase 3 |

---

## Minor (M1–M15)

Backend: M1 min/max on MaxPlayers/MaxSlots; M2 SetTrustedProxies; M3 OAuth cookie Secure/SameSite; M4 APP_ENV prod default; M5 AddEventMission 404; M6 swallowed `_ = db.First` / audit drops.

Frontend: M7 TopCommandStrip setTimeout cleanup.

Mod: M8 dead .gitignore lines; M9–M10 loadout validate version/modpackId; M11 meta.id vs schema; M12 roster event check; M13 Registry hint path; M14 dead arland comment; M15 empty SpawnManager overrides.

---

## Doc drift

| ID | Fix |
|----|-----|
| D1 | `MONOREPO_MIGRATION.md` — update layout to `apps/` + `packages/` + `.ai/` |
| D2 | Fix links to `docs/website/platform/context_handoff.md` | **Done** — stub @ `docs/platform/context_handoff.md` + frontend page-doc link depth fixed (Cursor doc pass) |

---

## Verified-clean

Dashboard hub link OK; T-060 body limit; no panic/log.Fatal on request path; MC lifecycle clean; zero Tbdevent_Website refs in mod code (dead .gitignore lines only).

---

## Verification (T-122 exit)

```bash
make test-it
cd apps/website/frontend && npm run build && npm run lint
```

Manual: C1 leader PATCH; C2 match ingest; C3 corrupt IDB; C4 gamemode boot; R1 deployments deep-link.

**Shipped:** T-122 fixes @ `f131770` (branch `ticket/T-122`). `make test-it` ok; frontend `npm run build` + `npm run lint` clean. Go `go build ./...` clean.

**Deferred (with reasons — not "skipped"):**
- **T1 — RESOLVED (T-123.5 @ `b5211f2`):** `CreateVersion` validates the version payload against a dedicated `mission-editor-payload.schema.json` (the editor superset) via `internal/contract/validate.go` (`go:embed` + `santhosh-tekuri/jsonschema/v6`) → 400 on malformed. A separate schema was authored precisely because validating against the canonical `mission.schema.json` would reject valid editor payloads.
- **T3** — `missions_integration_test.go` asserts a new version *can* be added to a `live` mission (intended re-versioning); blocking it is a mission-lifecycle change, not a one-liner.
- **T8 — RESOLVED (T-123.1 @ `04a73a1`):** the export envelope's version field was renamed `schemaVersion` → `exportFormatVersion` (int) across Go `missionJSON`, TS `MissionExport`/`compile.ts`, and the export/inject tests — freeing the `schemaVersion` key for the canonical string contract.
- **T15** — the real published addon GUID (and a non-dev Everon scenario `.conf`) are unknown here; a placeholder TODO + tracked note were added instead of guessing a wrong GUID.

**Mod note:** Enfusion `.c` changes (C4, T14/T16/T17, M8–M15) are not covered by `make test-it`/frontend build and need a Workbench pass to validate at runtime.

**Doc pass @ merge:** Frontend surface docs + `CLAUDE.md` context_handoff path; registry `shipped`; see commit after `efe14b2`.

**Shipped (T-123 @ `169e47d`):** documentation-standards program (range `f0af31a..169e47d`; CI green @ `7a08a8f`) — resolves audit **T1** (editor-payload validation) and **T8** (`exportFormatVersion` rename) above; adds in-code `@contract`/`@route`/`@authority` tags (Go/TS/Enfusion), schema codegen (`make schema-codegen` → `internal/contract/`), and the `contracts.yml` CI gates. See [`t123_documentation_standards_rollout.md`](t123_documentation_standards_rollout.md).

---

## Fable 5 audit follow-up — T-126 (shipped @ `4a47688e`, tag **T-126**)

Source: [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) §2 Backend · verify [`.ai/artifacts/t126_verify_log.md`](../../.ai/artifacts/t126_verify_log.md)

| ID | Audit finding | Fix | Proof |
|----|---------------|-----|-------|
| **S1** | `ExportMission` skipped `canViewMission` — cross-author draft leak | Gate export like `GetMission` → **404** | `TestExportMissionVisibility` |
| **S2** | Non-atomic refresh rotation; no reuse detection | `UPDATE … WHERE revoked_at IS NULL` + `RowsAffected==1`; spent token → **401** + token-family revoke | `TestRefreshReuseRevokesFamily` |
| **S3** | ORBAT slot claim check-then-set race | `FOR UPDATE` on event-mission row + conditional slot `UPDATE` → **409** | `TestSlotClaimRace` |
| **S4** | `Refresh` ignored `user.IsBanned` | **403** + family revoke | `TestRefreshBannedRejected` |
| **S5** | 401-retry dropped rotated refresh when no user in store | `setTokens()` persists full pair | FE build + lint |
| **S6** | Bootstrap/callback `clearSession()` after rotation + transient `/me` fail | Retain rotated pair; only failed rotation clears | FE build + lint |

**Still open (Fable program):** **T-127** (MC UX U1–U4) · **T-128** (doc links + staging honesty). Mod REST `/compiled` chain (**T-092**) unchanged.
