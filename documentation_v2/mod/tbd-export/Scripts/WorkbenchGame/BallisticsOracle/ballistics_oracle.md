**Status:** live

# Ballistics oracle

The ballistics oracle asks Arma Reforger's own engine how the vanilla mortar shells fly, and
writes the raw answers, stamped with the game build and hashed, so the platform's mortar
trajectory model can be checked against the game rather than against hand-copied tables. It
serves whoever builds and uploads a ballistics catalog's calibration fixtures.

## Where it lives

- Code: [the Workbench plugin](/apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/)
  (`TBD_BallisticsOraclePlugin.c`, `TBD_BallisticsOracleForwardAngles.c`) and
  [the game scripts](/apps/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/)
  (`TBD_BallisticsOracleSimulationComponent.c` and the shared classes).
- Entry: the Workbench menu entry Plugins > TBD > Ballistics Oracle
  (`TBD_BallisticsOraclePlugin`), then playing the export world, whose game mode prefab
  `apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et` carries
  `TBD_BallisticsOracleSimulationComponent`.
- Related: the [export addon index](/documentation_v2/mod/tbd-export/README.md); the
  calibration bundle contract `contracts_v2/definitions/ballistics-calibration.schema.json`,
  whose `oracle_samples` and `oracle_run` the outputs feed.

## Behaviour

The oracle has two halves, because two of the engine's answers exist in different places:

1. **Edit mode.** `BallisticTable` answers static queries from a prefab source, so the Workbench
   plugin asks them without a running world. For each of the seven vanilla mortar shells (M821,
   M879, M819, M853A1, O-832DU, D-832DU, S-832S) and each init speed coefficient (every charge
   ring plus every coefficient of the shell's native tables) it records the range and time of
   flight at every elevation from 1600 to 800 mils in 1.5625-mil steps, and, per charge, the raw aim
   height and time for altitude differences of -200 to 200 m at five distances. It writes
   `forward_angles.json` and records the generation id for step 2.
2. **Play mode.** `ProjectileMoveComponent.GetProjectileSimulationResult` needs a component
   instance, so it runs in the playing export world: the component spawns each shell and
   simulates every charge at 45, 55, 65, 75 and 85 degrees, under calm air and 5 and 10 m/s winds
   from the four cardinal bearings, to target heights of -100, 0 and 100 m, and records the world
   gravity from `PhysicsWorld.GetGravity`. It writes `simulation.json`.

Both halves write under `$profile:TBD_BallisticsOracle/<export generation id>/`; the operator
copies that folder out. The procedure, with the console lines to wait for, is in the
[plugin README](/apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/README.md#operator-procedure).

Rules the run keeps:

- The generation id is 1 to 64 characters of A-Z, a-z, 0-9, `_` and `-`; any other id writes
  nothing.
- A SHA-256 self-test against three FIPS 180-4 vectors runs first; a failure writes nothing.
- A shell whose prefab cannot be read is written with its reasons and no samples, and the status
  becomes `incomplete`; nothing is skipped silently.
- The engine documents that the simulation works only with projectile debugging enabled; a calm
  45 degree probe that stays at the launch point stops the run with status
  `simulation_unavailable`.
- A sidecar is written only when every write of its output succeeded, so a sidecar always
  describes a whole file.

## Data

- `forward_angles.json` and `forward_angles_meta.json`: format in the
  [plugin README](/apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/README.md#output).
  The forward-angle samples become the bundle's `forward_angle` oracle samples and the
  altitude-difference samples its `altitude_difference` samples.
- `simulation.json` and `simulation_meta.json`: format and decoding in the
  [game scripts README](/apps/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/README.md#output).
  Its samples become the bundle's `simulation` oracle samples, and `gravity.magnitude_m_s2`
  becomes the catalog's `gravity_m_s2` (`gravity_source` `oracle`) and the bundle's
  `oracle_run.gravity_reported_m_s2`.
- Each sidecar's `sha256` is the digest of its output's exact bytes; with `plugin_revision` and
  `run_at` it fills the bundle's `oracle_run`, and `game_build` must equal the catalog's.

### Simulation result decoding

`GetProjectileSimulationResult` returns a vector the engine documents as the end position. Every
case launches from the world origin at azimuth 0 (north, +z), so `downrange_m` is raw z,
`crossrange_m` raw x and `height_m` raw y. The engine returns no time, so the time of flight is
the shortest simulation time limit whose result lands within 0.01 m of the unlimited result,
bisected 16 times over 0 to 60 s. Each calm, level sample carries the
`BallisticTable.GetDistanceOfProjectileSource` range at the same elevation and coefficient as
`forward_angle_reference`; the decoding holds when that range equals `downrange_m`, and a
consumer checks this before using the file.

## Design

No interface beyond the Workbench dialog, which holds the one field and the buttons Run and
Cancel; the outputs are JSON for tools.

## Open work

- [T-940.10 — Mortar ballistics crate for API and offline frontend](/.ai/tickets/T-940.10.toml)
  (ready): the model this oracle calibrates; its calibration bundle carries these outputs.

## Decisions

- Both halves record raw engine answers only: the calibration compares the platform model with
  the engine, so any rounding or fitting here would hide the error it measures.
- Forward angles use the indirect-fire tables (`bDirectFire` false): mortars fire on the high
  branch, and the native tables the game shows the player are the indirect-fire ones.
- The play-mode run reads the generation id from a file the plugin writes: a component cannot see
  a Workbench dialog, and the file keeps both outputs of one run in one folder.
- The oracle carries its own SHA-256: the export addon loads without the framework addon, whose
  hash class it cannot call, and the sidecar must describe the exact bytes on disk.
- The time of flight of a simulation comes from bisecting the time limit: the engine call returns
  only a position, and the bisection resolves the time to about 1 ms, well inside the 0.1 s
  calibration tolerance.
