# Mission compiler

The `mission_compiler` crate: the second compile step of a
[mission](/documentation/glossary/g_to_m.md#mission), a saved payload into the document the
[mod](/documentation/glossary/g_to_m.md#mod) loads, with characters and vehicles resolved through
the kit-alias table, the findings of everything the compile drops or reshapes, and the compiler
identity every [artifact](/documentation/glossary/a_to_f.md#artifact) records. The first step, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document into the editor
payload a version saves, is the `mission_payload` crate.

## Contents

```text
crates/mission/mission_compiler/
├── Cargo.toml  the package: `mission_model`, `mission_payload`, `mission_validation`, `mission_wire_safety`, `orbat_slot_ids`, `serde`, `serde_json`, `thiserror`; layout tier 5
└── src/        the game-document compiler, its editor-input structs, its error and the compiler identity
```

## How it works

```text
Mission Creator document (mission_document::MissionDocCore)
   │  mission_payload::compile_payload
   ▼
editor payload ─────▶ POST /api/v1/missions/{id}/versions, body from mission_payload::version_body
   │  flatten_to_mod_document, kits from mission_payload::kit_aliases
   ▼
ModMissionDocument ─▶ the artifact bytes the game server loads, plus findings and substitutions
```

The Mission Creator runs the first step on every save and both steps for its compiled export, so
the file it offers comes from the same code as the server's. The
[API](/documentation/glossary/a_to_f.md#api) runs the second step when a mission is submitted, and
the bytes become the mission's artifact, stored with `COMPILER_PACKAGE_VERSION` and hashed with it
into the artifact digest. A compile that meets authored data the document cannot carry drops it
and reports a `mission_validation::Finding`; it fails only on a payload that does not parse or
places no slot.

The [source README](/crates/mission/mission_compiler/src/README.md) lists the modules, and the
[game-document README](/crates/mission/mission_compiler/src/game_document/README.md) the stages.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_compiler    # the locked contract, the goldens, every stage and finding
cargo clippy -p mission_compiler --all-targets -- -D warnings
```

One test is an `#[ignore]`d writer run by hand: `regen_compiler_shaped_fixture` regenerates
`contracts/fixtures/missions/valid/compiler-shaped-two-faction.json`.

## Configuration

None: no features and no environment variables. The kit alias table is `mission_payload`'s,
compiled in.

## Public surface

All also in `prelude`, the rule ids and flow defaults excepted:

- `flatten_to_mod_document`, `flatten_mod_document_json`, `flatten_mod_document_json_full`,
  `flatten_mod_document_json_with_diagnostics`, `flatten_mod_document_json_with_substitutions`,
  `CompiledOutput`.
- `MissionMeta` (its `id` a `mission_model::ids::MissionId`), `ModMissionDocument` (its rows are
  `mission_model::compiled`'s), `mission_terrain_key`, `apply_authored_environment`.
- `KitSubstitution` (`AssetId`, `SlotId`, `SlotUid`) and `KitSubstitutionReport`.
- `scan_editor_payload_types`, `unsupported_authored_data`.
- `COMPILE_DIAGNOSTIC_RULE_IDS` and the nine `DIAG_*` rule ids; the four `FLOW_DEFAULT_*` values.
- `COMPILER_PACKAGE_VERSION`, a stored literal (`src/compiler_identity.rs`).
- `Error` (`NoSlots`, `Parse`) and `Result`.

## Boundaries

- Depends on: `mission_model` (the compiled rows, the authored blocks, the ids), `mission_payload`
  (`terrain_bounds`, the kit aliases), `mission_validation` (`Finding`, `RuleId`, `SubjectId`,
  `AssetId`), `mission_wire_safety` (`is_wire_unsafe`), `orbat_slot_ids` (`SlotId`, `SlotUid`);
  `serde`, `serde_json` (`preserve_order`) and `thiserror`.
- Used by:
  - the API's compile, save and artifact code in `crates/api/api_missions/src/`, which stores
    `COMPILER_PACKAGE_VERSION` with every artifact;
  - the Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/` and the mission DTOs of
    `crates/frontend/foundation/frontend_api_dtos/src/`;
  - `mission_document`'s payload round-trip tests.
- Rules:
  - mission tier 5, one above `mission_validation` (`cargo xtask verify crate-tiers`);
  - `contracts/fixtures/missions/valid/compiler-shaped-two-faction.json` is this compiler's output
    byte for byte (`compiler_shaped_golden_is_a_fresh_emitter_output` in
    `src/game_document/tests/cases_3.rs`), and `cargo xtask schema validate` checks it against
    `contracts/definitions/mission.schema.json`;
  - `COMPILER_PACKAGE_VERSION` never changes: it is stored and hashed with every artifact.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the category and its dependency rule.
- [Mission schema](/contracts/definitions/mission.schema.json) — the compiled document.
- [Mission editor payload schema](/contracts/definitions/mission-editor-payload.schema.json) —
  the saved payload.
- [Mission artifacts](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
  — what the API does with a compiled document.
- [Mission Creator feature inventory: data persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md) — when the editor compiles and what Save Version sends.
