# T-128 — Fable audit doc link repair + staging honesty

**Ticket:** T-128 · **Executor:** cursor-docs · **Status:** **READY** (after **T-127**)  
**Git tag on ship:** **T-128**  
**Authority:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) §5 · [`FABLE_5_AUDIT_PROGRAM.md`](FABLE_5_AUDIT_PROGRAM.md)

---

## In one sentence

Repair **agent-breaking doc rot** from the monorepo move and mark **phantom mod staging gates** honestly — so humans and Fable/Claude sessions stop 404ing on handoffs.

---

## Problem

Fable counted **155 broken relative markdown links**, stale README trees, and staging docs that assert live routes (`GET /api/missions/:id/compiled`) that **T-092** has not shipped.

---

## Goal (prioritized batches)

### P0 — Agent handoffs (fixes every Claude send-off)

Fix `../docs/` → `../../docs/` (or correct depth) in:

- `.ai/artifacts/t090_*_handoff.md` and `*_SEND_TO_CLAUDE.md`
- `.ai/artifacts/t126_*`, `t127_*` (use correct paths at creation)
- `.ai/tickets/AI_PLAYBOOK.md`, `README.md`, `SPEC_TEMPLATE.md`, `CLAUDE_CODE_PROMPT.md`

### P1 — Staging honesty

- `docs/mod/STAGING-SERVER.md` — V2/V3 gates: **BLOCKED on T-092**; routes not live; expected status documented
- `docs/mod/MILESTONES.md`, `docs/mod/tbd-reforger-platform-build-plan.md` — same phantom route callouts
- `scripts/mod/deploy-staging.sh` — comment or guard curl expectations

### P2 — README rewrites

- `apps/mod/README.md` — monorepo paths (`docs/mod/`, `packages/tbd-schema/`, `scripts/mod/`)
- `apps/website/README.md` — fix relative links to `docs/website/`

### P3 — Orphans + renames

- Delete or stub-redirect `apps/website/frontend/docs/` duplicate tree (point to `docs/website/frontend/pages/`)
- Remove empty `apps/website/internal/handlers/missions/` dir if safe
- Renumber **floor picker** references **T-126 → T-129** in MC hub/ROADMAP (T-126 repurposed for audit)

### P4 — CLAUDE.md hygiene

- Run `./scripts/ticket sync` after registry updates
- Fix stale T-090 ACTIVE SLICE block contradictions via sync (not hand-edit markers)
- Fix Arland 10240 typo in `MissionCreatorPage.tsx` comment → 4096

---

## Out of scope

- Fixing all 155 links in one pass — batch by P0→P4; log remainder count in verify log
- `eden-wiki` scrape typos (verbatim external)
- Application code except comment typo (P4)

---

## Verify

```bash
./scripts/ticket sync && ./scripts/ticket check
make ticket-check-strict   # if link checker exists
# Manual: spot-check handoff links from .ai/artifacts/t126_SEND_TO_CLAUDE.md
```

Deliver `.ai/artifacts/t128_doc_link_repair_log.md` with before/after broken-link count (script or ripgrep).

---

## Documentation sync

Self-shipping ticket — update `FABLE_5_AUDIT_PROGRAM.md` §Status, registry `shipped_at`, tag **T-128**.
