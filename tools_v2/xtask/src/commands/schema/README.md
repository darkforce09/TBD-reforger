# Schema command group

The `cargo xtask schema` group: contract code generation, the contract and map asset gates, and
two tools for [mission](/documentation_v2/glossary.md#mission) files. Developers run single gates
while they change a schema or its data; CI and the [wave](/documentation_v2/glossary.md#wave)
gates run the `schema-validate` set.

## Contents

```text
tools_v2/xtask/src/commands/schema/
├── cli.rs                 the `SchemaCmd` clap enum: sixteen subcommands and their flags
├── dispatch.rs            routes each subcommand to the codegen, the gate or the tool that runs it
├── mission_flattening.rs  `flatten-orbat-slots`: builds a mission's `slots[]` from its ORBAT roles
├── mod.rs                 the module tree
└── tests/                 unit tests for the flattening's preserve and refuse rules
```

## How it works

`tools_v2/xtask/src/cli/mod.rs` mounts `SchemaCmd` as the `schema` group and `dispatch::run`
calls one function per subcommand, returning its exit code. This folder owns only the flattening
tool; every other subcommand runs code elsewhere:

```text
codegen                    ─▶ tools_v2/xtask/src/commands/generate/schema_types.rs
list-gates                 ─▶ tools_v2/xtask/src/commands/ci/ (the schema-validate row)
validate, validate-file,   ─▶ tools_v2/xtask/src/verifications/schemas/
citations, map-glyphs,
map-object-enums, type-inventory
map-object-golden, height-labels, terrain-alignment, locations,
town-labels, road-names, terrain-manifest ─▶ tools_v2/xtask/src/verifications/map_assets/
                                              (developer-tools map_verification)
flatten-orbat-slots        ─▶ mission_flattening.rs
```

## Commands

Run each as `cargo xtask schema <subcommand>` from the repository root. An error that escapes a
subcommand prints `xtask: <cause>` and exits 1; a clap usage error exits 2.

### codegen

- Synopsis: `schema codegen`
- Does: generates Rust serde types with `typify` from the schemas in `contracts_v2/definitions/`
  into the `generated/` modules of the owning domains under `apps/website/api_v2/src/`; the
  loadout projection stays hand-written.
- Exit codes: 0 generated.
- Example: `cargo xtask schema codegen`

### list-gates

- Synopsis: `schema list-gates`
- Does: prints the `schema-validate` gate set, one name per line, read from the `ci` task table:
  `validate`, `map-object-golden`, `map-glyphs`, `height-labels`, `map-object-enums`,
  `type-inventory`. The platform wave gate compares its own list against it.
- Exit codes: 0.
- Example: `cargo xtask schema list-gates`

### validate and validate-file

- Synopsis: `schema validate`; `schema validate-file <TARGET>`, where `-` reads stdin.
- Does: `validate` runs the full contract suite over every fixture, sample, catalog and terrain
  document; `validate-file` checks one mission file against `mission.schema.json`, the 8 MiB
  ceiling and, for schemaVersion 1.1, the [slot](/documentation_v2/glossary.md#slot) count and
  unique slot ids, and prints `ok`.
- Exit codes: 0 valid; 1 at least one document failed.
- Example: `cargo xtask schema validate-file contracts_v2/fixtures/missions/valid/empty-warning-fields.json`

### citations

- Synopsis: `schema citations`
- Does: resolves every `@contract <file>.schema.json#<pointer>` tag in the code under `apps/` and
  `tools_v2/` against `contracts_v2/definitions/`, and prints the scanned scope and a count per
  file extension.
- Exit codes: 0 every citation resolves; 1 a dangling citation, a missing scan root or no
  citation found.
- Example: `cargo xtask schema citations`

### map-object-enums, type-inventory and map-glyphs

- Synopsis: `schema map-object-enums`; `schema type-inventory`; `schema map-glyphs`
- Does: hold the map-object data inside `map-object-enums.schema.json`: the kinds and classes of
  the golden prefabs, the classify rules, the regions and the glyph keys; the type inventories'
  invariants and the instance-kind list; glyph coverage, SVGs and the built atlas.
- Exit codes: 0 OK; 1 FAIL.
- Example: `cargo xtask schema type-inventory`

### Map asset gates

- Synopsis: `schema map-object-golden`; `schema height-labels [--terrain <T>]`;
  `schema terrain-alignment [--terrain <T>] [--strict]`; `schema locations [--terrain <T>]`;
  `schema town-labels [--terrain <T>] [--zoom <Z>]`; `schema road-names [--terrain <T>] [--zoom <Z>]`;
  `schema terrain-manifest [--terrain <T>]`. `--terrain` defaults to `everon`; `--zoom` defaults
  to `-2` for town labels and `0` for road names.
- Does: the map-object semantic goldens, the height-label, location, town-label and road-name
  gates, the elevation-to-anchor alignment, and the terrain manifest against its schema and the
  terrains contract, all run by the map verification code of `developer-tools`.
- Exit codes: 0 pass; 1 fail; `terrain-manifest` exits 2 for a terrain other than `everon` or
  `arland`.
- Example: `cargo xtask schema terrain-manifest --terrain everon`

### flatten-orbat-slots

- Synopsis: `schema flatten-orbat-slots <PATH> [--in-place]`
- Does: expands every [ORBAT](/documentation_v2/glossary.md#orbat) role of the mission at `PATH`
  into `count` slots with the id `<faction>:<callsign>:<slot>:<n>`, placed in rings of eight
  around the faction's spawn zone,
  keeping each slot's `uid`, `loadout` and `y` from the role or from the prior slot with that id.
  It sets `schemaVersion` to `1.1` only when the file has none. The result goes to stdout, or
  back into the file with `--in-place`. It refuses to replace a non-empty `slots[]` with an empty
  one, or to drop any `loadout` or `uid`.
- Exit codes: 0 written; 1 unreadable JSON or a refused write.
- Example: `cargo xtask schema flatten-orbat-slots contracts_v2/fixtures/missions/valid/empty-warning-fields.json`

## Boundaries

- Depends on: `tools_v2/xtask/src/verifications/schemas/` and
  `tools_v2/xtask/src/verifications/map_assets/` for the gates;
  `tools_v2/xtask/src/commands/generate/schema_types.rs` for `codegen`;
  `tools_v2/xtask/src/commands/ci/task_runner/` for `list-gates`;
  `ticket_engine::sync::refuse_empty_write` and `serde_json` for the flattening.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, which mounts the group;
  - the `schema-validate`, `schema-codegen`, `verify-citations` and `verify-terrain` rows of
    `tools_v2/xtask/src/commands/ci/task_definitions.rs`, and through them `ci-local-schema` and
    `ci-local`;
  - the platform wave gate's schema step
    (`tools_v2/xtask/src/commands/platform/wave_execution/schema.rs`), which runs the listed gates
    as `cargo xtask schema <gate>` subprocesses;
  - `.github/workflows/schema.yml` (`schema validate`) and the `schema` job of
    `.github/workflows/ci.yml` (through `ci ci-local-schema`);
  - people, for `validate-file` and `flatten-orbat-slots`.
- Rules:
  - The flattening refuses a lossy write on both the stdout and the `--in-place` path
    (`flatten_in_place_refuses_lossy_loadout_drop` and `flatten_stdout_refuses_lossy_loadout_drop`
    in `tests/mission_flattening.rs`), and never overwrites a `schemaVersion` the file already
    has (`flatten_in_place_preserves_schema_version_1_0`).
  - The `schema-validate` gate set is defined once, in the `ci` task table, and `list-gates`,
    the wave gate and `cargo xtask verify ci-schema-parity` read it from there.

## Related documentation

- [Contract definitions](/contracts_v2/definitions/README.md) — the schemas these commands
  generate from and validate against.
- [Contract schema gates](/tools_v2/xtask/src/verifications/schemas/README.md) — what each
  contract gate checks.
