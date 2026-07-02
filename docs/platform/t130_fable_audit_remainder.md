# T-130 — Fable audit remainder (OPEN + PARTIAL)

**Status:** **ready** · **Active slice:** **T-130.1** (worktree `ticket/T-130`)  
**Parallel to:** **T-090.1.2.8** on **main** (registry queue — do not block map work)  
**Ticket:** T-130 · **Registry:** [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)  
**Living tracker:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md)  
**Handoff:** [`.ai/artifacts/t130_claude_code_handoff.md`](../../.ai/artifacts/t130_claude_code_handoff.md)

---

## Why this ticket exists

Fable program **T-126 → T-128** closed docs/security/MC UX. The living tracker still has **~21 OPEN** findings and **1 PARTIAL** (F4-03) with no owning ticket. **DEFERRED** rows stay on **T-090 / T-092 / T-122-T15** — do not reimplement here.

**Goal:** every OPEN/PARTIAL row → **RESOLVED** when T-130 ships; tracker summary **OPEN: 0**, **PARTIAL: 0**.

---

## Execution model

| Checkout | Branch | Agent | Scope |
|----------|--------|-------|-------|
| Repo root | `main` | Claude Code | **T-090.1.2.8** → T-068 → T-092 |
| `.ai/artifacts/worktrees/TBD-T-130` | `ticket/T-130` | Claude Code | **T-130.1 → T-130.6** |
| Either | — | Cursor | **T-130.7** docs (after .6 merges or on worktree) |

**Merge order:** ship slices on worktree → merge `ticket/T-130` → `main` when **T-130.1–.6** done (T-130.7 can land in same merge or follow-up doc sync).

---

## Slice index

| Slice | Executor | Findings | Summary |
|-------|----------|----------|---------|
| **T-130.0** | cursor-docs | — | Registry + this hub + handoffs + worktree README |
| **T-130.1** | claude-code | F2B-07, F2B-08, F2B-09, F2B-11 | Backend hygiene: count errors, export fail-loud, refresh purge, ratelimit path match |
| **T-130.2** | claude-code | F3-01, F3-02, F3-03 | Discord 429 + Retry-After; webhook title cap; OAuth blank guard |
| **T-130.3** | claude-code | F2B-06 | CI + `make ci-local` run `services`, `middleware`, `realtime` tests |
| **T-130.4** | claude-code | F1-16…F1-20 | Mod loaders/exporters: read cap, RPC bounds, Write checks, JSON escape, registry minItems |
| **T-130.5** | claude-code | F4-03, F4-07, F2F-07 | MC conflict new-tab; non-UUID trap; admin Aegis Dialog |
| **T-130.6** | claude-code | F2B-05, F4-04 | Mission archive/delete API + library UI |
| **T-130.7** | cursor-docs | F1-09, F1-11, F5-08, F5-09, F2C-04, F4-08, F5-10 | Manifest/schema/docs nits + ticket brief hybrid policy |

---

## T-130.1 — Backend hygiene

**Files:** [`missions.go`](../../apps/website/internal/handlers/missions.go), [`auth.go`](../../apps/website/internal/handlers/auth.go), [`middleware/ratelimit.go`](../../apps/website/internal/middleware/ratelimit.go)

| ID | Fix |
|----|-----|
| F2B-07 | Propagate `Count` error on list; don't return `total: 0` on failure |
| F2B-08 | `buildMissionDoc`: return error when current version load fails — no silent `{}` / `0.0.0` |
| F2B-09 | Purge revoked refresh tokens (startup sweep or periodic job; document retention) |
| F2B-11 | Ratelimit: exact path match or segment-aware match — no `strings.Contains` prefix footgun |

**Tests:** extend handler tests where patterns exist (T-126 style). **`make test-it`** if touching auth/missions.

---

## T-130.2 — Discord integration

**Files:** [`discord.go`](../../apps/website/internal/services/discord.go), [`webhook.go`](../../apps/website/internal/services/webhook.go), OAuth entry in [`handlers/auth.go`](../../apps/website/internal/handlers/auth.go)

| ID | Fix |
|----|-----|
| F3-01 | On HTTP 429: read `Retry-After`, backoff/retry (bounded) in `do()` and webhook push |
| F3-02 | Truncate embed title to 256 runes; footer to 2048 before POST |
| F3-03 | `AuthorizeURL`: fail fast with clear error when `client_id` empty (prod misconfig guard) |

**Tests:** `discord_test.go`, `webhook_test.go` (enabled by T-130.3 CI scope).

---

## T-130.3 — CI test scope

**Files:** [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), [`Makefile`](../../Makefile) (`ci-local` target)

| ID | Fix |
|----|-----|
| F2B-06 | Run `go test` on `./internal/services/...`, `./internal/middleware/...`, `./internal/realtime/...` in CI and `make ci-local` |

---

## T-130.4 — Mod robustness

**Files:** `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c`, `TBD_MissionBrowser.c`, export plugins under `Scripts/Game/TBD/Export/`

| ID | Fix |
|----|-----|
| F1-16 | Profile read >8 MB → explicit error, not silent truncate |
| F1-17 | Mission list RPC: bound payload or admin gate (match audit intent) |
| F1-18 | Check `FileHandle.Write` on all terrain/satellite/registry exporters |
| F1-19 | Registry export: refuse `items: []` when schema requires minItems 1 |
| F1-20 | Escape JSON strings in meta; document or env-var the Proton path |

---

## T-130.5 — MC + admin UX

**Files:** [`useMissionEditor.ts`](../../apps/website/frontend/src/features/mission-creator/hooks/useMissionEditor.ts), [`MissionCreatorPage.tsx`](../../apps/website/frontend/src/features/mission-creator/MissionCreatorPage.tsx), [`admin.tsx`](../../apps/website/frontend/src/pages/admin.tsx)

| ID | Fix |
|----|-----|
| F4-03 | **PARTIAL:** persist server-adopt / divergence marker so **new-tab** cold boot skips conflict when IDB matches (extend T-127 U1) |
| F4-07 | Non-UUID mission id: block save/export or redirect — not interactive-but-unsavable |
| F2F-07 | Replace `window.confirm` in admin Event Manager with Aegis `Dialog` |

---

## T-130.6 — Mission lifecycle

**Pattern:** announcement archive in [`cms.go`](../../apps/website/internal/handlers/cms.go)

| ID | Fix |
|----|-----|
| F2B-05, F4-04 | `PATCH` archive + soft `DELETE` handlers; library UI for author/admin; use existing `MissionArchived` / `DeletedAt` |

**API contract:** snake_case JSON; update frontend types + mutations.

---

## T-130.7 — Docs / schema nits (Cursor)

| ID | Fix |
|----|-----|
| F1-09 | Align `everon/manifest.json` `metersPerPixel` with schema (2 m) |
| F1-11 | Align `terrainId` extensibility between manifest + registry schemas |
| F5-08 | Rename `tmsY` → `xyzRow` (or similar) in `tileUrl.ts` + test |
| F5-09 | Mermaid `<br/>` in `t092_spawn_transform_program.md` |
| F2C-04 | `./scripts/ticket brief` documents hybrid policy (main + `ticket/T-0xx` worktrees) |
| F4-08 | Optional: shortcut cheat-sheet in MC help or tooltips (stretch) |
| F5-10 | Spelling dialect — only if trivial; skip Eden-wiki scrape |

---

## Verify (per slice)

```bash
# After T-130.1–.5 with website touches:
cd apps/website/frontend && npm run build && npm run lint

# Backend:
make test-it   # when DB available
go test ./internal/services/... ./internal/middleware/... ./internal/realtime/...

# Mod slices: manual Workbench smoke where applicable
```

Deliver **`.ai/artifacts/t130_verify_log.md`** per batch (or one log with sections).

---

## Return

- Commit prefix **T-130:** (or **T-130.1:** if slice-per-commit)  
- Tag **T-130** when all slices merged to main  
- **Ready for Cursor doc sync** — flip living tracker OPEN → RESOLVED; `./scripts/ticket sync`

**Do not edit** (Claude Code): `docs/**`, `.ai/tickets/registry.json`, CLAUDE markers — return to Cursor for doc sync.
