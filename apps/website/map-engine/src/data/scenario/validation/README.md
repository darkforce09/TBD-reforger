# Mission payload checks

The two kinds of check on a [mission](/documentation_v2/glossary.md#mission) editor payload: the
validation rules the [Mission Creator](/documentation_v2/glossary.md#mission-creator) evaluates
while a mission maker works, and the wire-safety scans the
[API](/documentation_v2/glossary.md#api) runs before it stores or compiles a version.

## Contents

```text
apps/website/map-engine/src/data/scenario/validation/
├── mod.rs        the module tree
├── validator/    the ordered rule list and its findings, exposed as `validate`
└── wire_safety/  the control-character and cargo capacity scans, exposed as `wire_safety`
```

## How it works

The Mission Creator's validation panel compiles the live document into a payload
(`crate::data::scenario::compile::compile_payload`), runs `validate::default_registry()` on it with
the item [registry](/documentation_v2/glossary.md#registry)'s asset ids as context, and lists the
findings beside the compile's own, which use the same `Finding` type. The API runs no rule: it runs
`wire_safety::scan_editor_payload` and `wire_safety::scan_cargo_capacity` after the payload schema
on every save (`POST /api/v1/missions/{id}/versions`), and the capacity scan again before every
compile. The `CARGO-OVER-CAPACITY` rule calls the same capacity scan, so a capacity fault reads the
same wherever it is reported.

## Public surface

- `validate` (`validator/`): `default_registry`, `EvalContext`, and the `Finding`, `Severity` and
  `Primitive` types, which the Mission Creator's validation panel and compiled export use and the
  API re-exports for the compile's findings.
- `wire_safety` (`wire_safety/`): `scan_editor_payload`, `scan_cargo_capacity`, `CargoPhys`,
  `CargoPhysCatalog` and `MAX_REPORTED`, used by the missions domain of the API;
  `is_wire_unsafe`, used by `crate::data::scenario::flatten`.

## Boundaries

- Depends on: `crate::data::scenario::compile` (`terrain_bounds`); `serde_json`.
- Used by: the Mission Creator's validation panel
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/`) and compiled export;
  the missions domain in `apps/website/api_v2/src/missions/` (`contract/`, `handlers/`,
  `services/`); inside the crate, `crate::data::scenario::flatten` and `crate::editing::commands`.
- Rules: every rule fires on its own trip fixture
  (`engine_self_check_passes_for_the_seed_registry` in `validator/tests/cases_1.rs`); the
  wire-safety byte rule is exactly the schema's `wireSafeString`
  (`byte_scan_equals_char_scan_over_the_schema_pattern` in `wire_safety/tests/cases_1.rs`); the
  capacity rule and the capacity scan agree
  (`cargo_over_capacity_agrees_with_the_standalone_scanner` in `validator/tests/cases_2.rs`).
