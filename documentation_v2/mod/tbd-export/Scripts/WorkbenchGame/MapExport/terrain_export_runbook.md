**Status:** live

# Rebuild a terrain's object and road data from a Workbench export

Takes one full world-object export of a terrain out of
[Workbench](/documentation_v2/glossary.md#workbench), stages it, builds the terrain's object and
road data for one import phase, verifies it and opens the next phase. It follows the operator steps
`cargo xtask map export-terrain` prints (`tools_v2/xtask/src/commands/map/terrain_export.rs:61-84`)
and the gate's own refusal text. The build and verify steps run for minutes on a full terrain.

## Prerequisites

- Arma Reforger Tools installed through Steam, with `apps/mod/tbd-export/addon.gproj` opened and
  the terrain's world loaded with all its layers; `wb_state` reports about a million entities or
  more on Everon.
- A way to run `TBD_WorldFullExportPlugin`. Its menu entry is commented out
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/TBD_WorldFullExportPlugin.c:19`) and
  no committed handler calls it, so from a clean checkout step 1 cannot run; see the
  [map export feature doc](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md#entry-points).
  A full export already in the profile lets you start at step 2.
- The terrain listed in `assets_v2/terrains/terrain-registry.json`, with an `importPhaseMax` at or
  above the phase you build.
- The Workbench profile folder in `PROFILE_DIR`. Under Proton it is
  `~/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile`.

## Steps

1. Run the full world-object export in Workbench. The command the operator steps name is below; it
   works only once the plugin has a menu entry again.

   ```bash
   MCP_CALL_TIMEOUT=3600 cargo xtask mcp call wb_execute_action '{"menuPath":"Plugins,TBD,Export TBD World Objects (full)"}'
   ```

   Expected: `$PROFILE_DIR/TBD_WorldExport_full.jsonl`, then `TBD_WorldExport_full_meta.json`,
   written last as the completion sentinel.

2. Stage the export into the terrain's scratch folder (gitignored).

   ```bash
   cargo run -q -p developer-tools --bin world -- copy-export-profile --terrain everon --full --profile "$PROFILE_DIR"
   ```

   Expected: `copy-world-export-profile: everon FULL — staged <n> rows →
   …/assets_v2/scratch/everon/export/raw-entities.jsonl; meta + stagedAt stamp written`.

3. Build the object and road data for the phase.

   ```bash
   cargo xtask map export-terrain everon --phase P5_props
   ```

   Expected: `export-terrain: everon P5_props — building catalog artifacts`, the output of
   `world build-objects` and `world build-roads` once each finishes, then
   `export-terrain: everon P5_props done — next: cargo run -q -p developer-tools --bin world -- verify-phase --terrain everon --phase P5_props`.
   The phase defaults to `P1_buildings`; `TERRAIN` in the environment stands in for the first
   argument.

4. Verify the phase.

   ```bash
   cargo run -q -p developer-tools --bin world -- verify-phase --terrain everon --phase P5_props
   ```

   Expected: one `PASS` line per gate, then `map-verify-phase: OK — everon P5_props (<n> prefabs,
   <n> instances, <n> chunks, <n> road segments, chunk gz <n> KB)`.

5. Open the next phase: set the terrain's `importPhaseMax` in
   `assets_v2/terrains/terrain-registry.json` to the next phase in the order `P1_buildings`,
   `P2_trees`, `P3_vegetation`, `P4_rocks`, `P5_props`, and commit it with the built data.

   ```bash
   git diff -- assets_v2/terrains/terrain-registry.json assets_v2/terrains/everon/manifest.json
   ```

   Expected: the registry row's `importPhaseMax` raised by hand, and the manifest's
   `importPhaseMax` and `importPhaseShipped`, which `build-objects --patch-manifest` wrote in step 3.

## Verify

The committed chunks and road data validate against their shapes:

```bash
cargo run -q -p developer-tools --bin world -- verify-phase --terrain everon --phase P5_props
```

Expected: exit 0 and the `map-verify-phase: OK` line.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `export-terrain: phase P6_roads_highway blocked — registry importPhaseMax=P5_props (advance only after map-verify-phase PASS + registry bump)` | the phase is above the registry's `importPhaseMax` (`tools_v2/developer-tools/src/world_export_pipeline/mathematical_verification/phase_validation.rs:34-39`) | verify the current phase, then raise `importPhaseMax` (step 5) |
| `export-terrain: staged raw export missing for 'everon':`, exit 2 | no staged `raw-entities.jsonl` under `assets_v2/scratch/everon/export/` | run steps 1 and 2 |
| `copy-world-export-profile: source jsonl not found: …` | the profile path is wrong; the default is `$HOME/Documents/Games/ArmaReforgerWorkbench/profile`, not the Proton profile | pass `--profile "$PROFILE_DIR"` or `--src` |
| `copy-world-export-profile: --full refused — completion-sentinel meta missing: …` | the export crashed or is still running; the meta is written last | rerun step 1 and wait for the meta file |
| `copy-world-export-profile: --full refused — meta.keptCount … != staged line count …` | the copied file is truncated; the staged copy is removed | rerun step 2, or step 1 if the source file is short |
| `verify-phase: phase 'P6_roads_highway' not implemented` | `verify-phase` and `build-objects` implement `P1_buildings` to `P5_props` only | stay at or below `P5_props` |
| `verify-phase: staged raw missing (…) — run cargo xtask map export-terrain first`, exit 2 | step 2 has not run on this checkout | run steps 2 and 3 |

## Related

- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  every exporter, its entry point and the known gaps in this procedure.
- [World export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) — the
  `world` subcommands this runbook runs.
- [Map commands](/tools_v2/xtask/src/commands/map/README.md) — `cargo xtask map export-terrain`.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — `cargo xtask mcp call`
  and its timeout.
