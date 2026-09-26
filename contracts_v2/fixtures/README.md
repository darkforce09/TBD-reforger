# Contract fixtures

The golden test data of every contract boundary: samples each schema must accept, and
[mission](/documentation_v2/glossary/g_to_m.md#mission) documents it must reject. A fixture pins
agreement between components that no single unit test can state: one committed file is read by
the schema gate, by the tests of the crates on each side of the boundary, and by the commands that
stage it into a game server.

## Contents

```text
contracts_v2/fixtures/
├── bridge_samples/    voice bridge messages across a voice session's lifecycle
├── enfusion_samples/  one sample per mission-schema definition the mod's JSON classes read
├── map/               terrain and world-object samples, chunk and density binaries included
├── missions/          complete missions that must pass, and wrapped missions that must fail
└── registry/          item, compatibility, alias, loadout, faction and editor-payload samples
```

## How it works

`cargo xtask schema validate` reads every folder here: it validates each sample against its schema
in `contracts_v2/definitions/`, requires each invalid mission to fail its named gate at its named
pointer, and cross-checks kit aliases and registry references.
`cargo xtask schema map-object-golden` adds the world export's semantic gates over `map/`, including
the byte-level checks of its binary twins. Both run in the `schema-validate` CI task, and the
`schema.yml` workflow runs the first on every change under `contracts_v2/`. Beyond the gates, the
[API](/documentation_v2/glossary/a_to_f.md#api)'s and the map engine's tests load single fixtures by path,
and the xtask [mod](/documentation_v2/glossary/g_to_m.md#mod) commands stage missions from
`missions/valid/` into a game server.

The negative fixtures matter as much as the positive ones. Each invalid mission isolates one
defect at one pointer, so a gate that grows permissive fails here, naming the rule that broke,
instead of letting malformed missions through until one reaches a live
[event](/documentation_v2/glossary/a_to_f.md#event).

## Format

- Encoding: UTF-8 JSON, one document per file, named by each folder's convention; `map/` also holds
  two small binaries, kept as plain git blobs so every clone and worktree reads real bytes.
- Schema: each file follows a schema in `contracts_v2/definitions/`, named in its folder's README;
  `missions/invalid/` wraps its missions in a `mustFail` envelope.
- Adding a file: put it in the folder of the boundary it exercises, name it where its gate reads
  it (most gates name their files one by one), and run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people, apart from two files that tools regenerate: the map engine's compiled
  two-faction mission in `missions/valid/` and the density tile in `map/density/`.
- Consumers:
  - the xtask schema gates in `tools_v2/xtask/src/verifications/schemas/checks/` and the
    map-object golden gates in `tools_v2/developer-tools/src/map_verification/object_goldens/`,
    which reach these folders through `tools_v2/developer-tools/src/repository_layout.rs`;
  - the xtask mod commands `world-boot`, `test-mission` and `dev-server`;
  - tests in `apps/website/api_v2/`, `apps/website/map-engine/` and `tools_v2/`, named in each
    folder's README.

## Boundaries

- Depends on: the schemas in `contracts_v2/definitions/`, the spawn registry
  `apps/mod/tbd-framework/Data/registry.json`, and the binary formats of
  `apps/website/map-engine/src/io/`.
- Used by: the xtask schema gates, the developer tools' map verification, the xtask mod commands
  and the crate tests above.
- Rules: a schema change keeps every fixture it validates passing, in the same change
  (`cargo xtask schema validate`); a binary fixture is the exact encoding of its JSON twin
  (`cargo xtask schema map-object-golden`); a golden that stops validating means the schema changed
  or a validator regressed, and is never edited just to pass; live
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) exports stay in `contracts_v2/catalogs/`,
  never here.
