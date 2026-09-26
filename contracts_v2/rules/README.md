# Contract rules

The deterministic lookup tables the platform applies while it produces data: the classification of
every [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) prefab the world export meets, and the kit
and vehicle aliases a compiled [mission](/documentation_v2/glossary/g_to_m.md#mission) names. What they
shape is a committed artifact or a compiled document, so an edit here takes effect only when that
output is rebuilt.

## Contents

```text
contracts_v2/rules/
├── kit-aliases.json      Enfusion resource name to `kit:` and `veh:` alias, with faction defaults
└── prefab-classify.json  Enfusion prefab to world-object classification, first matching rule wins
```

## How it works

### Prefab classification

`prefab-classify.json` holds `schemaVersion`, a `description`, 98 ordered `rules` and one
`fallback`. Each rule matches when any substring in its `match.resourceNameContains` occurs in a
prefab's resource name (case-sensitive), and emits the prefab's full classification: `kind` and
`class`, an `ai` block (summary, taxonomy path, confidence), a `spatial` model (oriented box,
pivot, half extents, height), `gameplay` properties (cover, movement blocking, line of sight,
destructible, climbable, enterable and the rest) and, for 51 of the rules, a `render` block whose
`iconKey` selects a glyph in `assets_v2/glyphs/manifest.json`. A rule without `render` produces a
prefab the map does not draw, such as a location composition.

Rules are evaluated in file order and the first match wins, so a new rule is appended: an appended
rule can only claim prefabs that would otherwise fall through to `fallback`, while an inserted one
silently reclassifies whatever the rules below it matched. A rule that must win over a broader one
sits above it. The fallback classifies anything unmatched as an unknown prop with `confidence: 0.0`
and `needsReview: true`, which keeps an unclassified prefab visible rather than plausible.

An edit changes nothing on the map until the catalogue is rebuilt: the committed
`assets_v2/terrains/<terrain>/objects/prefabs.json.gz` and its `prefabs.rkyv` archive carry the
classification of the last export. `cargo run -q -p developer-tools --bin world -- reclassify`
re-derives the classification from committed artifacts and exits 1 on drift, and `--write`
rewrites the catalogue and its archives; a full rebuild is `cargo xtask map export-terrain`.

### Kit aliases

`kit-aliases.json` is the inverse of the [mod](/documentation_v2/glossary/g_to_m.md#mod)'s spawn registry
`apps/mod/tbd-framework/Data/registry.json`, which maps each alias to a prefab. `kits` (15 rows)
maps a character prefab's resource name to its `kit:` alias, `vehicles` (222 rows) maps a vehicle
prefab to its `veh:` alias, `factionDefaults` gives each side (`blufor`, `opfor`, `indfor`, `civ`)
the `kit:` and `preset:` a [slot](/documentation_v2/glossary/n_to_z.md#slot) falls back to, and
`fallbackFaction` names the side used when a faction is unknown. The map engine's mission flatten
embeds the file at compile time and resolves aliases while compiling the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s payload into a mission: a slot
whose character has no kit row flattens to its faction's default kit with a warning, and a placed
vehicle with no vehicle row fails the compile.

## Format

- Encoding: UTF-8 JSON, one table per file, named `<subject>-<purpose>.json`. Both files open with
  a documentation string (`description` or `$comment`).
- Schema: no JSON Schema. `prefab-classify.json` is read by the classifier in
  `tools_v2/developer-tools/src/world_export_pipeline/classify.rs`, and its `kind` and `class`
  values must belong to the enums of `contracts_v2/definitions/map-object-enums.schema.json`.
  `kit-aliases.json` is read into the `KitAliasesRaw` structure of
  `apps/website/map-engine/src/data/scenario/compiler/kit/aliases.rs`.
- Adding a file: a new table needs a reader in code; add rows to the existing tables, then run
  `cargo xtask schema validate` and `cargo xtask schema map-object-enums`.

## Producers and consumers

- Producers: people edit both files. The `kits` rows are derived from the `kit:` entries of the
  spawn registry, and the `vehicles` rows from the `vehicle` items of
  `contracts_v2/catalogs/registry-items.workbench.json`.
- Consumers:
  - the world export's classifier, `tools_v2/developer-tools/src/world_export_pipeline/classify.rs`,
    which reads `prefab-classify.json` through `prefab_classify_path`
    (`tools_v2/developer-tools/src/repository_layout.rs`), and its `reclassify` command;
  - `cargo xtask schema map-object-enums`
    (`tools_v2/xtask/src/verifications/schemas/checks/object_enumerations.rs`), which checks every
    rule's and the fallback's `kind` and `class` against the closed enums;
  - the [wave](/documentation_v2/glossary/n_to_z.md#wave) gate's catalogue-drift step in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate/checkrun.rs`, which runs `reclassify`;
  - the map engine's mission compiler, which embeds `kit-aliases.json`
    (`apps/website/map-engine/src/data/scenario/compiler/kit/aliases.rs`), and through it the
    [API](/documentation_v2/glossary/a_to_f.md#api)'s mission compile; the API's release image copies the
    file (`apps/website/Dockerfile`);
  - `cargo xtask schema validate`
    (`tools_v2/xtask/src/verifications/schemas/checks/mission_validation.rs`), which requires every
    `kit:` alias of the spawn registry to appear in `kits` with the same prefab and the reverse, and
    every faction default to resolve in the registry.

## Boundaries

- Depends on: the enums of `contracts_v2/definitions/map-object-enums.schema.json`, the glyph keys
  of `assets_v2/glyphs/manifest.json`, the spawn registry
  `apps/mod/tbd-framework/Data/registry.json` and the vehicle items of
  `contracts_v2/catalogs/registry-items.workbench.json`.
- Used by: the developer tools' world export, the map engine's mission compiler and the API that
  links it, the xtask schema gates and the wave gate.
- Rules: every classification uses closed enum values (`cargo xtask schema map-object-enums`); new
  classification rules are appended, never inserted above a rule they would steal from; the kit
  table mirrors the spawn registry both ways (`cargo xtask schema validate`); a rule edit ships
  with a rebuilt catalogue, or the wave gate's catalogue-drift step fails.
