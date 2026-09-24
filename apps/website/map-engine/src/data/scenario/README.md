# Mission domain

The [mission](/documentation_v2/glossary.md#mission) as data, shared by the browser and the server:
the shapes of the saved payload and of the compiled document, the compiler between them, the
validation and wire-safety checks, the optional authored blocks, a plain-text line per
[slot](/documentation_v2/glossary.md#slot) of an [ORBAT](/documentation_v2/glossary.md#orbat) and
the mortar solver. The module name keeps the code spelling
[scenario](/documentation_v2/glossary.md#scenario); prose says mission.

## Contents

```text
apps/website/map-engine/src/data/scenario/
├── ast/          the parsed payload, the compiled document's rows, the ORBAT templates
├── ballistics/   mortar firing solutions from per-weapon charge tables
├── compiler/     the editor payload, the compiled mission document, the kit-alias table
├── extensions/   the optional authored blocks: their typing, validation and compiled form
├── mod.rs        the module tree; re-exports the domain aliases; `COMPILER_PACKAGE_VERSION`
├── slot_line/    the plain-text line the ORBAT manager shows for a slot
└── validation/   the Mission Creator's validation rules and the API's wire-safety scans
```

## How it works

A mission moves through this module from the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document to the bytes a game
server loads:

```text
Mission Creator document (crate::data::store)
  │  compile::compile_payload
  ▼
editor payload ──▶ validate::default_registry ──▶ the validation panel's findings, in the browser
  │
  │  POST /api/v1/missions/{id}/versions: the payload schema, the wire_safety scans and
  │  flatten::scan_editor_payload_types; any finding refuses the save
  ▼
stored version ──▶ orbat::parse_orbat_template when an event attaches the mission
  │
  │  POST /api/v1/missions/{id}/submit: the capacity scan, flatten::flatten_to_mod_document,
  │  flatten::unsupported_authored_data, then mission.schema.json
  ▼
artifact: the compiled bytes, their findings and COMPILER_PACKAGE_VERSION ──▶ the game server
```

The Mission Creator also runs `flatten` itself for its compiled export, and its inspector panels
validate each authored block with the `extensions` modules as the author edits it. The module keeps
no state beyond the kit-alias table it parses once from the embedded
`contracts_v2/rules/kit-aliases.json`. The [API](/documentation_v2/glossary.md#api) links the
crate with its default feature, `scenario`, which adds only `serde`, `serde_json` and `thiserror`,
so the server's build carries no graphics crate.

## Public surface

`mod.rs` publishes the aliases the API and the single-page app import; they are the domain's
boundary, so moving one is a change for its callers:

- `orbat` (`ast::factions`): the ORBAT templates, for the API's
  [event](/documentation_v2/glossary.md#event) attachment, reservation restore and
  [mission deployment](/documentation_v2/glossary.md#mission-deployment) slot bindings.
- `compile` (`compiler::payload`), `flatten` and `kit`: the payload, the compiled document and the
  alias table, for the Mission Creator, the mission library's upload and the API's save, compile
  and [artifact](/documentation_v2/glossary.md#artifact) code.
- `validate` (`validation::validator`) and `wire_safety`: the rule list for the Mission Creator's
  validation panel, and the scans the API runs on save and compile.
- `audio`, `weather`, `spawn_modules`, `tasks`, `win_conditions`, `radio_plan` and
  `tactical_graphics`: each authored block's constants and validation, for the Mission Creator's
  inspector panels.
- `slot_line`, for the Mission Creator's ORBAT manager, and `ballistics`, for the API's
  fire-mission routes.
- `COMPILER_PACKAGE_VERSION`: `website-map-engine` and the crate version, recorded with every
  artifact as its compiler.

## Boundaries

- Depends on: `serde`, `serde_json` and `thiserror`, the `scenario` feature's dependencies;
  `contracts_v2/rules/kit-aliases.json` at build time; no other module of the crate.
- Used by:
  - the API's [missions](/documentation_v2/glossary.md#missions) and
    [operations](/documentation_v2/glossary.md#operations) domains in
    `apps/website/api_v2/src/missions/` and `apps/website/api_v2/src/operations/`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`, the mission library in
    `apps/website/frontend/src/v2/pages/mission_hub/library/` and the DTOs of
    `apps/website/frontend/src/v2/core/api/dto/`;
  - inside the crate, `crate::data::store` and `crate::editing`.
- Rules:
  - nothing here imports the document store, the graphics engine, or the crate's `camera`,
    `diagnostics`, `doll`, `frame`, `io`, `overlay`, `spatial`, `streaming` and `world` modules
    (rule 4 of `cargo xtask verify engine-layers`); the only exceptions are two `store`-gated tests
    the gate pins by file and count;
  - no browser API, graphics device or Leptos state: callers pass every input; the payload keeps
    the document's `entityOrder`, and the compiled document keeps the authored faction, squad and
    slot order (`flatten_matches_locked_contract` in `compiler/flatten/tests/cases_1.rs`);
  - `contracts_v2/fixtures/missions/valid/compiler-shaped-two-faction.json` is the compiler's own
    output (`compiler_shaped_golden_is_a_fresh_emitter_output` in
    `compiler/flatten/tests/cases_3.rs`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — the compiled document.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the saved payload.
- [Mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — how a compiled mission becomes an artifact, a review and a deployment.
