# Agent commit checklist

**Use on every feature commit.** Sync docs **in the same commit** as code — never merge stale docs.

**Authority ladder:** running code → [`CLAUDE.md`](../../CLAUDE.md) §Status → [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) → domain **ROADMAP.md** → supporting docs → archive.

**Doc ownership (locked 2026-06):** **Cursor (Composer 2.5)** writes and syncs all documentation. **Claude Code** reads docs and implements code only — return verify output to Cursor for the §Same-commit sync pass before the human commits.

**Where does X go?** [`docs/platform/WHERE_DOES_X_GO.md`](../platform/WHERE_DOES_X_GO.md) (T-171 pin).

---

## Ticket registry workflow

1. **Plan / queue change** — edit [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) (status, order, spec path, `active_slice`).
2. **Regenerate views** — `./scripts/ticket sync` (updates `docs/TICKET_*.md`, `CLAUDE.md` status markers).
3. **Validate** — `./scripts/ticket check` or `make ticket-check-strict`.
4. **Implement** — Claude Code on **`main`**; **does not edit docs**.
5. **Ship** — human verifies → set row `status: shipped` → `./scripts/ticket sync` → Cursor syncs narrative docs below.

Playbook: [`.ai/tickets/AI_PLAYBOOK.md`](../../.ai/tickets/AI_PLAYBOOK.md). Lead view: [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md).

---

## Before you code

| Domain | Start here |
|--------|------------|
| **Any work** | [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) → registry row → spec path |
| **Frontend surfaces** | [`docs/website/frontend/ROADMAP.md`](frontend/ROADMAP.md) → [`INDEX.md`](frontend/INDEX.md) |
| **Mission Creator** | MC [`ROADMAP.md`](../specs/Mission_Creator_Architecture/ROADMAP.md) → [`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md) |
| **Backend / API** | [`docs/website/backend/ROADMAP.md`](backend/ROADMAP.md) · live code `apps/website/api/` |
| **Conventions pin** | [`WHERE_DOES_X_GO.md`](../platform/WHERE_DOES_X_GO.md) |
| **Cross-boundary comments** | [`DOCUMENTATION_STANDARDS.md`](../platform/DOCUMENTATION_STANDARDS.md) |
| **Coding standards** | [`CODING_STANDARDS.md`](../platform/CODING_STANDARDS.md) — before commit: `make db-up` then `make ci-local` |
| **Tag contract** | [`docs/TAGS.md`](TAGS.md) |

---

## Same-commit sync table

| What changed | Update these |
|--------------|--------------|
| **Shipped milestone** | Registry → `shipped`; `./scripts/ticket sync`; [`CLAUDE.md`](../../CLAUDE.md) §Status Done bullet |
| **Active slice** | Registry `active_slice`; MC `agent_execution.md` if applicable |
| **New or removed route** | [`apps/website/frontend/src/router.rs`](../../apps/website/frontend/src/router.rs) + [`pages/*.md`](frontend/pages) + [`INDEX.md`](frontend/INDEX.md) + [`ROADMAP.md`](frontend/ROADMAP.md) |
| **UI surface (no route)** | Page spec **Element Inventory** + **`Live source:`** → `apps/website/frontend/src/<page>.rs` |
| **Nav / sidebar** | [`apps/website/frontend/src/nav.rs`](../../apps/website/frontend/src/nav.rs) + [`shell/sidebar.md`](frontend/shell/sidebar.md) |
| **API / model** | `apps/website/api/src/models/` + matching `apps/website/frontend/src/dto.rs` (R-api golden) |
| **Cross-boundary type/handler** | `@contract` / `@route` / `@model` per DOCUMENTATION_STANDARDS — same commit as code |
| **Mission Creator** | Decisions log / feature_inventory / gap_analysis as applicable |
| **Deferred** | Registry `status: deferred` — never mark shipped until verified |
| **Doc-only reorg** | Own T-0xx commit; §Status note if authority changed |

---

## Mission Creator slice workflow

1. **Spec** — Cursor writes `t0xx_*.md`; registry `ready`; `./scripts/ticket sync`.
2. **Code** — Claude Code; `make ci-local-leptos` (+ `make test-it` when API touched).
3. **Docs** — Cursor: registry `shipped` + sync + narrative rows.

---

## Never update

- `docs/specs/**/code.html`, `screen.png` mockups (archive)
- Generated `docs/TICKET_*.md` (edit registry + sync)
- Historical T-0xx bullets in CLAUDE (commit archaeology)
- **Do not create** markdown under `apps/**/docs/` or `packages/**/docs/` — specs live in [`docs/website/frontend/`](frontend/)

Live UI authority: `apps/website/frontend/src/` (Leptos page modules).

---

## Verify before commit

```bash
make ci-local-leptos   # fmt + clippy wasm32 + cargo test + trunk release
make test-it           # when API/DB touched (needs make db-up)
./scripts/ticket check # when registry or authority docs changed
```

---

## Commit conventions

- Commit directly to **`main`** (no feature branches; old `ticket/T-0xx` flow retired).
- Tag messages **T-0xx** at start.
- End with `Co-Authored-By:` trailer when using AI.
- **Do not commit** unless the user explicitly asks.
