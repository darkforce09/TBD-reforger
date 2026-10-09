# Ballistics oracle plugin

The [Workbench](/documentation/glossary/n_to_z.md#workbench) menu entry that asks the engine's
own ballistic tables, in the editor, how far and how long each vanilla mortar shell flies at every
charge and elevation, and records the raw answers with the game build, so the platform's
trajectory model can be calibrated against the game. The play-mode half, which records the
engine's trajectory simulation and the gravity, lives in
`mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/
├── TBD_BallisticsOracleForwardAngles.c  `TBD_BallisticsOracleForwardAngles`: writes forward_angles.json
└── TBD_BallisticsOraclePlugin.c         the menu entry Plugins > TBD > Ballistics Oracle and its dialog
```

## How it works

`TBD_BallisticsOraclePlugin` is a `WorkbenchPlugin` registered by `[WorkbenchPluginAttribute]` as
"Ballistics Oracle" in the `TBD` category, with the `ResourceManager` and `WorldEditor` modules.
`Run` opens a dialog with one field, the export generation id, and the buttons Run and Cancel. On
Run it:

1. starts a `TBD_BallisticsOracleRun`: the id must be 1 to 64 characters of A-Z, a-z, 0-9, `_`
   and `-`; the game build and the UTC start time are recorded and
   `$profile:TBD_BallisticsOracle/<generation id>/` is created;
2. refuses to write anything when `TBD_BallisticsOracleSha256.SelfTest` fails;
3. writes `forward_angles.json` and its sidecar through `TBD_BallisticsOracleForwardAngles.Write`;
4. records the id in `$profile:TBD_BallisticsOracle/active_generation.txt` for the play-mode run,
   unless the output failed;
5. prints `[TBD Ballistics Oracle] forward_angles.json <status> in <n> ms, sha256 <digest>,
   folder <folder>`.

`TBD_BallisticsOracleForwardAngles` reads each vanilla mortar shell through
`TBD_BallisticsOracleShellSource` (the charge rings and the native table coefficients of its
`BallisticTableConfig`), then, per shell:

- forward angles: for every charge coefficient, then every native table coefficient no charge
  uses, `BallisticTable.GetDistanceOfProjectileSource(angle, out time, source, coefficient, false)`
  at each elevation from 1600 to 800 mils (6400 per circle) in 1.5625-mil steps (513 elevations
  per coefficient), the angle passed in radians and the indirect-fire tables selected; the step is
  one sixteenth of 25 mils, so every native table row on a 25 or 12.5-mil step is a lattice point;
- altitude differences: for every charge, the longest lattice range of its coefficient, then
  `BallisticTable.GetAimHeightOfProjectileAltitudeFromSource(distance, out height, out time,
  source, difference, coefficient)` at 0.2, 0.35, 0.5, 0.65 and 0.8 of that range and at
  -200, -100, -50, 0, 50, 100 and 200 m.

Every answer is written raw beside its inputs; nothing is rounded, filtered or interpolated.

### Output

`$profile:TBD_BallisticsOracle/<generation id>/forward_angles.json` holds `document_type`
(`ballistics_oracle_forward_angles`), `schema_version` 1, `export_generation_id`, `game_build`,
`plugin_revision`, `run_at`, `elevation_lattice` (`mils_per_circle`, `first_mils`, `last_mils`,
`step_mils` 1.5625, `direct_fire`), `altitude_lattice` (`altitude_differences_m`,
`distance_fractions_of_longest_range`), `shells`, and the trailer `forward_angle_sample_count`,
`altitude_difference_sample_count`, `shell_error_count`, `status` (`complete` or `incomplete`) and
`finished_at`.

Each shell holds `forward_angle_samples`, `altitude_difference_samples`, then `prefab`,
`prefab_guid`, `init_speed_m_s`, `ballistic_table_config`, `charge_rings` (`index`, `rings`,
`init_speed_coef`, `is_default`), `native_tables` (`list`: `indirect`, `direct` or
`indirect+direct`; `init_speed_coef`) and `errors`.

| Sample | Members |
|---|---|
| forward angle | `init_speed_coef`, `coefficient_source` (`charge_ring`, `native_table` or `charge_ring+native_table`), `rings` (null for a native-only coefficient), `inputs` {`elevation_mils_6400` (an exact decimal: a whole number of sixteenths of a mil), `elevation_rad`, `direct_fire`}, `outputs` {`range_m`, `time_of_flight_s`} |
| altitude difference | `rings`, `charge_index`, `init_speed_coef`, `inputs` {`distance_m`, `altitude_difference_m`}, `outputs` {`returned`, `aim_height_m`, `time_of_flight_s`} |

`forward_angles_meta.json` beside it holds `document_type` (`ballistics_oracle_forward_angles_meta`),
`schema_version`, the run header, `finished_at`, `file`, `bytes`, `sha256` of the file's exact
bytes, and `status`; it is written only when every write succeeded.

### Operator procedure

The oracle runs only in Workbench, which the operator drives:

1. Open `mod/tbd-export/addon.gproj` in Workbench. After these scripts change, restart
   Workbench so the WorkbenchGame module recompiles and the menu entry appears.
2. Choose Plugins > TBD > Ballistics Oracle, type the export generation id the ballistics catalog
   is trimmed from (the equipment export generation, for example `6A6F008DC5395616`), and press
   Run. Wait for the `forward_angles.json complete` line in the console; `incomplete` means a
   shell's `errors` list says what could not be read.
3. Enable the engine's projectile debugging diagnostic, which `GetProjectileSimulationResult`
   needs.
4. Open `mod/tbd-export/worlds/TBD_Export_Everon.ent` and play it. About one second in, the
   console prints `Simulation run started`, then, when every shell is sampled,
   `Simulation run complete: <n> samples`. With the vanilla shells' 31 charges the count is
   4,185 (31 charges x 5 elevations x 9 winds x 3 heights). `simulation_unavailable` means step 3
   was missed: enable it and play again.
5. Stop play mode. Delete `$profile:TBD_BallisticsOracle/active_generation.txt`, or every later
   play of the export world samples again and replaces `simulation.json`.
6. Check each output against its sidecar (`sha256sum forward_angles.json simulation.json`
   against the `sha256` of `forward_angles_meta.json` and `simulation_meta.json`), then hand the
   folder `$profile:TBD_BallisticsOracle/<generation id>/` over for the calibration bundle.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorkbenchPlugin` and `Workbench.ScriptDialog`; the engine's
  `BallisticTable`; the run, shell source, JSON, output file and hash classes in
  `mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/`; the vanilla mortar shell
  prefabs and their ballistic table configs.
- Used by: people, through the Workbench menu; the play-mode component, which reads the recorded
  generation id.
- Rules: nothing from `tbd-framework`; outputs stay under `$profile:TBD_BallisticsOracle/` and
  stay ASCII; a new plugin class appears in the menu after a Workbench cold restart.
  `TBD_BallisticsOracleForwardAngles` uses Game-module classes only, so
  `cargo xtask mod compile --probe=<folder holding copies of it and the game scripts>` compiles it;
  the plugin class compiles only in Workbench.

## Related documentation

- [Ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md)
  — what the oracle measures, why, and how its outputs become calibration fixtures.
- [Ballistics oracle game scripts](/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/README.md)
  — the play-mode simulation run, the shared classes and the simulation output.
