# T-123 — Documentation standards rollout (full program)

**Ticket:** T-123 · **Authority:** [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) (normative — verified 2026-06-29)  
**Program:** platform · **One ticket** — seven slices, ship in order  
**Status:** **shipped @ `169e47d`** — .0 `f0af31a` · .1 `04a73a1` · .2 `030cece` · .3 `169e47d` · .4 `dd4e4d0` · .5 `b5211f2` · .6 `102a835` (CI green @ `7a08a8f`)

## In one sentence

Implement the **entire** documentation standards program: cross-boundary tags + Godoc/TSDoc/Enfusion comments, schema codegen, Go-side JSON validation on mission versions, and CI gates — one branch, one ticket.

## Authority

[`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) (§0–§11). The standard doc is **already written**; T-123 **implements** it (including §9 codegen, §9.2 validation, §10 CI — no longer deferred).

**Out of scope:** markdownlint (not installed; add only if a future ticket approves the dep).

---

## Slice plan

| Slice | Executor | Scope |
|-------|----------|-------|
| **T-123.0** | cursor-docs | Doc hub wiring + handoff |
| **T-123.1** | claude-code | Go Godoc + `@contract` / `@route` |
| **T-123.2** | claude-code | TS TSDoc + `tsdoc.json` + tags |
| **T-123.3** | claude-code | Enfusion Backend/Gamemode docs + authority tags |
| **T-123.4** | claude-code | Schema codegen (Go + TS projections) |
| **T-123.5** | claude-code | Go-side JSON Schema validation (`CreateVersion`) |
| **T-123.6** | claude-code | CI gates (revive, eslint jsdoc, citation verifier) |

Advance after each slice verifies: `./scripts/ticket advance-slice T-123`

**Branch:** `ticket/T-123` · Claude: `./scripts/ticket run` (skips cursor-docs rows)

---

## T-123.0 — Doc hub wiring (Cursor)

**Edit (docs only):**

- [`docs/website/AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md) — link [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §11 cheat sheet; sync row for in-code comment changes
- [`CLAUDE.md`](../../CLAUDE.md) §Conventions — pointer to documentation standards
- [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §9/§10 — note implementation owner = T-123 (not “follow-up”)
- [`.ai/artifacts/t123_claude_code_handoff.md`](../../.ai/artifacts/t123_claude_code_handoff.md) — full slice order + verify matrix

**Verify:** `./scripts/ticket sync && ./scripts/ticket check --strict`

---

## T-123.1 — Go in-code docs (Claude Code)

Per §4 + §3. Priority cross-boundary surfaces:

| Area | Tags |
|------|------|
| `internal/models/registry.go`, `handlers/registry.go` | `@contract registry-items.schema.json#/$defs/item`, `@route GET /api/v1/registry` |
| Version POST payload | `@contract mission-editor-payload.schema.json#/` on `createVersionInput`; `@route` on `CreateVersion` |
| Mission export / inject | `exportFormatVersion` (not `schemaVersion`); `@route` on `ExportMission`/`InjectMission` |
| Loadout-export touchpoints | `@contract loadout-export.schema.json#` |

- Godoc starts with identifier name; **rename the export envelope's `schemaVersion` → `exportFormatVersion`** (int) so it never collides with the canonical string `schemaVersion` (§2.2)
- Package docs on touched packages

**Verify:** `make test-it && go build ./...`

---

## T-123.2 — TypeScript in-code docs (Claude Code)

Per §5.

1. Add `apps/website/frontend/tsdoc.json` — custom tags `@contract`, `@route`, `@model`, `@consumer`
2. TSDoc (not `//`-only) on exports in `src/types/models/*`, `src/types/api/*`, `src/api/*`, route hooks in `src/hooks/`
3. `@model` / `@contract` / `@route` on cross-boundary symbols

**Verify:** `cd apps/website/frontend && npm run build && npm run lint`

---

## T-123.3 — Enfusion in-code docs (Claude Code)

Per §6 + §7.

- `Scripts/Game/TBD/Backend/*` — file headers, per-field DTO docs, `@contract`
- `Scripts/Game/TBD/Gamemode/*` — `@authority`, `@rpc`, `@replicated`, server-gate comments
- Workbench registry export plugin — `@contract registry-items.schema.json#/`

**PREFLIGHT:** enfusion-mcp before any `.c` edit.

**Verify:** Workbench compile on touched scripts (human note in handoff).

---

## T-123.4 — Schema codegen (Claude Code)

Per §9.1. **Generate** projections from `packages/tbd-schema/schema/*.json`; stop hand-authoring new cross-boundary types.

**Deliver (minimum):**

| Output | Tool (pick one; document in commit) | Schemas first |
|--------|-------------------------------------|---------------|
| `apps/website/internal/contract/` | JSON Schema → Go generator (e.g. `jsonschema`/`quicktype`/`ogen` — justify choice) | `registry-items`, `loadout-export`, mission export envelope defs |
| `apps/website/frontend/src/types/contract/` | `json-schema-to-typescript` or equivalent | same set |

**Rules:**

- Generated files marked `DO NOT EDIT` + regen script in `packages/tbd-schema/scripts/` (e.g. `codegen.mjs` + root `make schema-codegen`)
- Hand-written GORM models **remain** for DB/API snake_case where they differ from export camelCase — generated types used for validation/export paths; add `@contract` bridging comments
- Enforce DTOs stay hand-written (§9.1) with `@contract` + golden fixtures

**Verify:** `npm run validate` in `packages/tbd-schema`; `make test-it`; FE build/lint

---

## T-123.5 — Go-side JSON Schema validation (Claude Code)

Per §9.2.

- Validate incoming mission version payload against `mission-editor-payload.schema.json` (the editor superset, **not** canonical `mission.schema.json`) **before persist** in `CreateVersion`, via [`internal/contract/validate.go`](../../apps/website/internal/contract/validate.go)
- Library: `santhosh-tekuri/jsonschema/v6`; schema `go:embed`-ed + compiled once (`sync.Once`)
- **400** with structured `{ error, details[] }` on validation failure; golden missions + invalid fixtures in integration tests
- Align with existing `packages/tbd-schema/scripts/validate-file.mjs` semantics

**Verify:** `make test-it` (new cases: golden pass, missing required field fail, wrong type fail)

---

## T-123.6 — CI enforcement gates (Claude Code)

Per §10. Wire all four gates:

| Gate | Deliver |
|------|---------|
| Go exported-doc | `golangci-lint` + `revive` exported rules in CI (website job or new job) |
| TS contract docs | `eslint-plugin-jsdoc` + `@microsoft/tsdoc`; rules on `src/types/`, `src/api/`, `src/hooks/` — require TSDoc + `@contract`/`@model` on cross-boundary exports |
| Citation integrity | Node script `packages/tbd-schema/scripts/verify-contract-citations.mjs` — every `@contract` in repo resolves to schema file + valid JSON pointer; shipped as a dedicated [`.github/workflows/contracts.yml`](../../.github/workflows/contracts.yml) workflow (citation + codegen-drift + golangci + eslint jobs) |
| Enfusion DTO conformance | Extend `validate.mjs` or sibling check: DTO scripts with `@contract` header have matching golden fixture |

**Verify:** CI green locally where possible (`npm run validate`, `golangci-lint run`, FE lint); citation script exits 0 on main after .1–.3 tags land

---

## Acceptance (whole ticket)

- [x] Slices **T-123.0–T-123.6** shipped; registry `status: shipped`
- [x] `@contract` / `@route` grep spot-check on priority surfaces (23 `@contract` citations resolve)
- [x] Codegen regen documented; `make schema-codegen` works (deterministic; codegen-drift CI job)
- [x] `CreateVersion` rejects invalid JSON against `mission-editor-payload.schema.json`
- [x] CI runs citation verifier + lint gates (`contracts.yml`)
- [x] `make test-it` + frontend build/lint clean
- [x] Cursor doc pass: `CLAUDE.md` §Done, backend ROADMAP, `./scripts/ticket sync`

## Decisions log

### 2026-06-29 — Single ticket; full §9–§10 in scope
- **Context:** User direction: one ticket; codegen, Go JSON validation, and CI gates are part of the same program — not split to T-124+.
- **Decision:** T-123 expands to seven slices (.0–.6); `DOCUMENTATION_STANDARDS.md` remains normative; implementation closes §9–§10.
- **Consequences:** `@contract` tags land in .1–.3; codegen (.4) and CI citation verifier (.6) depend on those tags existing.
- **Supersedes:** prior T-123 spec that deferred §9/§10.

### 2026-06-29 — Single ticket; standards doc precedes rollout
- **Context:** Hand-mirrored types lose boundary context.
- **Decision:** `DOCUMENTATION_STANDARDS.md` lands first; T-123 implements it end-to-end.
- **Supersedes:** none.
