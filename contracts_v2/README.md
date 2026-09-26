# Contracts

Every data shape that crosses a network, process or language boundary on the platform: between the
[API](/documentation_v2/glossary/a_to_f.md#api), the single-page app, the map engine, the game
[mod](/documentation_v2/glossary/g_to_m.md#mod), the developer tools and the external voice bridge. The
tree holds data only (JSON Schemas, lookup tables, exported catalogues and golden samples) and no
code; the code that follows it lives in `apps/` and `tools_v2/`.

## Contents

```text
contracts_v2/
├── catalogs/     the live item registry and compatibility graph exported from the Workbench
├── definitions/  the authoritative JSON Schema of every cross-boundary payload and file
├── fixtures/     golden samples each schema must accept, and missions the gates must reject
└── rules/        prefab classification and kit-alias tables applied while producing data
```

## How it works

The four folders hold four kinds of data, and each is changed differently:

| Folder | What it holds | Changing it means |
|---|---|---|
| `definitions/` | the shape of a wire message or file; nothing puts a property on the wire that its schema does not define | a contract change: regenerated types, updated fixtures and citing code in the same change |
| `rules/` | deterministic tables consulted while producing data: prefab classification, kit and vehicle aliases | a data change that takes effect only when the output it shapes (a terrain catalogue, a compiled [mission](/documentation_v2/glossary/g_to_m.md#mission)) is rebuilt |
| `catalogs/` | the live item [registry](/documentation_v2/glossary/n_to_z.md#registry), production content exported from the [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and imported by the platform | new game content, replaced whole by a re-export and never edited by hand |
| `fixtures/` | ground truth for tests: samples that must parse, and wrapped missions that must be rejected | a test-surface change; an invalid fixture that starts passing is a regression in a rejection gate |

The live catalogues stay out of `fixtures/` on purpose: the gates and the import read each from its
own folder, so an import can never fall back to a sample and fill the
[arsenal](/documentation_v2/glossary/a_to_f.md#arsenal) with sample data while every test passes.

```text
definitions/ ──schema codegen──▶ API models/generated/ and contract/generated/
     ├── include_str! ──▶ API validators, Mission Creator zone and loadout checks
     ├── @contract tags ◀── mod DTO classes, API models (schema citations)
     └── schema gates ◀── fixtures/, catalogs/ (schema validate)

rules/ ── prefab-classify.json ──▶ world export ──▶ assets_v2/terrains catalogues
      └── kit-aliases.json ──▶ map engine mission compiler ──▶ compiled missions

catalogs/ ──db registry-import──▶ Postgres registry tables ──▶ /api/v1/registry
```

The API's release image copies `definitions/` and `rules/kit-aliases.json`, since the API embeds
them at compile time (`apps/website/Dockerfile`).

## Getting started

Run these from the repository root after a change here; none needs a database or a server:

```bash
cargo xtask schema validate     # every fixture, catalogue and committed manifest against its schema
cargo xtask schema codegen      # regenerate the API's typed models after a schema change
cargo xtask ci ci-local-schema  # codegen freshness, the schema-validate task, citations
```

`cargo xtask schema validate-file <path>` checks one mission file on its own, and
`cargo xtask schema map-object-golden` runs the world export's semantic gates over
`fixtures/map/`. `cargo xtask db registry-import` loads `catalogs/` into the development database
once `cargo xtask db up` has started it.

## Boundaries

- Depends on: the mod's spawn registry `apps/mod/tbd-framework/Data/registry.json`, which the kit
  aliases and the mission fixtures must agree with; the registry export plugin in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/`, which produces the catalogues; the binary chunk and
  density formats of `apps/website/map-engine/src/io/`; and the glyph keys of
  `assets_v2/glyphs/manifest.json`.
- Used by:
  - `apps/website/api_v2/`: generated models, embedded validators, the registry import binary and
    the contract test suites;
  - `apps/website/frontend/`: the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
    schema embeds for zones and loadout export;
  - `apps/website/map-engine/`: the embedded kit-alias table and the compiler's fixture tests;
  - `apps/mod/`: DTO classes whose `@contract` tags cite the mission, loadout and registry schemas;
  - `apps/fleet_host_agent/`, whose ledger client follows the fleet-command schema;
  - `tools_v2/xtask/` (the schema gates, codegen, `db registry-import` and the `mod` commands that
    stage fixture missions) and `tools_v2/developer-tools/` (world export, blueprint compiler and
    map verification), which find these folders through
    `tools_v2/developer-tools/src/repository_layout.rs`;
  - the `schema.yml` and `contracts.yml` workflows, which run on every change under
    `contracts_v2/`.
- Rules:
  - the tree holds data only, never code;
  - readers ship before writers emit: a field lands in its schema and in the readers before any
    writer puts it on the wire, as the version 1.3 mission fixtures do ahead of the Mission Creator;
  - a schema change updates its generated types (`cargo xtask ci verify-codegen-fresh`), every
    fixture it validates (`cargo xtask schema validate`) and every `@contract` citation of it
    (`cargo xtask schema citations`) in the same change;
  - live exports stay in `catalogs/` and samples in `fixtures/`.

## Related documentation

- [Schema evolution policy](/documentation_v2/contracts_v2/schema_evolution_policy.md) — how a
  schema may change: additive rules, versions, breaking changes and the steps of a change.
- [TBD Voice game bridge contract](/documentation_v2/contracts_v2/definitions/bridge_messages.md)
  — the transport, envelope and lifecycle of the voice bridge messages.
