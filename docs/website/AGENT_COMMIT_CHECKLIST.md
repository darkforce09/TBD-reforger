# Agent commit checklist

**Use on every feature commit.** Sync docs **in the same commit** as code — never merge stale docs.

**Authority ladder:** running code → [`CLAUDE.md`](../../CLAUDE.md) §Status → [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) → domain **ROADMAP.md** → supporting docs → archive.

**Doc ownership (locked 2026-06):** **Cursor (Composer 2.5)** writes and syncs all documentation. **Claude Code** reads docs and implements code only — return verify output to Cursor for the §Same-commit sync pass before the human commits.

---

## Ticket registry workflow

1. **Plan / queue change** — edit [`tickets/registry.json`](../../.ai/tickets/registry.json) (status, order, spec path, `active_slice`).
2. **Regenerate views** — `./scripts/ticket sync` (updates `docs/TICKET_*.md`, `CLAUDE.md` status markers, `tickets/queue.json`).
3. **Validate** — `./scripts/ticket check` or `make ticket-check-strict` (zero legacy IDs in authority docs).
4. **Implement** — Claude Code reads spec from registry row; **does not edit docs**.
5. **Ship** — human verifies → set row `status: shipped` → `./scripts/ticket sync` → Cursor syncs narrative docs below.

Playbook: [`tickets/AI_PLAYBOOK.md`](../../.ai/tickets/AI_PLAYBOOK.md). Lead view: [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md).

---

## Before you code

| Domain | Start here |
|--------|------------|
| **Any work** | [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) → registry row → spec path |
| **Frontend surfaces** | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) → [`frontend/docs/INDEX.md`](../../apps/website/frontend/docs/INDEX.md) |
| **Mission Creator** | MC [`ROADMAP.md`](../specs/Mission_Creator_Architecture/ROADMAP.md) → [`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md) §ACTIVE SLICE |
| **Backend / API** | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| **Cross-boundary code comments** | [`docs/platform/DOCUMENTATION_STANDARDS.md`](../platform/DOCUMENTATION_STANDARDS.md) — §11 cheat sheet; in-code = same commit as code (§1) |
| **Platform deps / toolchain** | [`docs/platform/t124_dependency_upgrade.md`](../platform/t124_dependency_upgrade.md) — Go 1.26, Node 26, Postgres 18 (T-124 shipped) |
| **Coding standards (T-125)** | [`docs/platform/CODING_STANDARDS.md`](../platform/CODING_STANDARDS.md) + [`t125_coding_standards_enforcement.md`](../platform/t125_coding_standards_enforcement.md) — before commit: `make ci-local` green (needs `make db-up` + `nvm use`); Go: `golangci-lint run ./...` |
| **Tag contract** | [`docs/TAGS.md`](TAGS.md) |

If the registry row has a `spec`, read it before editing code.

---

## Same-commit sync table

| What changed | Update these |
|--------------|--------------|
| **Shipped milestone** | Registry row → `shipped`; `./scripts/ticket sync`; [`CLAUDE.md`](../../CLAUDE.md) §Status **Done** bullet (sync may auto-update markers — verify narrative) |
| **Active slice** (in progress) | Registry `active_slice`; [`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md) §ACTIVE SLICE |
| **New or removed route** | [`frontend/src/router.tsx`](../../apps/website/frontend/src/router.tsx) + matching [`frontend/docs/pages/*.md`](../../apps/website/frontend/docs/pages) + [`frontend/docs/INDEX.md`](../../apps/website/frontend/docs/INDEX.md) + [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) |
| **UI surface (no route change)** | Relevant page spec **Element Inventory** + **`Live source:`** path |
| **Nav / sidebar change** | [`frontend/src/config/navigation.ts`](../../apps/website/frontend/src/config/navigation.ts) + [`frontend/docs/shell/sidebar.md`](../../apps/website/frontend/docs/shell/sidebar.md) |
| **API / model change** | `internal/models/` JSON tags + matching `frontend/src/types/`; note handler if behavior changed |
| **Cross-boundary type or handler** | In-code Godoc/TSDoc/Enforce comments + `@contract` / `@route` / `@model` per [`DOCUMENTATION_STANDARDS.md`](../platform/DOCUMENTATION_STANDARDS.md) §3 — **same commit as code** (Claude Code); Cursor syncs narrative docs after ship |
| **Mission Creator UX lock** | [`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md) **Decisions log** row |
| **Mission Creator new capability** | [`feature_inventory.md`](../specs/Mission_Creator_Architecture/feature_inventory.md) FEDS row |
| **Eden parity gap closed** | [`eden/gap_analysis.md`](../specs/Mission_Creator_Architecture/eden/gap_analysis.md) ticket column + table row |
| **Frontend surface (MC route)** | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) recently shipped + [`frontend/docs/pages/mission-editor.md`](../../apps/website/frontend/docs/pages/mission-editor.md) |
| **New T-0xx in registry** | Row in [`tickets/registry.json`](../../.ai/tickets/registry.json) + `./scripts/ticket sync`; update [`docs/TAGS.md`](TAGS.md) only if contract text changed |
| **Deferred / blocked (not shipped)** | Registry `status: deferred` — **never** mark `shipped` until verified |
| **Doc-only reorg** | Own T-0xx commit; §Status note if authority changed |

---

## Mission Creator triggers (quick)

| Trigger | Doc |
|---------|-----|
| Layout / interaction / chrome decision | `agent_execution.md` Decisions log |
| New user-facing editor feature | `feature_inventory.md` |
| Engineering contract (schema, compiler, workers) | `engineering_plan.md` |
| Eden UI parity | `eden/gap_analysis.md` + `eden/ui_anatomy.md` |
| Shipped git milestone | Registry `shipped` + sync + §Status + MC ROADMAP + frontend ROADMAP + mission-editor + gap_analysis + feature_inventory + agent_execution as applicable |

Shell **T-033–T-040** shipped. Scale program **T-057–T-067** shipped. Next: **T-068+** per [`TICKET_LEAD.md`](../TICKET_LEAD.md).

### Mission Creator slice workflow

1. **Spec** — **Cursor** writes/updates `t0xx_*.md`; registry row `status: ready`; `./scripts/ticket sync`.
2. **Code** — **Claude Code** implements from spec; `cd frontend && npm run build && npm run lint`; `make test-it` when backend touched.
3. **Docs (same commit)** — **Cursor only** — registry `shipped` + sync + narrative rows above. Claude Code returns verify output; Cursor flips acceptance checkboxes.

---

## Never update

- `docs/specs/**/code.html`, `screen.png` mockups (archive)
- `frontend/src/stitch-exports/**` (archive)
- `artifacts/eden-wiki/*.md` (generated — re-run scraper instead)
- Generated `docs/TICKET_*.md` (edit registry + sync instead)
- Historical T-0xx bullets in CLAUDE (commit archaeology)

Live UI authority: `frontend/src/pages/` + `frontend/src/features/`.

---

## Verify before commit

```bash
cd frontend && npm run build && npm run lint
# Backend changes:
make test-it   # when API/DB touched
./scripts/ticket check   # when registry or authority docs changed
```

---

## Commit conventions

- **Single-ticket mode:** commit directly to **`main`** (no feature branches).
- **Batch pipeline mode** ([`tickets/README.md`](../../.ai/tickets/README.md)): docs on **`main`** (Composer 2.5 only); code on **`ticket/T-0xx`** until human merge; post-merge `./scripts/ticket done` + doc sync on `main`.
- Tag messages **T-0xx** at start.
- End with `Co-Authored-By:` trailer when using AI.
- **Do not commit** unless the user explicitly asks.

---

## Examples

**T-048 (library create dialog):** code + `mission-library.md` + sidebar.md + INDEX + both ROADMAPs + registry `shipped` + sync + t048 spec acceptance.

**Internal refactor (no UX/API change):** code only; no doc updates unless `Live source:` path moves.

**Blocked work:** registry `status: deferred` (e.g. **T-085**, **T-086**); do **not** mark `shipped` until implemented and verified.
