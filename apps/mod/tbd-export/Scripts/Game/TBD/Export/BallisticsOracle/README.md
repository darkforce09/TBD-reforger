# Ballistics oracle game scripts

The play-mode half of the ballistics oracle and the helpers both halves share. When the export
world plays after the Workbench menu entry "Ballistics Oracle" has run, a game mode component
spawns each vanilla mortar shell and records the engine's own trajectory simulation, with the
world gravity, into `simulation.json`; the shared classes read the shell prefabs, format the JSON,
hash every output and write its sidecar.

## Contents

```text
apps/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/
├── TBD_BallisticsOracleJson.c                 `TBD_BallisticsOracleJson`: numbers, strings, vectors and UTC time as JSON
├── TBD_BallisticsOracleOutputFile.c           `TBD_BallisticsOracleOutputFile`: streamed, hashed output file and its sidecar
├── TBD_BallisticsOracleRun.c                  `TBD_BallisticsOracleRun`: generation id, game build, revision, run time, folders
├── TBD_BallisticsOracleSha256.c               `TBD_BallisticsOracleSha256`: SHA-256 of ASCII text with a FIPS self-test
├── TBD_BallisticsOracleShellSource.c          `TBD_BallisticsOracleShellSource`: the seven shells and what each prefab declares
├── TBD_BallisticsOracleSimulationComponent.c  the game mode component that writes simulation.json
└── TBD_BallisticsOracleSimulationSampler.c    `TBD_BallisticsOracleSimulationSampler`: the case lattice and one simulation sample
```

## How it works

`TBD_BallisticsOracleSimulationComponent` is an `SCR_BaseGameModeComponent` the export game mode
prefab (`apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et`) carries beside the road
exporter. `OnPostInit` reads `$profile:TBD_BallisticsOracle/active_generation.txt`, which the
Workbench plugin writes after `forward_angles.json`; with no file it logs that the run stays idle
and does nothing. Otherwise, one second later:

1. `TBD_BallisticsOracleRun.Begin` checks the generation id (1 to 64 characters of A-Z, a-z, 0-9,
   `_`, `-`), records `GetGame().GetBuildVersion()` and the UTC start time, and creates
   `$profile:TBD_BallisticsOracle/<generation id>/`. `TBD_BallisticsOracleSha256.SelfTest` must
   reproduce three FIPS 180-4 digests, or nothing is written.
2. `TBD_BallisticsOracleShellSource.Load` reads each of the seven vanilla mortar shells (M821,
   M879, M819, M853A1, O-832DU, D-832DU, S-832S) from its prefab source: the
   `ProjectileMoveComponent` `InitSpeed` and `BallisticTableConfig`, the
   `SCR_MortarShellGadgetComponent` `m_aChargeRingConfig` (rings, init speed coefficient, default
   flag per charge), and the `InitSpeedCoefficient` of every table in the config's
   `Indirect fire Table data` and `Table data` lists.
3. The header goes out: document type, schema version, generation id, game build, plugin
   revision, run time, `PhysicsWorld.GetGravity(GetGame().GetWorldEntity())` as the raw vector and
   its magnitude, the case lattice and the decoding.
4. One call-queue tick per shell charge and elevation: the shell is spawned 1000 m above the world
   origin and, for each of 9 winds (calm, 5 and 10 m/s from 0, 90, 180 and 270 degrees) and 3
   target heights (-100, 0, 100 m), `TBD_BallisticsOracleSimulationSampler.Sample` calls
   `ProjectileMoveComponent.GetProjectileSimulationResult` from the origin at azimuth 0 with speed
   `InitSpeed x coefficient`, the elevation in degrees (45, 55, 65, 75, 85), the wind velocity,
   the target height, `mustFallDown` true and a 60 s limit.
5. The shell is deleted after its last charge; after the last shell the trailer (sample count,
   shell error count, run errors, status, finish time) closes the file and the sidecar is written.

### Decoding the simulation result

The engine documents the result as the world position where the simulation ends. With the launch
at the origin and azimuth 0 (north, +z), each sample records the raw vector and its decoding:

| Field | Read as |
|---|---|
| `raw_result` | world position (x east, y up, z north) |
| `downrange_m` | raw x sin(azimuth) + raw z cos(azimuth) |
| `crossrange_m` | raw x cos(azimuth) - raw z sin(azimuth), right of the line of fire positive |
| `height_m` | raw y, relative to the launch point |
| `reached_target_height` | the end height is within 1 m of the target height |
| `time_of_flight_s` | the shortest time limit, bisected 16 times over 0 to 60 s, whose result lies within 0.01 m of the full result; null when the target height was not reached |

The engine returns no time, so the time of flight comes from the bisection on the simulation time
limit, to 60 s / 2^16. Each calm, level sample also carries `forward_angle_reference`: the
`BallisticTable.GetDistanceOfProjectileSource` range and time at the same elevation and
coefficient, indirect-fire tables. Its `range_m` matches `downrange_m` when the decoding holds; a
consumer compares the two before trusting the file.

The engine documents that the simulation runs only with projectile debugging enabled. Before a
shell is sampled, a calm 45 degree probe must travel at least 1 m; otherwise the run stops, the
status is `simulation_unavailable` and the error says to enable projectile debugging and play
again.

### Output

`$profile:TBD_BallisticsOracle/<generation id>/simulation.json` holds `document_type`
(`ballistics_oracle_simulation`), `schema_version` 1, `export_generation_id`, `game_build`,
`plugin_revision`, `run_at`, `gravity` (`source`, `raw_vector`, `magnitude_m_s2`), `lattice`,
`decoding`, `shells` and the trailer `sample_count`, `shell_error_count`, `errors`, `status`
(`complete`, `incomplete` or `simulation_unavailable`) and `finished_at`. Each shell holds
`prefab`, `prefab_guid`, `init_speed_m_s`, `ballistic_table_config`, `charge_rings`,
`native_tables`, `errors` (what the prefab read left out), `samples` and `shell_errors` (every
error of the shell, sampling included). Each sample holds `rings`,
`charge_index`, `init_speed_coef`, `inputs` (`launch_position_world`, `init_speed_m_s`,
`elevation_deg`, `azimuth_deg`, `wind_speed_m_s`, `wind_from_deg`, `wind_vector_world`,
`target_height_m`, `must_fall_down`, `max_simulation_time_s`) and `outputs`.

`simulation_meta.json` beside it holds `document_type` (`ballistics_oracle_simulation_meta`),
`schema_version`, the same run header, `finished_at`, `file`, `bytes`, `sha256` (of the file's
exact bytes) and `status`. It is written only when every write of the output succeeded.

Wind follows the meteorological convention: a wind from bearing B moves the air toward B + 180,
so `wind_vector_world` is `-speed x (sin B, 0, cos B)`.

## Authority

- Server: the simulation run. `OnPostInit` returns on a client (`RplSession.Mode()` is
  `RplMode.Client`), so the run happens in Workbench's play mode or on a dedicated or listen
  server.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `SCR_BaseGameModeComponent`, `ProjectileMoveComponent`,
  `BallisticTable`, `PhysicsWorld`, `SCR_MortarShellGadgetComponent`, `SCR_EntityHelper`,
  `Resource`, `JsonSaveContainer`, `SaveContainerContext` and `FileIO`; the vanilla mortar shell
  prefabs and their ballistic table configs.
- Used by: `apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et`, which carries the
  component; the Workbench plugin in `apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/`,
  which uses the run, shell source, JSON, output file and hash classes.
- Rules: vanilla classes only, never a Workbench class, since a game compiles `Scripts/Game/`
  alone; every output is ASCII, so the hash of the characters is the hash of the file; outputs
  stay under `$profile:TBD_BallisticsOracle/`; `PLUGIN_REVISION` in `TBD_BallisticsOracleRun.c`
  is raised whenever an output's meaning or layout changes. `cargo xtask mod compile` compiles the
  framework addon only; these scripts compile in the gate through
  `cargo xtask mod compile --probe=<folder holding copies of them>`, and in Workbench.

## Related documentation

- [Ballistics oracle](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md)
  — what the oracle measures, why, and how its outputs become calibration fixtures.
- [Ballistics oracle plugin](/apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/README.md)
  — the edit-mode half and the operator procedure.
