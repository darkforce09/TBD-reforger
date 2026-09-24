**Status:** live

# README template: data

**When to use:** a folder of schemas, fixtures, migrations, seeds or asset data that code reads
rather than runs: `contracts_v2/definitions/`, a fixture folder, `apps/website/api_v2/migrations/`,
`apps/website/api_v2/seeds/`, `assets_v2/terrains/`. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the data kind adds Format, and Producers and consumers.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. A folder of one
homogeneous collection lists it as one glob line in Contents.

````markdown
# <What the data is, in plain words: no path, no backticks>

<One to three sentences: what the data is for and what reads it.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/  <what it holds: a lowercase phrase, no closing period>
└── <file or glob>   <what it holds>
```

## How it works

<How the files are read and checked: the loader or gate that consumes them, the order they apply
in, and the invariants that hold across files. A folder with no child folders besides exempt ones
and at most three files, whatever its kind, may leave the section out (README.md not counted).>

## Format

- Encoding: <file type and encoding, and the naming convention>
- Schema: <the schema or grammar the files follow, linked in contracts_v2/, or the structure a
  loader expects>
- Adding a file: <where it goes, what it must hold, and the command that checks it>

## Producers and consumers

- Producers: <what writes the files (a command, a tool, a person), with its path>
- Consumers: <what reads them (loaders, gates, tests, the game), each with its path>

## Boundaries

- Depends on: <the schemas and registries the data must agree with>
- Used by: <everything outside the folder that reads it, found with git grep>
- Rules: <the invariants a change must keep: stability of names, ordering, generated files>

## Related documentation

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `contracts_v2/fixtures/missions/`. No document goes deeper into this corpus, so the
sample has no Related documentation. The sample sits in a fenced block, so no gate reads it as a
README; the folder's own README.md is written from the same code and may differ.

````markdown
# Mission fixtures

The mission contract's golden corpus: complete missions the contract must accept, and wrapped
documents it must reject, each for one stated reason.

## Contents

```text
contracts_v2/fixtures/missions/
├── invalid/  wrapped documents the gates must reject, each naming its gate and JSON pointer
└── valid/    complete missions that must pass the schema, the size ceiling and the kit-alias check
```

## How it works

`cargo xtask schema validate` reads both folders. Every file in `valid/` must pass
`contracts_v2/definitions/mission.schema.json`, stay under the schema's `x-tbd-missionFileMaxBytes`
ceiling (8 MiB, the
same value as `MISSION_FILE_MAX_BYTES` in the mod's mission loader), and use only kit aliases that
the mod's spawn registry defines. Every file in `invalid/` must be rejected by the gate its wrapper
names, and every finding must sit at or below the wrapper's JSON pointer, so a fixture rejected for
an unrelated reason fails the run. Commands and tests also load single missions from `valid/` by
name.

## Format

- Encoding: UTF-8 JSON, one document per file, named in lowercase hyphenated words after what the
  file holds (`valid/bridgehead-at-levie.json`, `invalid/net-range-any.json`).
- Schema: a file in `valid/` is a mission document following
  `contracts_v2/definitions/mission.schema.json`, with its `schemaVersion` (1.0 to 1.3 among the
  current files). A file in `invalid/` is a wrapper: `$comment` says why it exists; `mustFail`
  holds `gate` (`schema` or `registry`), `at` (the JSON pointer the rejection must sit at or below)
  and `because`; `document` holds the mission. The document is well-formed JSON that breaks one
  rule, and a `registry` fixture must pass the schema.
- Adding a file: put it in the folder that matches, check a mission alone with
  `cargo xtask schema validate-file <path>`, then run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people; no tool writes these files. The `schema-1_3-*` missions are written by hand
  ahead of the Mission Creator emitting their fields.
- Consumers:
  - `cargo xtask schema validate`, also the `schema-validate` CI task, in
    `tools_v2/xtask/src/verifications/schemas/checks/`: `mission_validation.rs` builds both folder
    paths itself and checks every file, and `kit_registry_references.rs` cross-checks the kit
    aliases;
  - `cargo xtask mod world-boot --mission <name>`, which resolves the name in `valid/` through
    `mission_fixtures_valid_dir` (`tools_v2/developer-tools/src/repository_layout.rs`, called from
    `tools_v2/xtask/src/commands/mod_ops/world_boot/execution.rs`), and
    `cargo xtask mod test-mission <name>`, which stages the mission it finds by file name under
    `contracts_v2/`; `cargo xtask mod dev-server` only names `valid/bridgehead-at-levie.json` in its
    usage text, as the offline `--artifact-file` for `cargo xtask mod playtest`;
  - tests that load one mission by name: the map engine's compiler flatten tests
    (`apps/website/map-engine/src/data/scenario/compiler/flatten/tests/`), the API's schema
    validator test (`apps/website/api_v2/src/missions/contract/tests/schema_validators.rs`), and the
    xtask schema and mission-test tests, which reach `valid/` through `mission_fixtures_valid_dir`.

## Boundaries

- Depends on: `contracts_v2/definitions/mission.schema.json`, and the spawn registry
  `apps/mod/tbd-framework/Data/registry.json`, whose `entries[].alias` values are the kit aliases a
  mission may name.
- Used by: the xtask schema gate, the xtask mod commands and the tests listed above.
- Rules: a valid mission stays valid and under the size ceiling; an invalid fixture breaks exactly
  one rule, at its declared pointer, and is never malformed JSON; commands and tests load missions
  by file name, so a rename updates every `git grep` hit in the same change.
````
