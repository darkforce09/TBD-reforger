# CLAUDE.md — TBD Reforger Platform

Working context for AI sessions. Read this first; it is the source of truth for
**current state and how to run things**. Design specs live under [`docs/`](docs/website/README.md)
(`docs/website/platform/context_handoff.md`, [`docs/website/backend/architecture.md`](docs/website/backend/architecture.md) — archive) — verify against
live code for post-T-008 behavior.

## HARD GATE — No deferrals without explicit operator word

**Do the whole ask.** Do not "fold forward", invent Out-of-scope, ship an MVP and call the
program done, or write a verify-log DEFERRED section instead of code — unless the **operator
explicitly** said to defer that piece ("defer X", "skip X", "not this pass"). Soft plan language
("if feasible", "optional", "P1 later") and **agent-authored** deferral lists are **not** permission.

Full rule: [`.cursor/rules/no-silent-deferrals.mdc`](.cursor/rules/no-silent-deferrals.mdc).
Applies to Claude Code, Cursor, Fable, and any finish/audit plan.

## What this is
A web suite for the "TBD" Arma Reforger milsim community: Discord auth, event /
ORBAT scheduling, a mission library (2D editor payloads), server telemetry +
leaderboards, doctrine wiki, CMS, and admin tooling.

- **Backend:** Rust (Axum + sqlx), PostgreSQL — crate `website-api` in `apps/website/api/` (the T-145 Go→Rust rewrite).
- **Frontend:** Leptos 0.8 CSR (Rust→wasm, Trunk) in `apps/website/frontend/` — the T-159 rewrite; the React app was deleted at T-159.29.3. All tooling is Rust (T-165 Node eradication); Node exists solely as the `enfusion-mcp` runtime (`scripts/mod`).
- **Mod:** Enfusion framework in `apps/mod/tbd-framework/`; shared mission schema in `packages/tbd-schema/`.
- **Auth:** Discord OAuth2 → JWT access token + rotating single-use refresh token.

## Monorepo layout
- `apps/website/` — app nest: `api/` (Axum, pkg `website-api`) + `frontend/` (Leptos Trunk, pkg `website-frontend`); seeds at `api/seeds/`
- `apps/mod/` — Enfusion mod framework (`tbd-framework`, gitignored `crf_framework`/EnfusionMCP)
- `packages/tbd-schema/` — mission JSON schema + golden missions
- `packages/map-assets/` — terrain DEM/sat (LFS) + rebuildable staging/tiles; served by API `/map-assets`
- `docs/specs/` — design specs (Mission Creator, blueprints); `docs/mod/`, `docs/website/` — app docs (frontend surface specs: `docs/website/frontend/pages/`, not under `apps/`)
- `scripts/mod/`, `scripts/website/`, `scripts/deploy/` — ops scripts (dev/staging/deploy); **`scripts/mod/mcp-call.sh`** + warm daemon for Workbench MCP (see [`docs/mod/MCP_TOOLING.md`](docs/mod/MCP_TOOLING.md))
- `.ai/tickets/` + `scripts/ticket` — unified ticket registry at repo root; `.ai/artifacts/` pipeline **output only** (fixtures live crate-local `tests/fixtures/` — see [`WHERE_DOES_X_GO.md`](docs/platform/WHERE_DOES_X_GO.md))
- `apps/website/api/src/bin/api.rs` — entrypoint: loads `.env`, runs migrations on boot, serves `/api/v1`.
- `apps/website/api/src/handlers/` — Axum HTTP handlers, one file per resource (auth, missions, events, telemetry, admin, …).
- `apps/website/api/src/models/` — serde models; **JSON field names (snake_case) here are the API contract**.
- `apps/website/api/migrations/` — sqlx SQL migrations (extensions, enums, indexes, leaderboard MV).
- `apps/website/api/src/{services,middleware,realtime}/` — logic core, auth tiers, SSE hub.
- `apps/website/frontend/src/` — one module per page + `client.rs` (gloo-net + single-flight refresh), `dto.rs` (API DTOs, R-api golden-tested), `mission_editor.rs` + editor modules (wgpu engine via `map-engine-render`).

## Run it locally
Everything is configured in `apps/website/api/.env` (`APP_ENV=development`, DB on port 5434). Cargo lives at `~/.cargo/bin`; the root `Makefile` prepends it (plus `~/go/bin` for the editorconfig-checker binary only).

```bash
make db-up         # start local Postgres (podman/docker compose), port 5434
make api           # run the Rust API on :8080 (cargo run --bin api; migrates on boot)
make leptos        # Leptos on :3000 — trunk serve --release (T-173 P8; day-to-day perf path)
make leptos-debug  # debug wasm rebuilds only — editor FPS NOT representative
make test-it       # Rust integration tests (needs db-up; sets TEST_DATABASE_URL)
make db-down       # stop Postgres (keeps volume)
```

Frontend checks: `make ci-local-leptos` (fmt + clippy wasm32 + cargo test + trunk release build); full editor gates: `make leptos-gates` (T-177: runs **`gate doctor`** first — see [`EDITOR_GATE_RUNBOOK.md`](docs/website/EDITOR_GATE_RUNBOOK.md); full Chrome `--headless=new`, not `chrome-headless-shell`). Toolchain pin: root [`rust-toolchain.toml`](rust-toolchain.toml) (**1.95.0**). Editor HUD shows `rf <ms>`; console `window.__editorBench(500)` for local pan/zoom encode samples (T-173).

### Dev login (no Discord needed)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted`.
It mints a real session and 302-redirects to the SPA callback exactly like Discord —
open it in the browser to log in, or curl it and read `access_token` from the
`Location` fragment for API testing.

## Conventions
- **Where does X go?** — [`docs/platform/WHERE_DOES_X_GO.md`](docs/platform/WHERE_DOES_X_GO.md) (T-171 pin: SPA pages, handlers, migrations, seeds, fixtures, map-assets, tickets).
- API JSON is **snake_case** (from serde field names). The Rust models in `apps/website/api/src/models/`
  are the snake_case DB/API source of truth, and the Leptos `dto.rs` DTOs mirror them (R-api golden
  round-trip tests) — when changing a model, update the matching DTO. Cross-boundary **contract** types are **generated** from
  `packages/tbd-schema/schema/*.json` via `make schema-codegen` into
  `apps/website/api/src/contract/generated/` (DO NOT EDIT; T-123.4 — Rust-only since the T-159.29.3
  React deletion; the Leptos SPA hand-writes `dto.rs` gated by R-api golden tests). The mission
  **export** JSON (`/missions/:id/export`) is the one camelCase exception.
- List endpoints return `{data, total, limit, offset}` (audit logs use a `next_cursor`).
- Auth tiers: public, `RequireAuth` (JWT), `RequireMinRole(admin|mission_maker)`,
  `RequireServiceToken` (`X-Service-Token`, for game-server ingest).
- Refresh tokens are **single-use** (rotated + revoked each call). All refreshes go
  through one single-flight helper (`apps/website/frontend/src/client.rs`) so the token is
  never double-spent.
- Git: **commit directly to `main`; never create a branch** (single-ticket mode). End commit messages with
  the `Co-Authored-By` trailer. Commits are tagged `T-00x`.
- **Ticket pipeline** ([`.ai/tickets/README.md`](.ai/tickets/README.md)): all work happens **directly on `main` — no branches** (supersedes the old `ticket/T-0xx` flow). Composer 2.5 owns doc writes/sync; Claude Code ships code + in-code comments; the registry is source of truth (`./scripts/ticket sync`).
- **Documentation standards:** [`docs/platform/DOCUMENTATION_STANDARDS.md`](docs/platform/DOCUMENTATION_STANDARDS.md) — cross-boundary `@contract` / `@route` / `@model`, codegen + validation + CI (**T-123**).
- Docs: see **§Documentation** — sync before commit. Ticket queue: [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md).

## Documentation

Keep docs in sync **in the same commit** as the code change (or immediately before — never merge stale docs).

**Agent split (2026-06):** **Cursor (Composer 2.5)** owns all documentation writes and sync. **Claude Code** reads specs and ships code only — return verify output to Cursor for doc updates. See [`agent_execution.md`](docs/specs/Mission_Creator_Architecture/agent_execution.md) §Agent roles and [`docs/website/AGENT_COMMIT_CHECKLIST.md`](docs/website/AGENT_COMMIT_CHECKLIST.md).

> **SUPERSEDED FOR THE PLATFORM FACTORY (2026-07-26).** Claude Code's budget ran out after wave 5, so
> **Cursor + Grok 4.5 now runs the T-182…T-297 platform factory and owns application code there**, via
> slice agents in `slice/T-XXX` worktrees. The 2026-06 split above still governs all *other* work.
> Runbook: [`docs/platform/FACTORY_FOR_CURSOR.md`](docs/platform/FACTORY_FOR_CURSOR.md) · mode switch:
> [`.cursor/rules/platform-factory-mode.mdc`](.cursor/rules/platform-factory-mode.mdc).

**CRITICAL — Executor gate:** Agents may **ONLY** execute ticket slices where `executor` is `claude-code` (Claude Code) or `cursor-docs` (Cursor documentation pass). If the active slice has `executor: workbench`, `human`, or `ci`, the agent **must stop** and wait for human completion. Do not edit `apps/mod/tbd-framework` Enfusion scripts unless the slice explicitly assigns `claude-code` to a mod script path. `./scripts/ticket run` skips non-`claude-code` rows automatically.
**In platform-factory mode, `executor: claude-code` means "any AI coding agent may take this" — it is not a vendor claim,** and Grok now fills that role. Do **not** mass-edit the 95 open platform tickets to `cursor-docs`. `workbench` and `human` still mean stop.

**Before every T-0xx commit, check what changed:**

| Change type | Update |
|-------------|--------|
| Shipped feature / milestone | **§Status** — new T-0xx bullet under **Done**; bump `latest shipped` line |
| **Active slice** (code in progress, not shipped) | **§Status — ACTIVE SLICE** block at top; keep `latest shipped` on last **git tag** only |
| New/changed route | Matching `docs/website/frontend/pages/*.md` + row in `docs/website/frontend/INDEX.md`; verify against `apps/website/frontend/src/router.rs` |
| UI surface (no new route) | Relevant page doc + `Live source:` path to the `apps/website/frontend/src/` page module |
| API / model change | Backend model/handler + the matching `apps/website/frontend/src/dto.rs` DTO (R-api golden); note handler if behavior changed |
| Mission Creator | MC README, `agent_execution.md` Decisions log, and/or `feature_inventory.md` — only if editor contract or Eden parity changed |
| Deferred / queued work | [`.ai/tickets/registry.json`](.ai/tickets/registry.json) row `status: deferred` or `queued` — sync via `./scripts/ticket sync`; never mark shipped until verified |

**Doc hub:** [`docs/website/README.md`](docs/website/README.md) → [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md) → domain **`ROADMAP.md`** files. Tag contract: [`docs/website/TAGS.md`](docs/website/TAGS.md). **Commit checklist:** [`docs/website/AGENT_COMMIT_CHECKLIST.md`](docs/website/AGENT_COMMIT_CHECKLIST.md).

**Do not update** blueprint HTML, stitch exports, or mock-up HTML — archive tier only. Live UI = `apps/website/frontend/src/` (the React app was deleted at T-159.29.3).

**Doc-only commits** (reorgs, typo fixes) get their own T-0xx tag and a §Status note if structure or authority changed.

## Ticket operations

**Source of truth:** [`.ai/tickets/registry.json`](.ai/tickets/registry.json). **Lead view:** [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md). **Full table:** [`docs/TICKET_REGISTRY.md`](docs/TICKET_REGISTRY.md).

| Step | Command / doc |
|------|----------------|
| Edit queue / status / spec | Edit `.ai/tickets/registry.json` |
| Regenerate views + CLAUDE status block | `./scripts/ticket sync` (or `make ticket-sync`) |
| Validate structure | `./scripts/ticket check` |
| Strict legacy-ID scan | `make ticket-check-strict` |
| Operator playbook | [`.ai/tickets/AI_PLAYBOOK.md`](.ai/tickets/AI_PLAYBOOK.md) |
| Claude Code brief | `./scripts/ticket brief T-0xx` |
| Batch implement | `./scripts/ticket run` on `main` (claude-code slices only) |
| Mod / Workbench queue | [`docs/TICKET_MOD_QUEUE.md`](docs/TICKET_MOD_QUEUE.md) |
| Advance slice | `./scripts/ticket advance-slice T-0xx` |

Do **not** hand-edit generated `docs/TICKET_*.md` or the `<!-- ticket-sync:status -->` markers — change the registry and sync.

## Status

> **ACTIVE PROGRAM (2026-07-26): the platform factory — T-182…T-297.**
> Process: [`docs/platform/PLATFORM_FACTORY.md`](docs/platform/PLATFORM_FACTORY.md) ·
> wave plan: [`docs/platform/wave_plan.tsv`](docs/platform/wave_plan.tsv).
> **T-181 is complete** (54 slices; the mod boots, all five screens open, objectives/radio/
> play-area/briefings/markers all run). Both **T-068** and **T-181** are `deferred` because their
> only remaining slice is `executor: human` — a live two-client E2E on a dedicated server. Do not
> pick either up; they are not agent-actionable.
> The generated block below is derived from the registry and lags this note by design — it reports
> the lowest-ordered `ready` program, not the program in flight.

<!-- ticket-sync:status:start -->
**Latest shipped:** **T-780**

**ACTIVE NOW:** **T-090** — T-090.6 (Map visualization program). Slice spec: `docs/specs/Mission_Creator_Architecture/t090_6_geometry_placement_audit.md`.

**Next (by order):**
- **T-090** — Map visualization program (`ready`)
- **T-120** — Staging soak + golden mission smoke (`queued`)
- **T-146** — Asset Browser Data Wiring (`queued`)
- **T-170** — Prod default flip to Leptos SPA (`queued`)
- **T-673** — Marker style and Area markers — the $defs/marker widening (`queued`)
- **T-674** — T-216 follow-on: slot identity reaches the wire (`queued`)
- **T-675** — Vehicle roster reaches the game — the compile half of T-076 (`queued`)
- **T-676** — Trigger activation and effects — the Enfusion runtime (`queued`)
- **T-677** — Waypoints — group movement orders (`queued`)
- **T-678** — Group AI state: combat mode, behaviour, formation, speed (`queued`)
<!-- ticket-sync:status:end -->

**Shipped history — every slice, sha and tag — lives in**
[`docs/platform/SHIPPED_HISTORY.md`](docs/platform/SHIPPED_HISTORY.md).
It was moved out of this file on 2026-08-07: it was 86% of CLAUDE.md and ~98% archive, and it
described the deleted Go backend and React/TypeScript frontend in the present tense. Nothing was
dropped — the relocation is verified entry-for-entry. Git tags (`git tag -l 'T-*'`) remain the
independent index.

**Current environment notes:**
- Discord OAuth credentials ARE populated in `apps/website/api/.env` (client id, secret, guild id); `DISCORD_BOT_TOKEN` and `DISCORD_WEBHOOK_URL` are empty and the bot token is read by no consumer. Dev still uses dev-login by default. Runbook: [`DEV_RUNBOOK.md`](docs/website/DEV_RUNBOOK.md) — and note `FRONTEND_URL`/`DISCORD_REDIRECT_URL` host mismatch (T-303) breaks the first live login.
- Telemetry is ingested via service-token endpoints; no live game-server bridge wired.
- A fresh DB is empty of content (events, missions, etc.) — seed those via the API
  or `psql`. The committed seeds live at `apps/website/api/seeds/`: `discord_roles.sql` +
  `registry_dev.sql` via `make seed`; `mock_data.sql` (Operation Red Dawn etc., four fixed
  UUIDs) is **manual `psql` only** (the Go `cmd/seed` applier was deleted at T-145). DEV_RUNBOOK.md
  has the DELETE SQL to purge those mock missions if they leak into the live library.

## Verifying changes
Source of truth for the API contract is the Axum handlers + `apps/website/api/src/models/`;
the Leptos `dto.rs` yields to the backend on conflict. To check a wire change for real, run the
stack, `dev-login`, hit the endpoint, and confirm the JSON round-trips through the DTO — the
R-api golden tests (`cargo test -p website-frontend`) pin this against committed captures.

**Platform CI replay:** `make db-up` → **`make ci-local`** (mirrors
[`ci.yml`](.github/workflows/ci.yml): verify-editorconfig, verify-no-python, rust-ci (cargo
fmt/clippy/build + wasm-ci + test-it), verify-coding-standards, ci-local-leptos (fmt + clippy
wasm32 + cargo test + trunk release), schema validate + citations). See
[`CODING_STANDARDS.md`](docs/platform/CODING_STANDARDS.md) §11.
