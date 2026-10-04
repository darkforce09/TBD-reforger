**Status:** live

# Documentation standards

How this repository documents its code and writes its documents: the comment rules for Rust and
EnfScript, the cross-boundary tags and the checks that hold them, and the layout, naming and
lifecycle of the documentation tree. It details law 8 of [CLAUDE.md](/CLAUDE.md) with a real
example of each rule. README.md files follow the
[README standard](/documentation/standards/readme_standard.md), which this document links rather
than repeats.

## 1. Scope and authority

- The rules apply to every comment in `apps/`, `crates/` and `tools/` and to every document
  under `documentation/`; the audience is developers and AI agents.
- When sources disagree, the running code wins, then `CLAUDE.md`, then the rest of the
  [authority ladder](/documentation/README.md#authority-ladder) in the entry README.
- A comment or document the code contradicts is a defect, fixed by the next change that touches
  it; a stale doc comment is a bug, not a cosmetic issue.

## 2. Contracts behind the tags

A `@contract` tag (section 3) points at the single definition a type projects; these rules say
what that definition is and how its projections stay true to it.

- **One definition.** A shape that several programs or languages share (the mission document, the
  editor payload, the registry and loadout exports, fleet commands, machine credentials and the
  rest of `contracts/definitions/`) is defined once there, as a JSON Schema, and every
  projection follows it; a new field goes into the schema first. The REST API's other request and
  response bodies are defined by the Rust models in `apps/api/src/<domain>/models/`
  (law 9).
- **Projections.** `cargo xtask ci schema-codegen` generates Rust types from the schemas into the
  `generated/` folder of the `contract_schema_types` crate
  (`crates/contracts/contract_schema_types/src/generated/`), which nobody edits by hand; `cargo xtask ci verify-codegen-fresh`
  fails when they drift from the schemas. The web app's DTOs in
  `crates/frontend/foundation/frontend_api_dtos/src/` are hand-written and held to the API's answers by
  the golden responses in `contracts/fixtures/api_goldens/`
  (`crates/frontend/foundation/frontend_api_dtos/src/tests/r_api.rs`). EnfScript has no code generator:
  its DTOs are hand-written, carry `@contract`, and the golden missions in
  `contracts/fixtures/missions/` are validated against the schemas by
  `cargo xtask schema validate`.
- **Runtime validation.** `POST /api/v1/missions/:id/versions` validates the payload against
  `mission-editor-payload.schema.json` before it stores anything and answers 400 with the
  violations (`crates/api/api_missions/src/handlers/mission_versions.rs`).
- **Three version fields.** The canonical mission document (`mission.schema.json`, what the mod
  loads) carries `schemaVersion` as a string; the editor payload
  (`mission-editor-payload.schema.json`, what the version save accepts) carries `schemaVersion` as
  an integer; the export document (`GET /api/v1/missions/:id/export`) carries
  `exportFormatVersion`, an integer, and no `schemaVersion`. A projection types each field as its
  own document does.
- **Published data is immutable.** A mission version is written once, unique per mission and
  semver, and a database trigger refuses any update
  (`crates/api/api_database/migrations/0052_mission_version_immutability.sql`). A contract change is a
  schema change plus regeneration, never an edit of a stored payload.

Wire casing is fixed per document:

| Document | Casing |
|---|---|
| REST request and response bodies under `/api/v1` | snake_case, as the API's models serialize |
| List responses | `{ data, total, limit, offset }`; the audit log pages with `{ data, next_cursor }` |
| The export document, `mission.schema.json`, `mission-editor-payload.schema.json`, `loadout-export.schema.json` | camelCase |
| `registry-items.schema.json` | camelCase envelope, snake_case item fields |

A change to a cross-boundary contract lands in one commit: the schema change, the regenerated
types, every `@contract` and `@route` that moves with it, and a `decisions.md` entry in the
feature folder that owns the contract.

## 3. Tags

Two tags link code across the boundaries between the contracts, the website and the mod; three
more state where an Enfusion method runs. Each tag sits in a doc comment (`///`, `//!`, or a
`/** … */` header) so a reader can grep it from either end. A tag points from a projection or a
caller to its single source, the schema definition or the route, and never lists consumers,
which are many and change. A contract exchanged as a file rather than over a route, such as the
loadout export (`loadout-export.schema.json`), takes `@contract` alone.

### 3.1 Grammar

Each tag is required on the code its row names; sections 5 to 7 give its exact form with a real
example, and section 10 the checks.

| Tag | Written on | Checked by |
|---|---|---|
| `@route <METHOD> <path>` | every Axum handler; every EnfScript REST call site | `cargo xtask verify route-tags` (Rust side, both directions); `cargo xtask verify enfusion-comments` (EnfScript call sites, ECM-6) |
| `@contract <schema>#<pointer>[ (<sub-path>)][ partial]` | every type that projects a schema definition: Rust models and DTOs, hand-written EnfScript DTOs | `cargo xtask schema citations`: every citation resolves; `cargo xtask verify enfusion-comments`: present on every EnfScript `*Struct` (ECM-6); the API's `contract_parity_mod_wire` suite: every tagged EnfScript class matches the node it cites |
| `@authority server\|client\|owner` | every EnfScript method whose correctness depends on where it runs | `cargo xtask verify enfusion-comments` (ECM-5) |
| `@rpc <Reliable\|Unreliable> <Server\|Owner\|Broadcast>` | directly above every `[RplRpc]` | `cargo xtask verify enfusion-comments` (ECM-5) |
| `@replicated <prop>` | directly above every `[RplProp]` | `cargo xtask verify enfusion-comments` (ECM-5) |

## 4. Comments in all code

A comment says what the code does now and why: the invariant it keeps, the model behind a
calculation, the engine or hardware constraint it works around, the way it fails. It describes the
code as it stands, in the present tense.

- No history: no "formerly", "rewritten from", "fixed in", "legacy", no dates and no comparison
  with a retired implementation. Commit history owns history.
- No ticket identifiers and no delivery vocabulary (the wave, the slice or the run that produced
  the code): a reader of the code has no registry to look them up in.
- No restating the signature: a doc comment adds what the name and types do not say.
- A change that alters documented behaviour updates the comment in the same diff.

Two test suites hold parts of these rules, and run with their crate's tests:
`apps/api/src/tests/prose_rules.rs` refuses ticket identifiers, delivery vocabulary,
narrative about another implementation and retired paths in the API crate's sources, tests,
`.env.example`, seeds and migration comments; `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`
refuses ticket identifiers, retired names and deleted script names in every tracked file under
`tools/`. Elsewhere review holds them. The comment below states an engine constraint and the
invariant that follows from it
(`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/TBD_MissionSlotStruct.c:61-63`):

```c
	//! Sentinel for "y absent from JSON". JsonLoadContext leaves a missing key at the
	//! field initializer, and no real ASL height approaches -1e6 m, so the initializer
	//! doubles as the presence flag (standard JSON cannot carry NaN/Infinity).
```

## 5. Rust comments

These rules cover every Rust crate of the workspace, under `apps/`, `crates/` and `tools/`.

**Module header.** A non-trivial module opens with a `//!` summary line and the four-point
contract: `**Role:**` (its responsibility), `**Position:**` (its boundary layer, what feeds it and
who consumes it), `**Signals & state:**` (mutable state, reactive signals and thread ownership, or
none) and `**Invariants:**` (the guarantees a change must keep). From
`apps/frontend/src/app_routes.rs:1-11`:

```rust
//! The router's route table, in render form.
//!
//! **Role:** binds every path to the component that renders it, and names the fallback used when
//! none match.
//! **Position:** rendered by the frame — inside `<main>` for a chromed route, and directly for
//! the bare and chromeless ones. The chrome lives outside this component, so navigation swaps
//! only what is declared here.
//! **Signals & state:** none. Each route component owns its own.
//! **Invariants:** this list mirrors the route table in `router.rs`, which is the contract the
//! layout flags and the required tiers are read from; a path added here without a row there
//! renders with default layout and no tier requirement.
```

**Symbol docs.** Every public type, function, method and enum carries a `///` Markdown doc
comment, and names other items as intra-doc links (``[`crate::path::Type`]``) that rustdoc
resolves. No gate runs rustdoc, so review holds this rule. From
`crates/api/api_http_layer/src/observability/metrics_registry.rs:90-96`:

```rust
/// One registry per [`crate::core::http_router::router`] call.
///
/// Deliberately **not** a `static`: a process-global recorder makes every test that
/// asserts a count depend on which other tests ran first, which is precisely the
/// "green over something it never examined" shape this design exists to avoid. The
/// cost is that only code holding the `Arc` can record — see the module header.
pub struct Registry {
```

**Route tags.** Every Axum handler a route table registers carries `/// @route <METHOD> <path>`,
written at column 0 in the doc comment of a column-0 `pub fn` or `pub async fn`, with the full
`/api/v1` path; a path parameter is written `:name` or `{name}` with the router's name. The
route-tag check fails a tag no route table registers for that method and handler, a registered
route whose handler has no matching tag, and a tag with no handler under it. From
`crates/api/api_administration/src/handlers/audit_logs.rs:66-69`:

```rust
/// `GET /api/v1/admin/audit-logs` — newest-first, keyset pagination via `?before=`.
///
/// @route GET /api/v1/admin/audit-logs
pub async fn list_audit_logs(
```

**Contract tags.** A model or DTO that projects a schema definition carries
`@contract <schema>#<pointer>[ (<sub-path>)][ partial]`: the schema's file name in
`contracts/definitions/`, then an RFC 6901 JSON pointer, `#` or `#/` for the whole document.
Two optional parts follow, in this order and each after one space:

- `(<sub-path>)` walks nested properties below the pointer, dot-separated, with `name[]` stepping
  into an array's items: `mission.schema.json#/$defs/slot (loadout.cargo[])` cites the items of a
  slot's `loadout.cargo` array. A pointer that lands on a `oneOf` list cites every alternative.
- `partial` declares a projection that reads a subset of the cited object, so it need not carry
  every property the object requires; without it, the type carries every required property.

The grammar is closed: a tag of any other shape is an error. `cargo xtask schema citations`
resolves the schema and pointer of every tag, and the API's `contract_parity_mod_wire` suite reads
the whole tag on the mod scripts, parsed by
`apps/api/tests/enfscript_source_support/contract_tag.rs`: every field name and `//!<`
JSON key binding of a tagged class must be a property of the cited node, and a class without
`partial` must carry every required one. A module of such types carries the tag in its `//!`
header (`crates/api/api_missions/src/models/registry.rs:4`); a single type in its `///`
comment (`crates/api/api_administration/src/models/audit_stream.rs:19-21`):

```rust
/// @contract audit-log.schema.json#/definitions/AuditStreamReady
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStreamReady {
```

## 6. Enfusion comments

These rules cover every EnfScript file of the three addons under `apps/mod/`. Every line a change
adds to a `.c` file is ASCII: write `--`, `->` and `...` rather than typographic dashes, arrows and
ellipses. `git diff -U0 -- <file> | grep -P '^\+.*[^\x00-\x7F]'` prints nothing for a conforming
change.

**File header.** A top-level script or Workbench plugin opens with a `/** … */` block naming the
file and what it does. From `apps/mod/tbd-export/Scripts/Game/TBD/Export/TBD_RoadClassifier.c:1-12`:

```c
/**
 * TBD_RoadClassifier.c
 *
 * Deterministic road classification engine for tbd-export.
 * Categorizes road segments into:
 *   1. Highways & Major Arterials (highways.json)
 *   2. Secondary Paved Roads (roads_paved.json)
 *   3. Dirt & Gravel Roads (roads_dirt.json)
 *   4. Tracks & Tractor Trails (tracks.json)
 *   5. Footpaths & Hiking Trails (paths.json)
 *   6. Airfield Runways & Taxiways (runways.json)
 */
```

**Banners, member docs and DTO contracts.** Every class and every non-trivial method carries a
`//!` banner stating its purpose or contract, and every field or enum member whose meaning the
name does not carry gets a trailing `//!<` comment with its unit, default or JSON key. A
hand-written JSON DTO struct carries `//! @contract` and documents every field, because
`JsonLoadContext` maps JSON keys to field names and the coupling is invisible otherwise. From
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/TBD_MissionSlotStruct.c:37-44`:

```c
//! One container cargo row (loadout-export v2 {container,item,qty}).
//! @contract mission.schema.json#/$defs/slot (loadout.cargo[])
class TBD_SlotCargoStruct
{
	string container; //!< Wear container key: vest / pants / jacket / backpack.
	string item;      //!< Item ResourceName.
	int qty = 1;      //!< Units to insert (>= 1).
}
```

A method banner states where the call runs when the call crosses machines, as
`//! @authority owner` does on `TBD_RequestLobbyRoster`
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/SCR_PlayerController.c:20`).

**Editor attributes.** Every `[Attribute]` carries a description (`desc:` or the third positional
argument) with its unit and default, and every `[ComponentEditorProps]` a `description:`. From
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/TBD_SpectatorComponent.c:43`:

```c
	[Attribute("2000", desc: "Max metres a spectator may steer their streaming host from their own death position. Default 2000. 0 uses the default; never unlimited.")]
```

**REST call sites.** The class or method that calls the API carries `//! @route <METHOD> <path>`
naming the route it calls, so a search for the route string finds both the Rust handler and the
EnfScript caller. From
`apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/Delivery/TBD_TelemetryDelivery.c:63`,
on the method that posts the telemetry queue's head entry, one tag per route it may call:
``//! @route POST /api/v1/ingest/match-results``.

**Gate.** `cargo xtask verify enfusion-comments` checks the rules of this section and section 7
(rules ECM-1 to ECM-9, specified in the
[comment gate README](/tools/checks/mod_script_checks/src/enfusion_comments/README.md))
over the pinned mod Scripts roots, today `apps/mod/tbd-framework/Scripts` and
`apps/mod/tbd-emcp/Scripts`; `--path` narrows it to
any folder or file under `apps/mod/`. `cargo xtask ci verify-coding-standards` and the CI
language-gates job run it without `--path`.

## 7. Network authority

In a replicated game, which machine runs a method is part of its contract.

- `//! @authority server|client|owner` sits on every method whose correctness depends on where it
  runs.
- `//! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast>` sits directly above every `[RplRpc]`
  attribute and repeats its channel and receiver.
- `//! @replicated <prop>` sits directly above every `[RplProp]` field, naming who owns the value
  and the `onRplName` hook clients react in, when the attribute names one.
- `cargo xtask verify enfusion-comments` (ECM-5) requires the three tags: `@authority` on every
  method whose body calls `Rpc(` or asks where it runs, `@rpc` matching the attribute it sits
  above, and `@replicated` above every `[RplProp]`.
- A server gate, `if (TBD_Authority.IsClient())`, carries a
  `// Authority only -- <reason>` comment above it, as at
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/SCR_PlayerController.c:24`.

From `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/SCR_PlayerController.c:33-36` and
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:25-27`:

```c
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_LobbyRoster()

	//! @replicated m_Stage
	[RplProp(onRplName: "OnStageReplicated")]
	protected TBD_EGameStage m_Stage = TBD_EGameStage.LOADING; //!< current round stage; server-owned, LOADING until the load settles
```

## 8. Documentation tree

### 8.1 Layout

`documentation/` holds every document beside the code: feature docs, runbooks, standards,
design references, known bugs, ticket specs and plans, and the archive. The
[entry README](/documentation/README.md) lists what the root holds now; the layout it grows
into is:

```text
documentation/
├── README.md              entry: the map, the authority ladder, where to find things
├── architecture/          the workspace layout as it stands
├── glossary/              project terms and abbreviations, one file per letter range
├── product_roadmap.md     the operator-curated plan
├── apps/                  mirrors apps/: api/, frontend/ (pages/<area>/, apps/editor/ for the
│                          Mission Creator), fleet_host_agent/, ticketboard/
├── mod/                   mirrors apps/mod/: tbd-framework/, tbd-export/, tbd-emcp/
├── tools/<crate>/  crates/<tier>/<crate>/  contracts/  assets/
├── design_system/         tokens, typography, colour, symbology, token exports
├── runbooks/              every operator procedure
├── standards/             this document, the README standard, templates/, coding standards,
│                          placement, the commit checklist, ticket identifiers, crate boundaries
├── known_bugs/            the live bug registry
├── restructure/           the active workspace restructure program, archived when it closes
├── tickets/               specs/ and plans/, flat and frozen once their ticket closes
└── archive/<topic>/       frozen history
```

- **READMEs.** Every folder of the documentation tree and of the code trees carries a README.md
  built to the [README standard](/documentation/standards/readme_standard.md), which lists the
  exempt folders. A code README describes its folder at a high level and links the deeper
  documents here; a documentation README indexes its folder.
- **Mirror naming.** A feature's documents sit at the documentation root plus the path of its
  code without `src/`, keeping the code's folder spellings: the documents on
  `tools/developer_tools/src/` are in `documentation/tools/developer_tools/`, and those on
  `apps/api/src/` in `documentation/apps/api/`; the single-page app follows the same rule
  (`crates/frontend/pages/operations_pages/src/schedule/` is documented under
  `documentation/crates/frontend/pages/operations_pages/schedule/`, and
  `crates/frontend/workspaces/mission_creator_workspace/src/` under `documentation/crates/frontend/workspaces/mission_creator_workspace/`).
  One mirror keeps a shorter path until a stage of the
  [workspace restructure](/documentation/restructure/README.md) reshapes its code: until M1 a
  mirror of the mod leaves out `apps/` and `Scripts/Game/TBD/` and sits under
  `documentation/mod/`. The grain is chosen
  per case: one `pages/account/` folder covers login, the auth callback and settings, while
  administration has a folder per page.
- **Mirror moves.** A [relocation manifest](/documentation/restructure/manifests/README.md) that
  moves code also moves that code's documentation mirror, in a row of the same manifest, so the
  documents never fall behind the code they mirror.
- **Feature grouping.** Everything about one feature lives together: behaviour, interface design,
  the design target and its `visual_references/`, roadmap, research and evidence. All Mission
  Creator material sits under `documentation/crates/frontend/workspaces/mission_creator_workspace/`.
- **Feature docs.** Each feature doc is its own file beside its folder's README index, built from
  the [feature doc template](/documentation/standards/templates/feature_doc.md): a page's doc is
  `<page component>_page.md` (`personnel_roster_page.md`), the account pages share
  `account_pages.md`, the frame is `app_layout_and_navigation.md`, and a mod screen's doc is
  `<screen>_specification.md`.
- **Visual references.** A set is named `<subject>_<kind>`, kind `blueprint`, `mockup` or
  `render`, and holds `<set_name>.html` (the export), `<set_name>.png` (its screenshot; a render
  has only this) and `design_tokens.md` when the export carries tokens. It sits in the
  `visual_references/` folder of the feature it depicts; a mod screen's in-game captures sit in
  `reference_screenshots/` beside its sets. Design references are the only images.
- **Evidence.** Verification evidence sits in a `verification_evidence/` folder of its feature
  (`documentation/apps/api/verification_evidence/`); hyphenated evidence JSON names keep
  their spelling.

### 8.2 Placement

Documents live under `documentation/`, the single documentation root. The code trees (every
top-level folder but `documentation/` and the retired `docs`, read from git's listing so a new one
is judged at once) hold no Markdown besides README.md files, except below
a `tests`, `generated`, `Generated` or dot-folder, which `cargo xtask verify markdown-placement` enforces; the
repository root keeps its own README.md and `CLAUDE.md`. `cargo xtask ci verify-documentation`, a
`ci-local` step, and the `language-gates` job of `ci.yml` run it with readme-coverage and
link-check, so no application grows a documentation tree of its own.

### 8.3 Names, status lines, size and links

- **File names.** snake_case, except `README.md`, `t-<id>_plan.md` and the hyphenated evidence
  JSON. Sources waiting under `documentation/pending_merge/<writer>/` keep their original names
  until their writer merges and deletes them.
- **Status line.** Every Markdown document under `documentation/` starts with one status line
  and a blank line: `**Status:** live`, `**Status:** frozen record` or `**Status:** archived`, an
  archived document adding `— see [its replacement](…)` when one exists. No other header block
  (audience, authority, updated date) follows it. Code READMEs carry no status line.
- **Size.** A live document stays at or under 500 lines; a longer one splits by topic into a folder
  with a README.md index. Frozen and archived documents are exempt, and so are the two documents
  `cargo xtask ticket sync` targets (`SYNC_MANAGED_DOCUMENTS` in
  `tools/checks/documentation_checks/src/path_regions.rs`):
  `documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md`, whose next-work
  block it rewrites between markers, and
  `documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md`,
  whose tables its ticket-column writer parses but leaves unchanged, since their header has no
  `priority` column; that ticket column is kept by hand.
- **Links.** Links are repository-root (`[README standard](/documentation/standards/readme_standard.md)`,
  `[API](/apps/api/README.md)`), never `../` climbs. A frozen or archived document's
  link to code that no longer exists becomes a GitHub permalink with the full commit id,
  `https://github.com/darkforce09/TBD-reforger/blob/<commit>/<path>`.
- **Paths and commands.** A path written in backticks is repository-relative
  (`crates/api/api_missions/src/`), and must name a tracked file or folder, or one git
  ignores on purpose, such as `deploy/deploy.env`; a `cargo xtask` command written
  in a document must exist in the command tree. A README writes paths inside its own folder
  relative to it, as the README standard says.
- **Terms.** The editor is the Mission Creator; the document a mission maker authors is a mission;
  Enfusion's world plus game-mode configuration is the mission header; an event is a scheduled
  session record; operations is the domain around events. A document links a term's first use to
  its [glossary](/documentation/glossary/README.md) entry and quotes code identifiers as the code
  spells them.
- **Diagrams.** ASCII, in `text` blocks.
- **Hosts and paths.** No IP address of a host and no personal absolute path: the deploy host is
  whatever `TBD_SSH_HOST` names in `deploy/deploy.env`.

## 9. Document lifecycle

- **Open work.** A feature doc lists its open work under `## Open work`, each gap linked to its
  ticket's spec or to its `/.ai/tickets/T-<id>.toml`; a `deferred` ticket counts as open. A
  README never names a ticket.
- **Decisions.** A feature's decisions live in a `decisions.md` beside its feature docs, one entry
  per decision in the [decisions entry format](/documentation/standards/templates/decisions_entry.md):
  a `### YYYY-MM-DD — <decision>` heading, then Context, Decision, Consequences and Supersedes.
  There is no separate decision-record tree. A local choice is explained by a comment in the code
  it concerns; a cross-cutting rule goes into a document under `documentation/standards/`.
- **Specs and plans.** A ticket's spec is `documentation/tickets/specs/t<id>_<topic>.md` and its
  plan `documentation/tickets/plans/t-<id>_plan.md`; both folders are flat, and ticketboard
  reads them. A spec is live while its ticket is `idea`, `queued` or `ready`, and frozen once the
  ticket ships or is cancelled, and the knowledge that outlasts the ticket then moves into the
  feature doc. The ticket templates live in `.ai/tickets/`.
- **Frozen and archived.** A frozen record (a closed ticket's spec or plan) and an archived
  document (history under `documentation/archive/<topic>/`) are never reworded; only their
  links change. `link-check` judges only the links in both trees and `markdown-placement` exempts
  them from the size limit; their README indexes stay live and list the files as they land.
- **Known bugs.** `documentation/known_bugs/` is the live registry, one file per bug in the
  [known bug format](/documentation/standards/templates/known_bug.md); a resolved bug stays
  with its status set to resolved.
- **Templates.** Every document type has a template in
  [standards/templates/](/documentation/standards/templates/README.md): the README kinds, the
  feature doc, the runbook, the decisions entry, the known bug and the glossary entry.

## 10. Gates and the same-commit rule

Documentation ships in the same commit as the code it describes, whoever writes that code: a
change updates the comments of the code it alters, the README of every folder whose contents,
surface, commands or boundaries it changes, and the feature docs whose behaviour it changes.

Two checks hold the cross-boundary tags of section 3. `cargo xtask verify route-tags` compares
every Rust `@route` tag with the routes the API registers, in both directions, and
`cargo xtask schema citations` resolves every `@contract` citation against
`contracts/definitions/`. On the mod scripts, the API test binary `contract_parity_mod_wire`
(run by `cargo xtask db test-it`) also holds each tag's sub-path and `partial` marker against the
fields of the class it sits on. `cargo xtask ci verify-coding-standards` and
`cargo xtask ci verify-citations` run them, and `cargo xtask ci ci-local` runs both. The citation
check reads `.c`, `.go`, `.js`, `.mjs`, `.rs`, `.ts` and `.tsx` files under `apps/` and
`tools/`, never Markdown, and prints that scope on every run; when the printed scope and this
section disagree, the printed scope is right. A third check, `cargo xtask verify enfusion-comments`,
holds the EnfScript tags and the Enfusion comment rules of sections 6 and 7 over the pinned mod
Scripts roots, and `cargo xtask ci verify-coding-standards` runs it too. Review holds the other
comment rules, apart from the prose tests section 4 names.

Three gates check the documents, specified in the
[documentation gates README](/tools/checks/documentation_checks/src/README.md):
`cargo xtask verify readme-coverage` (every folder in the README span has a README.md whose
Contents block matches the folder), `cargo xtask verify markdown-placement` (the code trees hold no
Markdown besides README.md, and live documents stay within 500 lines) and
`cargo xtask verify link-check` (links, anchors, backticked paths and cited commands resolve).

Before committing a documentation change, run the gates over the folders it touches:

```bash
cargo xtask verify readme-coverage --path <folder>
cargo xtask verify link-check --path <folder>
cargo xtask verify markdown-placement
```

### 10.1 Prose citations

The citation check reads code, never documents, so a citation in prose is held by convention.

- Cite a symbol by name together with its file path: `TBD_SpawnManager.ClaimSlot` survives an edit
  above it, and a line number does not. A `:line` suffix may follow the path as a pointer into the
  file, as this document's examples do, but never replaces the name.
- Cite a file path without a line number when no symbol fits.
- A `@contract` written in prose is an illustration, not a checked link; a document that needs a
  checked one points at the code that carries it.
- A prose citation that must be machine-checked belongs in code as a tag, or in an index gate of
  its own, as `cargo run -q -p developer_tools --bin enf -- citations` checks every `@idx`
  citation under `documentation/` against the Enfusion symbol index.

## Related documentation

- [README standard](/documentation/standards/readme_standard.md) — the README core, kinds,
  Contents grammar and writing rules.
- [Templates](/documentation/standards/templates/README.md) — the skeleton and worked sample of
  every README kind and document type.
- [Glossary](/documentation/glossary/README.md) — the project's terms.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the code rules that
  sit beside these comment rules.
- [Where does X go](/documentation/standards/where_does_x_go.md) — where code, fixtures and
  data live.
- [Commit checklist](/documentation/standards/commit_checklist.md) — what a commit carries.
