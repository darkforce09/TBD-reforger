# Mission validation

The `mission_validation` crate: the ordered rule list the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) evaluates on a
[mission](/documentation/glossary/g_to_m.md#mission) editor payload while a mission maker works, the
findings it answers, the facts a context-dependent rule reads, and the self-check that proves every
rule can fire. The wire-safety scans the [API](/documentation/glossary/a_to_f.md#api) runs before it
stores or compiles a version live in the `mission_wire_safety` crate, which the capacity rule calls.

## Contents

```text
crates/mission/mission_validation/
├── Cargo.toml  the package: `mission_payload`, `mission_wire_safety`, `newtype_ids`, `serde_json`, `thiserror`; layout tier 4
└── src/        the rules, the registry and its self-check, the context, the findings and their ids
```

## How it works

`default_registry()` builds the sixteen rules in a fixed order; `Registry::evaluate_with_context`
runs each rule whose `applies` gate holds over the payload and the caller's `EvalContext` and
returns every finding, with no early exit; `validate_editor_payload` is the same over the default
context. `Registry::self_check` runs each rule on its own trip fixture and fails, as
`Error::SelfCheckFailed`, when one stays silent. The Mission Creator's validation panel compiles
the live document into a payload (`mission_payload::compile_payload`), evaluates it with the item
[registry](/documentation/glossary/n_to_z.md#registry)'s asset ids as context, and lists the
findings beside the compile's own, which `mission_compiler` reports as the same `Finding` type.

The [source README](/crates/mission/mission_validation/src/README.md) lists every rule.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_validation    # every rule, the context gates and the self-check
cargo clippy -p mission_validation --all-targets -- -D warnings
```

## Configuration

None: no features and no environment variables.

## Public surface

All also in `prelude`:

- `default_registry`, `validate_editor_payload`, `Registry`, `Rule`, `SelfCheckFailure`.
- `EvalContext` (`known_asset_ids`, `cargo_phys`, `loadout_policy`) and `LoadoutPolicy`.
- `Finding`, `Severity`, `Primitive`.
- `RuleId`, `SubjectId`, `AssetId`: serde-transparent ids, so a finding written to JSON keeps its
  bytes.
- `Error` (`SelfCheckFailed`) and `Result`, the outcome of `Registry::self_check`.

## Boundaries

- Depends on: `mission_payload` (`terrain_bounds`), `mission_wire_safety` (`CargoPhysCatalog`,
  `scan_cargo_capacity`), `newtype_ids`, `serde_json` (`preserve_order`) and `thiserror`.
- Used by:
  - `mission_compiler`, whose compile findings are `Finding` values and whose kit substitutions
    name an `AssetId`;
  - `mission_editing_commands::document_text`, which summarises compile findings for the
    compiled export;
  - the Mission Creator's validation panel
    (`apps/frontend/src/workspaces/editor/ui/inspector/validation_panel/`) and compiled export;
  - the API, which re-exports `Finding` and `Severity` for its compile and artifact code and runs
    no rule.
- Rules: mission tier 4, one above `mission_payload` (`cargo xtask verify crate-tiers`); every rule
  fires on its own trip fixture (`engine_self_check_passes_for_the_seed_registry` in
  `src/tests/cases_1.rs`); the capacity rule and the capacity scan agree
  (`cargo_over_capacity_agrees_with_the_standalone_scanner` in `src/tests/cases_2.rs`).

## Related documentation

- [Mission crates](/crates/mission/README.md) — the category and its dependency rule.
- [Mission editor payload schema](/contracts/definitions/mission-editor-payload.schema.json) —
  the payload the rules read.
