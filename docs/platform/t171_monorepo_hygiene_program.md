# T-171 — Monorepo / website hygiene program

**Status:** SHIPPED @ tag **T-171** / `2421b335` (code + layout) · docs pass **T-171.docs**  
**Scope:** website + platform tooling/docs — **not** `apps/mod/`  
**Goal:** one coherent, thorough hygiene pass so the website/monorepo layout, dead code, tickets, docs, conventions, fixtures, and crate/path names match reality — and new work has one obvious home.

**No silent deferrals.** Soft “later / optional / separate ticket” language is forbidden unless the operator explicitly says `defer X` / `skip X`. If blocked, **ASK** — do not invent Out-of-scope.

## Why

Post T-159 / T-165 / T-166–T-169 the tree is still disjointed: SPA beside API, dual Go/sqlx migration trees, nested Go/Vite Makefiles, crate names ≠ paths, golden/fixture sprawl, stale ADRs/docs, map-assets coupling unclear, ticket/docs drift.

## Locked end-state layout

```text
apps/website/
  api/                        # Axum API crate (MOVED from apps/website/{src,migrations,…} root)
  frontend/                   # Leptos Trunk SPA (MOVED from apps/website-leptos/)
crates/map-engine-*
tools/tbd-tools/
packages/{tbd-schema,map-assets}/
docs/                         # Cursor prose pass from Claude’s return list (T-171.docs)
.ai/tickets/                  # registry SoT
```

**Names (mandatory alignment — not vanity “later”):**

| Path | Cargo package name |
|------|--------------------|
| `apps/website/api` | `website-api` (shipped; was `reforger-backend`) |
| `apps/website/frontend` | `website-frontend` (shipped; was `website-leptos`) |

Update every `-p`, CI job, Makefile target, and doc string in the same ship. Prefer clear names over preserving obsolete package ids.

**Mod:** `apps/mod/**` OFF LIMITS (operator: not the mod).

## Agent split (HARD)

| Who | Owns |
|-----|------|
| **Claude Code** | Full structural + dead-code + tooling + fixture consolidation + map-assets consumption story + inventory; gates green; tag T-171 |
| **Cursor (T-171.docs)** | Applies Claude’s return list to docs/, rules, CLAUDE.md, registry summaries, ADRs/conventions pins |

Claude does not hand-edit `docs/**` / registry / CLAUDE sync markers — **returns a complete fix list that Cursor must apply as part of finishing the program** (not a deferral of content; a role split). Operator still gets the whole outcome.

## Phases (all must-do)

### Phase 0 — Inventory (Class-R) → `.ai/artifacts/t171_inventory.md`

No deletes until this exists. Tables:

1. Layout debt (website → `api/` + `frontend/`)
2. Dead code SAFE / UNSAFE / ASK (`internal/**`, nested Makefile, `__pycache__`, …)
3. Dual SoT (migrations/seeds, context_handoff twins, ROADMAPs)
4. Crate/path rename blast radius (`reforger-backend`, `website-leptos`, CI job ids)
5. Golden/fixture homes (`t159_gates/fixtures`, `tbd-schema/golden*`, engine `tests/fixtures`) — target one convention
6. map-assets: how CI/dev/prod consume DEM vs sat LFS; what must change so clone/CI isn’t a landmine
7. Doc/rule/ADR rot (Vite, React, Go, Deck “read first”, ticket branch lore)
8. Ticket hygiene
9. Conventions gaps (“where does page / handler / smoke / ticket / migration / fixture / asset go?”)

### Phase 1 — Layout

- Move SPA → `apps/website/frontend/`
- Move API crate root → `apps/website/api/` (src, migrations, tests, Cargo.toml, rust-toolchain as needed)
- Rename packages to `website-frontend` / `website-api` (or document the single exception with operator ASK if truly blocked)
- Root workspace, Makefile, CI, Trunk, compose, `.env.example`, scripts, gates — all paths updated
- Prove `make api` + `make leptos`; `/map-assets` 200; FRONTEND_URL `:3000`

### Phase 2 — Dead code + dual-SoT

- SAFE deletes only, with evidence
- Relocate still-used seeds; purge dead Go `internal/`
- Kill nested Go/Vite Makefile (pointer to root OK)
- Purge eradication debris (`scripts/__pycache__`, …)
- Fix website scripts / deploy env examples still saying `go run` / `npm`

### Phase 3 — Fixtures + map-assets story

- Consolidate golden/fixture **homes** to a documented convention (move or re-home with path updates; no “leave sprawl”)
- Document + implement the map-assets consumption story (CI selective LFS, local sat, gate `map_assets_dir`) so the monorepo doesn’t stay accidentally coupled to the wrong pull set — **finish the story in this ticket**, don’t fold to a future packaging program unless operator says `defer map-assets`

### Phase 4 — Tooling one story

- `./scripts/ticket *`, `make leptos-gates`, `make ci-local`, `make verify-no-node`
- CI job names/paths match `api` + `frontend`

### Phase 5 — Return for Cursor (T-171.docs) — complete list

Every stale doc/ADR/rule/ROADMAP/checklist/CLAUDE path string + **“Where does X go?”** conventions pin content. Cursor applies immediately after; program not “done” until that pass lands (operator/Cursor), but Claude’s ship tag covers structural verify.

## Verify (Claude, before tag T-171)

```bash
make leptos-gates
make ci-local
make verify-no-node
./scripts/ticket check
test -d apps/website/frontend && test -f apps/website/frontend/Trunk.toml
test -d apps/website/api && test -f apps/website/api/Cargo.toml
test ! -e apps/website-leptos
# API not left as dual root with src/ still at apps/website/src (unless thin wrapper — prefer clean)
```

## Claude Code prompt

See [`.ai/artifacts/t171_claude_code_handoff.md`](../../.ai/artifacts/t171_claude_code_handoff.md).
