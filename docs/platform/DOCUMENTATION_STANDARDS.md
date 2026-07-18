# TBD Reforger — Documentation Standards

**Status:** living
**Audience:** every engineer and AI agent that writes Rust, Enfusion, or tooling code in this monorepo
**Authority:** Running code → [`CLAUDE.md`](../../CLAUDE.md) → [`docs/website/README.md`](../website/README.md) → **this doc** (supporting tier)
**Updated:** 2026-07-18 (T-171 path refresh)

> **Live stack (T-145 / T-159 / T-171):** `apps/website/api/` (Axum + sqlx) + `apps/website/frontend/` (Leptos). Go/TS examples below are **historical patterns** for `@contract` / `@route` vocabulary — prefer Rust rustdoc + clippy today. Homes: [`WHERE_DOES_X_GO.md`](WHERE_DOES_X_GO.md).

> This document is the source of truth for **how code is documented** across the three
> boundaries of `TBD-Reforger`. It is **ruthless and prescriptive**: where it says REQUIRED,
> non-conforming code is a defect to fix on next edit; where it says FORBIDDEN, the pattern must
> not be introduced. It defers to running code (the authority ladder above) and never overrides a
> rule in [`CLAUDE.md`](../../CLAUDE.md) or the [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md).

---

## 0. Why this exists

The monorepo spans three hard boundaries:

| Boundary | Language | Role |
|----------|----------|------|
| [`packages/tbd-schema`](../../packages/tbd-schema) | JSON Schema (draft 2020-12) | **The source of truth.** Declares every cross-boundary data contract. |
| [`apps/website`](../../apps/website) | Rust: `api/` (Axum) + `frontend/` (Leptos) | API server + SPA. |
| [`apps/mod`](../../apps/mod) | Enfusion / Enforce Script (`.c`) | The Arma Reforger game framework. |

A single concept — a mission, a loadout, a registry item — is declared in **four** places (schema,
Go struct, TS interface, Enforce DTO). **Go and TS contract projections are generated** from
`packages/tbd-schema` (T-123); Enforce DTOs stay hand-written with `@contract` + golden fixtures
(Enforce has no codegen). GORM models remain the snake_case DB/API source of truth. The result
without tags: a developer reading an Enfusion DTO cannot mechanically discover which Go route feeds
it or which schema defines it. **Architectural context is lost at every boundary crossing.** This
document fixes that with: a contract ontology (§2), a cross-boundary hyperlink vocabulary (§3),
strict per-language syntax (§4–§6), mandatory network-authority tagging in Enfusion (§7), a
decision-record tier (§8), codegen + validation (§9), and CI gates that enforce all of it (§10).

---

## 1. Ownership & the agent split

[`CLAUDE.md`](../../CLAUDE.md) §Documentation and [`.cursor/rules/`](../../.cursor/rules) lock an
agent split: **Cursor owns documentation, Claude Code owns code.** That split is silent on one
thing — **in-code comments** — which this section resolves:

- **In-code doc comments are CODE.** Godoc comments, TSDoc blocks, and Enforce `//!`/`/** */`
  banners are authored and edited by **Claude Code**, in the **same commit** as the code they
  describe (the same-commit rule in [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md)).
- **Standards & decision markdown are DOCS.** This file, the per-spec Decisions logs (§8), and
  anything under `docs/` are owned by **Cursor**.
- Authoring *this* file is a one-time, user-directed exception to "Cursor owns docs"; future
  edits to it follow the split (Cursor).

A code change that adds or alters a documented symbol **must** update its in-code comment in the
same diff. A stale doc comment is treated as a bug, not a cosmetic issue.

---

## 2. The contract ontology

**Rule 2.1 — Single source of truth.** Every data shape that crosses a boundary is **defined
once** in [`packages/tbd-schema/schema/*.json`](../../packages/tbd-schema/schema). Go structs, TS
interfaces, and Enforce DTOs are **projections** of a schema definition — never the origin of one.
A new cross-boundary field is added to the schema **first**.

**Rule 2.2 — A field's type is fixed by its schema definition.** Every projection MUST match the
schema's declared type exactly. The rule is **scoped per artifact** — there are three version
namespaces and they do not collide:
- **Canonical mission document** ([`mission.schema.json`](../../packages/tbd-schema/schema/mission.schema.json),
  consumed by the Enfusion mod loader): `schemaVersion` is a **string enum** (`"1.0"`, `"1.1"`).
  Every Go/TS/Enforce projection of a *canonical mission* MUST type it as a **string**.
- **Editor payload** ([`mission-editor-payload.schema.json`](../../packages/tbd-schema/schema/mission-editor-payload.schema.json),
  the `POST /missions/:id/versions` `json_payload` superset): `schemaVersion` is an **integer**
  (editor format version) — a distinct namespace from the canonical string.
- **Export envelope** (`GET /missions/:id/export` / inject — `missionJSON`/`MissionExport`): carries
  **`exportFormatVersion`** (integer), **not** `schemaVersion`, keeping it off the canonical key
  (renamed in T-123.1). Type drift *within* a namespace is a defect.

**Rule 2.3 — Wire casing is per-artifact and fixed.** Casing is not a matter of taste; it is
nailed down per artifact and enforced by codegen (§9). This table is normative:

| Artifact | Casing | Authority |
|----------|--------|-----------|
| REST API request/response bodies (`/api/v1/**`) | **snake_case** | GORM struct tags = the API contract ([`CLAUDE.md`](../../CLAUDE.md) §Conventions) |
| Mission **export** envelope (`GET /missions/:id/export`; version field **`exportFormatVersion`**, int — T-123.1) | **camelCase** | the one documented exception |
| `mission-editor-payload.schema.json` (`POST /missions/:id/versions` payload; int `schemaVersion`) | **camelCase** | schema source |
| `mission.schema.json`, `loadout-export.schema.json` | **camelCase** | schema source |
| `registry-items.schema.json` — envelope vs. items | envelope **camelCase**, item fields **snake_case** | schema source |
| List endpoints | `{ data, total, limit, offset }` (audit logs: `next_cursor`) | [`CLAUDE.md`](../../CLAUDE.md) §Conventions |

**Rule 2.4 — Published artifacts are immutable.** Mission versions are write-once (unique semver).
A contract change is a **schema bump + regenerate**, never an in-place edit of a published payload.

---

## 3. Cross-boundary hyperlinking — the tag vocabulary

This is the core fix for the Interconnection Problem. Four **machine-greppable** tags, usable from
any language's comment syntax. They are verified in CI (§10).

### 3.1 Grammar

```
@contract <schema-basename>#<json-pointer>   ; basename resolved under packages/tbd-schema/schema/
@route    <METHOD> <path>                     ; e.g. GET /api/v1/registry
@model    <go-type>                            ; e.g. models.User  (TS → Go GORM model)
@consumer <lang>:<repo-relative-path>          ; lang ∈ {go, ts, enf}   (OPTIONAL)
```

- **`@contract`** — **REQUIRED** on any type that projects a tbd-schema definition. The
  `<schema-basename>` is the filename only (stable, greppable; resolved against
  `packages/tbd-schema/schema/`). The `<json-pointer>` is an RFC 6901 pointer — `#/` for the
  root, `#/$defs/item` for a definition. Example: `@contract registry-items.schema.json#/$defs/item`.
- **`@route`** — **REQUIRED** on (a) the Go handler that serves the route, (b) the TS query/
  mutation hook that calls it, and (c) the Enfusion REST call site that hits it. This is the
  three-way triangulation: a Mod author greps the route string and finds the Go + TS ends.
- **`@model`** — **REQUIRED** on a TS type that mirrors a Go GORM model (the snake_case contract).
- **`@consumer`** — **OPTIONAL** reverse pointer. Forward links (`@contract`/`@route`/`@model`)
  are authoritative; reverse links rot, so they are optional and best-effort.

### 3.2 Forward links only are mandatory

A type points **up** to its source (schema / Go model / route). It does **not** have to enumerate
its consumers. Rationale: the source is stable and singular; consumers are many and churn.

### 3.3 Worked example — the `registry-items` contract

This concept exists in all three languages plus a route. Today it carries *ad-hoc* cross-refs;
the standard makes them uniform. The schema definition:

```jsonc
// packages/tbd-schema/schema/registry-items.schema.json  →  #/$defs/item
"item": {
  "required": ["resource_name", "display_name", "category", "kind"],   // snake_case items
  ...
}
```

**Go model** — already well-documented ([`internal/models/registry.go:17`](../../apps/website/internal/models/registry.go)); add the `@contract` tag:

```go
// RegistryItem is one placeable/equipable engine item in a modpack's flat catalog
// (the web Virtual Arsenal source). Identified by its full Enfusion ResourceName.
//
// @contract registry-items.schema.json#/$defs/item
type RegistryItem struct {
    ResourceName string `gorm:"column:resource_name;not null" json:"resource_name"`
    ...
}
```

**Go handler** — already names the route in prose ([`internal/handlers/registry.go:14`](../../apps/website/internal/handlers/registry.go)); make it the `@route` tag:

```go
// ListRegistry returns a modpack's flat Virtual Arsenal catalog.
//
// @route GET /api/v1/registry
// Auth: mission_maker+ (mm group). Response: { data, etag, modpack_id, modpack_version }.
func (h *Handler) ListRegistry(c *gin.Context) { ... }
```

**TS type** — today a `//` note ([`types/models/registry.ts:1`](../../apps/website/frontend/src/types/models/registry.ts)); promote to TSDoc with tags:

```ts
/**
 * One Virtual Arsenal catalog item.
 * @model models.RegistryItem
 * @contract registry-items.schema.json#/$defs/item
 * @route GET /api/v1/registry
 */
export interface RegistryItem { resource_name: string; /* ... */ }
```

**Enfusion producer** — [`TBD_RegistryItemsExportPlugin.c`](../../apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c) hand-writes the snake_case keys; its header cites the schema in prose. Standardize:

```cpp
//! Workbench export → packages/tbd-schema/registry/registry-items.workbench.json
//! @contract registry-items.schema.json#/
class TBD_RegistryItemsExportPlugin { ... }
```

The same `@contract registry-items.schema.json` string now links all four artifacts; the same
`@route GET /api/v1/registry` links the Go and TS ends. (For a file-based contract with no HTTP
route — e.g. `loadout-export`, copied as `$profile:TBD_LoadoutTest.json` — use `@contract` alone;
omit `@route`.)

---

## 4. Go — Godoc

The Go backend already sits near 100% Godoc coverage; this section **locks that baseline** and
adds the cross-boundary tags.

**REQUIRED**

1. Every exported `func`, method, type, and `const`/`var` has a doc comment, and it **starts with
   the identifier name** (Godoc convention). Gold standard: [`internal/models/mission.go:77`](../../apps/website/internal/models/mission.go),
   [`internal/handlers/handlers.go:1`](../../apps/website/internal/handlers/handlers.go).
2. Every package has a `// Package <name> …` doc on exactly one file.
3. Struct fields carry a **trailing intent comment** where the name is not self-evident
   (e.g. units, nil-meaning, enum domain). See `RegistryItem` fields above.
4. A **handler** doc comment states: `@route`, the auth tier, the response DTO name, and
   `@contract` if it (de)serializes a schema type.
5. A type that projects a schema definition carries `@contract` (§3).

**FORBIDDEN**

- `@param` / `@returns` JSDoc-style tags — not Godoc idiom. Use prose sentences.
- Restating the signature ("// GetUser gets a user"). Document *why* / *contract*, not *what the
  name already says*.

---

## 5. TypeScript / React — TSDoc

The feature code is moderately documented; the **contract layer is the gap** and the focus here:
`src/types/`, `src/api/`, `src/hooks/`.

**REQUIRED**

1. Every exported type/interface/hook/component in the contract layer has a **`/** … */` TSDoc
   block** (not `//`). Bare interfaces such as [`types/models/user.ts`](../../apps/website/frontend/src/types/models/user.ts)
   and most of [`types/api/index.ts`](../../apps/website/frontend/src/types/api/index.ts) are
   non-conforming and fixed on next touch.
2. TSDoc tags where applicable: `@param`, `@returns`, `@remarks`, `@see`.
3. Cross-boundary tags from §3: `@model` on any type mirroring a Go model; `@contract` on any
   type mirroring a schema def; `@route` on the query/mutation hook that calls an endpoint.
4. The custom block tags (`@contract`, `@route`, `@model`, `@consumer`) are declared in a
   **`tsdoc.json`** at the frontend root so `@microsoft/tsdoc` / the linter accept them.
5. A **hook** documents its query key, its `@route`, and the return shape. A **component**
   documents its props via the props interface and a one-line summary of what it renders.

**FORBIDDEN**

- `//`-only comments on exported contract-layer symbols (use TSDoc so tags are parseable).
- A TS type silently diverging from its Go model without an `@model` pointer.

---

## 6. Enfusion / Enforce Script — Doxygen

The mod has a strong house style already; this section codifies it as policy.

**REQUIRED**

1. `//!` single-line banner on **every class** and **every non-trivial method**.
2. `/** … */` **file-header block** is mandatory on every script under `Scripts/Game/TBD/Backend/`
   and `Scripts/Game/TBD/Gamemode/` (the cross-boundary + lifecycle-heavy code). Gold standard:
   [`TBD_LoadoutEquipComponent.c:1`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c).
3. Every `[Attribute(...)]` and `[ComponentEditorProps(...)]` carries a human `desc:` /
   `description:` string.
4. **DTO structs** parsed from JSON carry a `@contract` header **and a per-field doc comment** on
   every field. Bare DTO field blocks (e.g. in [`TBD_MissionSlotStruct.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c)
   and the struct block atop [`TBD_MissionLoader.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c))
   are non-conforming: the JSON-key↔field-name coupling is invisible without them.
5. A REST call site carries `@route` (§3) naming the Go endpoint it hits.

**Process note:** per [`CLAUDE.md`](../../CLAUDE.md), do not edit `apps/mod` `.c` files unless a
ticket slice explicitly assigns `claude-code` to that path, and **use `enfusion-mcp` before editing
any `.c` file** — do not guess Enforce APIs.

---

## 7. Execution context — Enfusion network authority (critical)

In a replicated game, *which machine runs a method* is part of its contract. Today this is
signalled inconsistently (an `[RplRpc]` attribute here, an `RplMode.Client` guard there, a
`// CLIENT -> SERVER` line sometimes). This section makes execution context **explicit and
mandatory**.

**REQUIRED**

1. **`//! @authority server|client|owner`** on any method whose correctness depends on which
   machine executes it.
2. Directly above every `[RplRpc(...)]` attribute: **`//! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast>`**
   (mirroring the attribute in human-readable form) **plus** a `// <SIDE> -> <SIDE>: <intent>`
   line.
3. **`//! @replicated <prop>`** on every `[RplProp]` field, naming its `onRplName` hook.
4. Every server gate — `if (RplSession.Mode() == RplMode.Client) return;` — carries a
   **`// Authority only — <reason>`** comment.

**Annotated example** (formalizing the pattern in [`TBD_MissionBrowser.c:79`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_MissionBrowser.c)):

```cpp
//! @authority owner
//! @rpc Reliable Server
// CLIENT (owner) -> SERVER: ask for the current mission list.
[RplRpc(RplChannel.Reliable, RplRcver.Server)]
void TBD_RpcAsk_MissionList() { ... }

//! @rpc Reliable Owner
// SERVER -> CLIENT (owner): reply routed only to the requesting admin.
[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
void TBD_RpcDo_ReceiveMissionList(string payload) { ... }
```

```cpp
//! @replicated m_Stage  (client UI reacts in OnStageReplicated)
[RplProp(onRplName: "OnStageReplicated")]
protected TBD_EGameStage m_Stage;
```

---

## 8. Architectural decisions — where "why" lives

Three tiers. Pick by **scope of the decision**, not by length.

| Tier | Home | Use for |
|------|------|---------|
| In-code comment | the code | A local choice / non-obvious line. Explains *this* code only. |
| **Decisions log** | the relevant `docs/specs/<area>/*.md` | A reversible-but-load-bearing architecture decision tied to a feature area. **The formal ADR home.** |
| Platform doc | `docs/platform/` | A cross-cutting standard or audit (this file; [`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md)). |

**We do NOT add a `docs/adr/` tree.** Decisions live next to their feature spec, extending the
existing Decisions-log pattern (e.g. the UX Decisions log in
[`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md)).

### 8.2 Documentation filesystem layout

**Rule 8.2.1 — Single docs root.** All markdown documentation MUST live under repo-root
[`docs/`](../../docs/). Exceptions:

- Root [`README.md`](../../README.md) and [`CLAUDE.md`](../../CLAUDE.md) (agent runtime)
- Per-package **`README.md` only** (one file, no `docs/` subtree) under `apps/*` and `packages/*`
- Generated pipeline output under [`.ai/artifacts/`](../../.ai/artifacts/) (not hand-authored specs)
- Archive tiers named in [`docs/website/archive/README.md`](../website/archive/README.md)

**Rule 8.2.2 — FORBIDDEN paths.**

- `apps/**/docs/**` (e.g. `apps/website/frontend/docs/`) — **never create**
- `packages/**/docs/**` except a single schema README adjacent to JSON (not surface specs)
- Duplicate hub trees mirroring `docs/website/` inside application folders

**Rule 8.2.3 — Frontend surface spec contract.** When adding or changing a frontend route
([`apps/website/frontend/src/router.rs`](../../apps/website/frontend/src/router.rs)):

1. Create or update [`docs/website/frontend/pages/<name>.md`](../website/frontend/pages/) from
   [`_template.md`](../website/frontend/_template.md)
2. Add a row to [`docs/website/frontend/INDEX.md`](../website/frontend/INDEX.md)
3. Update [`docs/website/frontend/ROADMAP.md`](../website/frontend/ROADMAP.md)
4. Sync per [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md)

**Rule 8.2.4 — Link style.**

- **Within the same hub:** relative paths (`pages/foo.md`, `../platform/...`)
- **From MC specs to page docs:** `../../website/frontend/pages/...` (from
  `docs/specs/Mission_Creator_Architecture/`)
- **In authority docs:** prose uses canonical `docs/website/frontend/...`; markdown hrefs may be
  relative within `docs/website/`
- **Never** use `docs/frontend/` (directory does not exist) or `frontend/docs/` (retired)

**Rule 8.2.5 — Doc tree map.**

| Doc type | Location |
|----------|----------|
| Platform standards | `docs/platform/` |
| Website hub | `docs/website/README.md` |
| Frontend surfaces | `docs/website/frontend/pages/` |
| Backend API | `docs/website/backend/` |
| Mission Creator engineering | `docs/specs/Mission_Creator_Architecture/` |
| Tickets (generated views) | `docs/TICKET_*.md` (registry + `./scripts/ticket sync`) |
| Live code | `apps/website/`, `apps/mod/`, `packages/` |

**Rule 8.2.6 — Agent routing.** Cursor owns all paths under `docs/`. Claude Code MUST NOT create
markdown under `apps/` except in-code comments per §1.

Enforced by `make verify-doc-layout` (see [`Makefile`](../../Makefile)).

**Decisions-log entry format** (normative):

```markdown
### YYYY-MM-DD — <decision in one line>
- **Context:** what forced the choice.
- **Decision:** what we chose.
- **Consequences:** what this commits us to / rules out.
- **Supersedes:** <prior entry date or "none">.
```

**Rule 8.1 — Contract changes require a paper trail.** Any decision that changes a cross-boundary
contract MUST, in the **same change**: (a) add a Decisions-log entry, (b) bump the tbd-schema
definition, and (c) regenerate the affected DTOs (§9).

---

## 9. Codegen & validation — the target state

Hand-mirroring is the root cause of boundary drift. Generation + validation is **shipped**
(T-123.4/.5/.6): Go and TS contract types are generated from the schemas, and the mission version
payload is validated server-side before persist. `@contract` (§3) is **not** a temporary measure —
it remains **permanently required on hand-written Enforce DTOs** (Enforce has no codegen).

**Mandate**

1. **Generated projections (shipped, Rust-only since T-159.29.3).** Contract types are **generated from**
   `packages/tbd-schema/schema/*.json` via `make schema-codegen`:
   - Rust → `apps/website/api/src/contract/generated/` (DO NOT hand-edit).
   - Leptos SPA hand-writes `apps/website/frontend/src/dto.rs` gated by R-api golden tests.
   - Enforce Script has no codegen tooling: Enforce DTOs stay hand-written but MUST carry
     `@contract` (§3/§6.4) **and** a golden fixture that round-trips through schema validate.
2. **API runtime validation (shipped).** `CreateVersion` validates the incoming version payload
   against [`mission-editor-payload.schema.json`](../../packages/tbd-schema/schema/mission-editor-payload.schema.json)
   **before persist** (`apps/website/api/src/contract/validate.rs`),
   returning **400** on a malformed payload. It validates the **editor superset**, not the canonical
   `mission.schema.json` — those are different artifacts (see §2.2).
3. **Hand-written types remain debt** where not generated. API wire models = `apps/website/api/src/models/` (serde snake_case).

> **Implementation:** [**T-123**](t123_documentation_standards_rollout.md) slices **T-123.4** (codegen), **T-123.5** (validation), **T-123.6** (CI).

---

## 10. CI enforcement gates

Ruthless means enforced. Primary gates live in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)
(`make ci-local`); path-filtered supplements in `contracts.yml` / `schema.yml`.

| Gate | Tool | Scope |
|------|------|-------|
| Rust API / SPA | `cargo fmt` + `clippy -D warnings` | `website-api` + `website-frontend` (`ci.yml` jobs) |
| Cross-boundary tags | citation verifier (xtask / schema CI) | `@contract`/`@route` resolve; Enfusion `@contract` |
| Enfusion DTO conformance | golden fixture + schema validate | each Backend `@contract` DTO has a validating fixture |

> **Historical (retired T-145/T-159):** golangci `exported`, eslint TSDoc — replaced by clippy + rustdoc.

The citation verifier is the keystone: it turns `@contract` from a comment into a **checked link**,
so a renamed schema definition fails CI instead of silently parsing to empty.

---

## 11. Fixture homes (T-171)

Pin: [`WHERE_DOES_X_GO.md`](WHERE_DOES_X_GO.md).

1. **Fixtures live crate-local** in `tests/fixtures/` beside their primary consumer.
2. **Cross-crate contract data** lives in `packages/tbd-schema` (schema / golden / golden-missions / registry).
3. **`.ai/artifacts/` is pipeline OUTPUT only** — never a load-bearing input (`include_str!` / gate reads forbidden).
4. Byte-pinned goldens are excluded from editorconfig-checker (see `.editorconfig-checker.json`).

SPA R-api goldens: `apps/website/frontend/tests/fixtures/api/`. Gate oracles/manifests: `tools/tbd-tools/fixtures/t159/`.

## 12. Quick-reference cheat sheet

Cross-link this from [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md). Doc
**placement** (where markdown files live): §8.2. Homes: [`WHERE_DOES_X_GO.md`](WHERE_DOES_X_GO.md).

**Every exported symbol needs:**

| Language | Syntax | Cross-boundary tags |
|----------|--------|---------------------|
| Rust (API) | rustdoc on public items; `@route` / `@contract` where cross-boundary | handlers in `api/src/handlers/`; models = wire contract |
| Rust (SPA) | module docs; DTO comments cite schema where useful | `dto.rs` R-api goldens |
| Enfusion | `//!` banner; `/** */` header on Backend/Gamemode; `[Attribute(desc:)]` text; per-field DTO docs | `@contract` on DTOs; `@route` on REST calls; **`@authority` / `@rpc` / `@replicated`** on networked code |
| Go / TS | **retired** (T-145 / T-159) — historical examples in §3–§6 only | — |

**Contract change checklist:** schema bump → regenerate DTOs → Decisions-log entry → update every
`@contract`/`@route`/`@model` link → same commit (§1, §8.1).

---

*Defects against this standard are fixed on next edit of the affected file. Disputes resolve up the
authority ladder: running code wins, then [`CLAUDE.md`](../../CLAUDE.md), then this doc.*
