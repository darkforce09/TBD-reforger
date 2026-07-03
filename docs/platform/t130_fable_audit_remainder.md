# T-130 — Fable audit remainder (OPEN + PARTIAL)

**Status:** **shipped** @ `90c9f261` (tag **T-130**) · merged to **main** 2026-07-03  
**Verify:** [`.ai/artifacts/t130_verify_log.md`](../../.ai/artifacts/t130_verify_log.md)  
**Ticket:** T-130 · **Registry:** [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)  
**Living tracker:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) — OPEN/PARTIAL rows flipped on doc sync  
**Handoff:** [`.ai/artifacts/t130_claude_code_handoff.md`](../../.ai/artifacts/t130_claude_code_handoff.md)

---

## Why this ticket exists

Fable program **T-126 → T-128** closed docs/security/MC UX. The living tracker still had **~21 OPEN** findings and **1 PARTIAL** (F4-03) with no owning ticket. **DEFERRED** rows stay on **T-090 / T-092 / T-122-T15** — do not reimplement here.

**Goal:** every OPEN/PARTIAL row owned by T-130 → **RESOLVED** when shipped; tracker summary **OPEN: 0**, **PARTIAL: 0** (except intentional deferrals).

---

## Shipped slices

| Slice | SHA | Findings |
|-------|-----|----------|
| **T-130.0** | `9f896563` | Registry + hub + handoffs |
| **T-130.1** | `6426600f` | F2B-07, F2B-08, F2B-09, F2B-11 |
| **T-130.2** | `9db1b9e1` | F3-01, F3-02, F3-03 |
| **T-130.3** | `755a889b` | F2B-06 |
| **T-130.4** | `b62a66b7` | F1-16…F1-20 |
| **T-130.5** | `bb40a61a` | F4-03 (closed), F4-07, F2F-07 |
| **T-130.6** | `c8b2fd6e` | F2B-05, F4-04 |
| **T-130.7** | doc sync | F1-09, F1-11, F5-08, F5-09, F2C-04, F4-08; F5-10 deferred |

---

## Slice index (reference)

| Slice | Executor | Findings | Summary |
|-------|----------|----------|---------|
| **T-130.0** | cursor-docs | — | Registry + this hub + handoffs + worktree README |
| **T-130.1** | claude-code | F2B-07, F2B-08, F2B-09, F2B-11 | Backend hygiene |
| **T-130.2** | claude-code | F3-01, F3-02, F3-03 | Discord 429 + embed caps + OAuth guard |
| **T-130.3** | claude-code | F2B-06 | CI + `make ci-local` services/middleware/realtime |
| **T-130.4** | claude-code | F1-16…F1-20 | Mod loaders/exporters |
| **T-130.5** | claude-code | F4-03, F4-07, F2F-07 | MC conflict cross-tab; non-UUID trap; admin Dialog |
| **T-130.6** | claude-code | F2B-05, F4-04 | Mission archive/delete API + library UI |
| **T-130.7** | cursor-docs | F1-09, F1-11, F5-08, F5-09, F2C-04, F4-08 | Manifest/schema/docs nits + ticket brief hybrid policy |

---

## T-130.7 — Docs / schema nits (shipped)

| ID | Fix |
|----|-----|
| F1-09 | `everon/manifest.json` `metersPerPixel` → **2** (Info & Diags / DEM native; matches `precision.demNativeMetersPerPixel`) |
| F1-11 | `terrainId` extensibility aligned — manifest, type-inventory, anchors schemas use `minLength: 1` like `terrain-registry.schema.json` |
| F5-08 | `tileUrl.ts` internal `tmsY` → `xyzRow` (+ test comments) |
| F5-09 | Mermaid `<br/>` in `t092_spawn_transform_program.md` node labels |
| F2C-04 | `./scripts/ticket brief` prints hybrid execution policy (main vs worktree) |
| F4-08 | Keyboard shortcuts table in `ux_spec.md` §Keyboard shortcuts |
| F5-10 | Spelling dialect — **deferred** (trivial; Eden wiki scrape out of scope) |

---

## T-130.1 — Backend hygiene

**Files:** [`missions.go`](../../apps/website/internal/handlers/missions.go), [`auth.go`](../../apps/website/internal/handlers/auth.go), [`middleware/ratelimit.go`](../../apps/website/internal/middleware/ratelimit.go)

| ID | Fix |
|----|-----|
| F2B-07 | Return 500 if `Count` fails |
| F2B-08 | Propagate `buildMissionDoc` load failure |
| F2B-09 | Purge old revoked refresh rows ([`token_purge.go`](../../apps/website/internal/services/token_purge.go)) |
| F2B-11 | Strict rate-limit path prefix match |

---

## T-130.2 — Discord

| ID | Fix |
|----|-----|
| F3-01 | Honor `Retry-After` on 429 ([`httpretry.go`](../../apps/website/internal/services/httpretry.go)) |
| F3-02 | Truncate embed title/footer before POST |
| F3-03 | OAuth blank `client_id` → SPA `#error=oauth_unconfigured` |

---

## T-130.3 — CI

| ID | Fix |
|----|-----|
| F2B-06 | CI + `make ci-local-backend` run `services`, `middleware`, `realtime` tests |

---

## T-130.4 — Mod

| ID | Fix |
|----|-----|
| F1-16 | Profile read >8 MB → explicit error (`GetLength` before read) |
| F1-17 | Mission list RPC admin-gated + 100-line cap |
| F1-18 | All exporter `Write` checks via `TBD_ExportJson.Write` |
| F1-19 | Registry export refuses `items: []` |
| F1-20 | JSON escape + `TBD_ExportPaths.c` `PROFILE_WIN` constant |

---

## T-130.5 — MC + admin UX

| ID | Fix |
|----|-----|
| F4-03 | Cross-tab `localStorage` adopted-semver marker — new-tab cold boot skips conflict when semver matches |
| F4-07 | Non-UUID id → full-bleed hard block + Library link |
| F2F-07 | Admin delete → Aegis `Dialog` |

---

## T-130.6 — Mission lifecycle

| ID | Fix |
|----|-----|
| F2B-05, F4-04 | `PATCH` archive + soft `DELETE`; library Manage section; global hides archived, mine badges archived |

---

## Verify

See [`.ai/artifacts/t130_verify_log.md`](../../.ai/artifacts/t130_verify_log.md).

**Operator follow-up:** Workbench compile + one export re-run recommended after T-130.4 (mechanical guards; Workbench was down during Batch 2 verify).

---

## Return

**Shipped** on `main` @ `90c9f261` · tag **T-130** · Fable OPEN/PARTIAL program **complete** (deferrals unchanged).
