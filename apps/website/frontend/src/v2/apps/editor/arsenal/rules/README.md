# Arsenal loadout rules

The decisions the [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal) makes about a loadout, with no
rendering and no browser: the compatibility graph and the options each row offers, cargo
defaults and capacity, the loadout-export schema check, and the paper-doll regions and weight.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/arsenal/rules/
├── cargo_capacity_and_delivery.rs    cargo defaults, capacity budgets, capacity and unworn findings
├── compatibility_and_row_options.rs  `CompatGraph`, `CompatFeed`, `row_options`, `validate_loadout`
├── export_schema_contract.rs         the embedded loadout-export schema and its fail-closed checker
├── export_schema_validation.rs       per-node schema checks and the bounded pattern matcher
└── paper_doll_and_weight.rs          the doll and rail regions, `loadout_weight`, `index_by_name`
```

## How it works

Every function takes the flat [registry](/documentation_v2/glossary/n_to_z.md#registry) rows, the
compatibility edges and the current picks, and returns a value; the parent file
`apps/website/frontend/src/v2/apps/editor/arsenal/rules.rs` declares the 14 `LOADOUT_ROWS`, of
which optic and magazine are edge rows fed by the graph, and re-exports this folder's items.

- `CompatGraph::from_edges` stores every compatibility edge in both directions under its edge
  type, so `items_for(host, edge)` is one lookup. `CompatFeed` pairs the graph with its loading
  status, and until the feed is ready an edge row offers only its current pick. `row_options`
  builds a row's choices without abstract and variant rows, sorted by name, and keeps a stranded
  current pick, marked incompatible, so it can still be seen and cleared; `validate_loadout`
  returns the edge rows whose pick the graph rejects.
- Cargo sits in `CARGO_CONTAINERS` (vest, pants, jacket, backpack). `cargo_capacity_errors` refuses
  cargo over the catalogued capacity of the garment that wears the container, but never invents a
  limit the catalog lacks; `cargo_unworn_container_errors` names cargo in a container no picked
  garment wears as a warning only, because the [slot](/documentation_v2/glossary/n_to_z.md#slot)'s kit
  may supply the garment. `cargo_defaults_by_character` derives default cargo from raw
  `character_default_cargo` edges; only the tests call it, since the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) reads the server's aggregated
  cargo defaults.
- `validate_against_loadout_export_schema` checks a document against
  `contracts_v2/definitions/loadout-export.schema.json`, embedded at compile time. It first audits
  the whole schema, and any keyword outside `SUPPORTED_SCHEMA_KEYWORDS`, a `$ref` chain or a
  pattern the anchored matcher cannot evaluate is a refusal, never a pass.

## Boundaries

- Depends on: `RegistryItem` and `RegistryCompatEdge` from `crate::v2::core::api::dto`;
  `website_map_engine::data::store::operations::cargo_rules` (`CargoRow`, `cargo_from_loadout`,
  `cargo_rows_json`), re-exported by the parent; the embedded schema file.
- Used by: `rules.rs`, which re-exports the items; in
  `apps/website/frontend/src/v2/apps/editor/`:
  - the rest of the arsenal: `arsenal/mod.rs`, `arsenal/doll.rs`, `arsenal/loadout/`,
    `arsenal/loadout_commands.rs` and `arsenal/tab_content/`;
  - the Arsenal panels in `ui/arsenal/`, for the options, the cargo groups and budgets;
  - `mission_editor.rs`, `mission_editor/registry_loading.rs` and
    `mission_editor/canvas_mount/`, which build the `CompatFeed` from the registry fetch, and
    `bridge/viewport.rs` and `ui/inspector/attributes_modal.rs`, which carry it to the Arsenal
    tab.
- Rules: an unsupported schema construct refuses
  (`an_unimplemented_keyword_is_a_refusal_not_a_shrug`,
  `an_unsupported_pattern_refuses_rather_than_waving_through`); capacity never invents a limit
  (`cargo_capacity_never_invents_a_limit`); an unworn-container warning never becomes an export
  refusal (`the_unworn_warning_never_becomes_an_export_refusal`); the tests live in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tests/rules/`. `CARGO_CONTAINERS` has a hand
  copy in `apps/website/map-engine/src/data/scenario/validation/wire_safety/scan.rs`, which checks
  cargo when a [mission](/documentation_v2/glossary/g_to_m.md#mission) saves; no test holds the two equal.

## Related documentation

- [Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md) — the
  verdict, the capacity refusals and the compatibility rows as the mission maker meets them.
- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Arsenal tab whose verdict chip the rules drive.
