# Mission compiler

The two compile steps of a [mission](/documentation_v2/glossary.md#mission): the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document into the editor payload
a version saves, and a saved payload into the document the [mod](/documentation_v2/glossary.md#mod)
loads, with characters and vehicles resolved through the kit-alias table.

## Contents

```text
apps/website/map-engine/src/data/scenario/compiler/
├── flatten/  a saved payload into the compiled mission document, with findings and substitutions
├── kit/      the kit and vehicle alias table embedded from `contracts_v2/rules/kit-aliases.json`
├── mod.rs    the module tree
└── payload/  the editor payload from the document, the export envelope, the version-save body
```

## How it works

```text
Mission Creator document (crate::data::store)
   │  compile::compile_payload                                  payload/
   ▼
editor payload ─────▶ POST /api/v1/missions/{id}/versions, body from compile::version_body
   │  flatten::flatten_to_mod_document, kits from kit::KitAliases     flatten/, kit/
   ▼
ModMissionDocument ─▶ the artifact bytes the game server loads, plus findings and substitutions
```

The Mission Creator runs the first step on every save and both steps for its compiled export, so
the file it offers comes from the same code as the server's. The
[API](/documentation_v2/glossary.md#api) runs the second step when a mission is submitted, and the
bytes become the mission's [artifact](/documentation_v2/glossary.md#artifact). The Export path
adds the derived [ORBAT](/documentation_v2/glossary.md#orbat) to the payload and wraps it in
`compile::compile_export`'s envelope; a saved payload carries no ORBAT, which the API derives
itself.

## Public surface

The three modules are reached through the aliases of `data::scenario`:

- `compile` (`payload/`): `compile_payload`, `compile_export`, `version_body`,
  `version_body_to_writer` and `terrain_bounds`, for the Mission Creator, the mission library's
  upload and the document store.
- `flatten` (`flatten/`): `flatten_to_mod_document` and the `flatten_mod_document_json*` entry
  points, `MissionMeta`, `ModMissionDocument` and its rows, `CompileError`,
  `apply_authored_environment`, `scan_editor_payload_types`, `unsupported_authored_data` and the
  `FLOW_DEFAULT_*` values, for the API's compile, save and artifact code and the Mission Creator.
- `kit` (`kit/`): `KitAliases` and `load_kit_aliases`, which the API's contract layer re-exports.

## Boundaries

- Depends on: `crate::data::scenario::ast` (the payload and document shapes, the ORBAT
  derivation), `crate::data::scenario::extensions` (the authored blocks),
  `crate::data::scenario::validate` (`Finding`) and `crate::data::scenario::wire_safety`;
  `contracts_v2/rules/kit-aliases.json`; `serde`, `serde_json` and `thiserror`.
- Used by: the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` and the mission
  library in `apps/website/frontend/src/v2/pages/mission_hub/library/`; the
  [missions](/documentation_v2/glossary.md#missions) domain in
  `apps/website/api_v2/src/missions/`; inside the crate, `crate::data::store::operations`,
  `crate::editing::persist` and `crate::data::scenario::validate` (`terrain_bounds`,
  `compile_payload`).
- Rules: `contracts_v2/fixtures/missions/valid/compiler-shaped-two-faction.json` is this
  compiler's output byte for byte (`compiler_shaped_golden_is_a_fresh_emitter_output` in
  `flatten/tests/cases_3.rs`), and `cargo xtask schema validate` checks it against
  `contracts_v2/definitions/mission.schema.json`; a saved payload has no `orbat` key
  (`save_payload_omits_orbat_and_has_editor_shape` in `payload/tests/cases_1.rs`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — the compiled document.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the saved payload.
- [Mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — what the API does with a compiled document.
