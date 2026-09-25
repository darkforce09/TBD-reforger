# Everon surface anchors

Ground heights that [Enfusion](/documentation_v2/glossary.md#enfusion) itself reports at eleven
named points of Everon, the oracle a gate checks the elevation model against. Because the height
map is a resample of the same engine probe, an anchor that drifts means a re-export has moved the
ground under the placed objects.

## Contents

```text
assets_v2/terrains/everon/anchors/
├── surface-y-log.txt          the raw Workbench height probes the anchors were reduced from
├── verification.example.json  a one-anchor sample of the anchors shape, for the schema gate
└── verification.json          the eleven probed anchors and the 1 m drift threshold the gate applies
```

## Format

- Encoding: UTF-8.
  - `verification.json`: `terrainId`, `schemaVersion` 1, `thresholdM` (1.0), the probe's
    `probedAt`, `workbenchVersion` and `demSource` (`mod-getsurfacey-resample`, as the manifest's
    `dem.source`), and `anchors[]`, each an `id`, world `x` and `z` in metres and the engine's
    `surfaceYM`.
  - `verification.example.json`: the same shape with one placeholder anchor.
  - `surface-y-log.txt`: the text log of one `getHeight` probe per anchor, run through the
    Enfusion MCP `wb_terrain` tool in [Workbench](/documentation_v2/glossary.md#workbench) against
    the Eden world, with the full-precision heights that `verification.json` rounds.
- Schema: both JSON files follow `contracts_v2/definitions/terrain-anchors.schema.json`.
- Adding a file: probe the new points in Workbench, record the log, and add the anchors to
  `verification.json`; `cargo xtask schema terrain-alignment --terrain everon --strict` then checks
  them.

## Producers and consumers

- Producers: a person, from Workbench height probes; the log records the run.
- Consumers:
  - `cargo xtask schema terrain-alignment --terrain everon`
    (`tools_v2/developer-tools/src/map_verification/labels/terrain_alignment.rs`): it validates
    the manifest and the anchors file against their schemas, checks the PNG's size against the
    manifest, samples the height map bilinearly at every anchor with the map engine's
    `sample_elevation_meters`, and fails when one differs from `surfaceYM` by more
    than `thresholdM`. It reads `verification.json`, or the example when that is missing, with a
    warning. With `--strict` it refuses the example, fewer than 10 anchors, or a manifest with no
    height map. The `cargo xtask ci verify-terrain` and `verify-terrain-strict` tasks run it.
  - `cargo xtask schema validate`, which validates `verification.example.json`
    (`tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs`).
  - Nothing reads `surface-y-log.txt`.

## Boundaries

- Depends on: the height map in `assets_v2/terrains/everon/dem/` and the `dem` block of
  `assets_v2/terrains/everon/manifest.json`.
- Used by: the xtask schema gates above.
- Rules: an anchor records what the engine reports, never a value read off the height map; a
  re-exported height map keeps every anchor within `thresholdM`
  (`cargo xtask schema terrain-alignment --terrain everon --strict`).
