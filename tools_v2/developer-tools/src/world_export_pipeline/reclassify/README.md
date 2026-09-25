# Catalogue reclassification

The body of `world reclassify`: it rebuilds the classification lane of a terrain's committed
prefab catalogue from `contracts_v2/rules/prefab-classify.json` and the committed chunks alone,
with no [Workbench](/documentation_v2/glossary.md#workbench) export, and by default only reports
where the catalogue and the rules disagree.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/reclassify/
└── obj_get.rs  `reclassify_terrain`, `reclassify_rows` and the row and census rebuilders
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/reclassify.rs` holds the `Drift`, `Report`
and `Mode` types and re-exports the functions here. `reclassify_terrain` reads
`objects/prefabs.json.gz` under `assets_v2/terrains/<terrain>/` (a terrain without one is an
error), classifies every row's resource name with the current rules, and prints the kind
histograms before and after, the share of `unknown` classes, one line per drifted prefab and any
census kind no committed row carries.

- Check mode, the default, exits 0 when nothing drifted and 1 when a rule change has not reached
  the catalogue.
- `--write` rewrites `prefabs.json.gz` with the new kind, class, AI, gameplay, render and tags
  fields, rebuilds `type-inventory.json` from the instance counts in `objects/chunks/*.json.gz`
  and the road census of `objects/roads.json.gz`, re-emits the catalogue's `.rkyv` twins, and
  exits 0. `--out <dir>` writes under that folder instead, relative to the checkout root.

A measured `spatial` block is kept as it is, because its extents come from the Workbench export;
one equal to the old rule's template is replaced by the new rule's template. The census's
`needsReview` list counts raw rows the catalogue never held, so it is kept, not recomputed.

## Boundaries

- Depends on: the sibling modules `classify` (the rules and the classifier), `chunk_partitioner`
  (`gunzip`, `gz9`, `road_census`), `catalog_emit` and `json_number_formatting`;
  `crate::repository_layout`.
- Used by: `world reclassify` (`tools_v2/developer-tools/src/world_export_pipeline/cli.rs`); the
  platform wave gate, which runs it in check mode
  (`tools_v2/xtask/src/commands/platform/wave_execution/gate/gate_dispatch.rs`,
  `tools_v2/xtask/src/commands/platform/wave_execution/gate/checkrun.rs`).
- Rules: empty rules or an empty catalogue are refused rather than reported as drift, and a
  measured `spatial` block survives a rebuild
  (`empty_rules_are_refused_not_reported_as_massive_drift`,
  `measured_spatial_is_preserved_and_template_spatial_is_reclaimed` in
  `tools_v2/developer-tools/src/world_export_pipeline/tests/reclassify/tests.rs`).
