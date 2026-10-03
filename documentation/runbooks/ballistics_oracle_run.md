**Status:** live

# Run the ballistics oracle and publish a calibrated catalog

Checkpoint W of the game ballistics: record the engine's own mortar shell flights in Workbench, trim
them with the equipment export into a ballistics catalog and its calibration bundle, prove the
flight model reproduces them, and publish the pair through the API. Run it after a game build
changes the mortar shells, or after the oracle scripts change. The Workbench part takes about a
minute of engine time (the forward angles about 10 s, the play-mode simulation under a minute);
the trim and the checks a few minutes.

## Prerequisites

- The gameplay equipment export of the game build, gitignored under
  `assets/equipment/gameplay/generations/<generation id>/export/`, written by the tbd-export
  equipment export plugin; its generation id (16 uppercase hexadecimal digits, for example
  `6A6F008DC5395616`) names every step below.
- Workbench with the tbd-export addon (`apps/mod/tbd-export/addon.gproj`). The oracle is
  tbd-export only; nothing of it ships in tbd-framework.
- The Workbench profile folder in `PROFILE_DIR`, by default
  `$HOME/Documents/Games/ArmaReforgerWorkbench/profile`; the oracle writes under
  `$PROFILE_DIR/TBD_BallisticsOracle/`.
- For step 9, the local stack of [local development](/documentation/runbooks/local_development.md)
  and an administrator session (dev login with `role=admin` in development).
- For a game build after 1.8.0.13: `CATALOG_VERSION` in
  `tools/commands/ballistics_oracle_tooling/src/trim_export.rs` raised by one, so the trim writes a new
  catalog version instead of replacing the stored one. A stored version never changes.

## Steps

1. Start Workbench on the tbd-export addon. After any change to the oracle scripts, close
   Workbench and start it again: the WorkbenchGame module compiles a new plugin class only on a
   cold start, and the Enfusion MCP bridge cannot compile one.

   Expected: the menu Plugins > TBD holds "Ballistics Oracle".

2. Choose Plugins > TBD > Ballistics Oracle, type the generation id in the dialog and press Run.
   This step and the next may also be driven through the Enfusion MCP bridge
   ([Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md)); the dialog's
   generation id is typed by hand.

   Expected: the console prints `[TBD Ballistics Oracle] forward_angles.json complete in <n> ms,
   sha256 <digest>, folder <folder>`; for the vanilla shells the file is about 13.9 MB (the run of
   2026-09-28 wrote 13,888,200 bytes).

3. Open `apps/mod/tbd-export/worlds/TBD_Export_Everon.ent` and press Play.

   Expected: about one second in, `Simulation run started`; then
   `Simulation run complete: 4185 samples` (31 charges × 5 elevations × 9 winds × 3 target
   heights).

4. Stop play mode, then delete the run marker, or every later play of the export world samples
   again and replaces `simulation.json`.

   ```bash
   rm "$PROFILE_DIR/TBD_BallisticsOracle/active_generation.txt"
   ```

   Expected: no output; the folder holds only the `<generation id>/` output folder.

5. Check both outputs against their sidecars.

   ```bash
   sha256sum "$PROFILE_DIR/TBD_BallisticsOracle/<generation id>/forward_angles.json" "$PROFILE_DIR/TBD_BallisticsOracle/<generation id>/simulation.json"
   ```

   Expected: each digest equals the `sha256` of `forward_angles_meta.json` and
   `simulation_meta.json` beside it, and both sidecars say `"status": "complete"`.

6. Create the gitignored scratch folder the trim reads.

   ```bash
   mkdir -p assets/scratch/ballistics_oracle
   ```

   Expected: no output.

7. Copy the output folder into it.

   ```bash
   cp -r "$PROFILE_DIR/TBD_BallisticsOracle/<generation id>" assets/scratch/ballistics_oracle/
   ```

   Expected: `assets/scratch/ballistics_oracle/<generation id>/` holds `forward_angles.json`,
   `simulation.json` and their two `_meta.json` sidecars.

8. Trim the export and the oracle output into the catalog, the calibration bundle and the refused
   variants.

   ```bash
   cargo xtask ballistics trim-export --generation <generation id>
   ```

   Expected: exit 0; the catalog and bundle paths with their sha256, the table, row and sample
   counts, the rows fixed by a forward sample and by a lattice end, and the gravity. For build
   1.8.0.13: 31 native tables of 476 rows (414 by a forward sample, 62 by a lattice end), 31 wind
   tables, 21,111 oracle samples, gravity 9.81. The fixture README
   (`contracts/fixtures/ballistics/vanilla_mortars.v1/README.md` for version 1) records the
   provenance.

9. Validate both documents against their schemas.

   ```bash
   cargo xtask schema validate
   ```

   Expected: the ballistics section passes for the catalog, the bundle and its provenance and
   coverage.

10. Prove the flight model reproduces the new bundle.

    ```bash
    cargo test -p ballistics_calibration --locked
    ```

    Expected: every `tests::…` case `ok`, the seven
    per-shell committed-bundle tests included; a red case is a model finding, never a reason to
    widen a tolerance.

11. Publish the pair: open `/admin/ballistics-catalogs`, pick the catalog under
    `contracts/catalogs/ballistics/` and the bundle under `contracts/fixtures/ballistics/`,
    and press "Validate and store".

    Expected: "Accepted — all N calibration cases passed; the version is stored." (7,865 cases for
    version 1), and the version appears under "Stored versions" marked latest.

## Verify

The published version is served publicly with its digest as the entity tag:

```bash
curl -sI http://localhost:8080/api/v1/ballistics-catalogs/vanilla_mortars/versions/1
```

Expected: `200`, `etag` equal to the catalog sha256 the trim printed, and
`cache-control: public, max-age=31536000, immutable`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| no "Ballistics Oracle" under Plugins > TBD | the plugin class is new or changed and Workbench was not cold-started | close Workbench and start it again (step 1) |
| `forward_angles.json incomplete` | a shell's prefab or table config could not be read; its `errors` list says which | fix the named source, then run step 2 again |
| `simulation_unavailable` | the engine's trajectory simulation answered nothing to the calm 45° probe | enable the engine's projectile debugging diagnostic and play again (the runs of 2026-09-28 passed the probe without it) |
| the export world logs that the run stays idle | `active_generation.txt` is missing: step 2 did not complete | run step 2, then step 3 |
| the trim refuses a hash | an export file differs from the export manifest, or an oracle file from its sidecar | re-copy the file (step 7), or rerun the export or the oracle |
| the trim refuses an unmatched native row | no forward sample equals the row within 0.01 m and 0.001 s: the oracle lattice does not hold that row's elevation | check the plugin revision and the lattice step (1.5625 mils) in `forward_angles.json`, then rerun step 2 |
| the upload answers "Already stored — " | the version or its exact bytes are stored | raise `CATALOG_VERSION` (Prerequisites) and trim again; a stored version never changes |
| the upload answers "Refused — F of N calibration cases failed" | the model misses the game on those cases | read the listed cases; the calibration tests of step 10 fail on the same cases |

## Related

- [Ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md)
  — what the oracle measures and why.
- [Ballistics oracle plugin](/apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/README.md)
  — the edit-mode half, its dialog and its output.
- [Ballistics trim](/tools/commands/ballistics_oracle_tooling/src/README.md) — the trim command's rules
  and exit codes.
- [Game ballistics design note](/documentation/apps/api/verification_evidence/game_ballistics.md)
  — the fixture lifecycle, the calibration criterion and the operator decisions.
- [Ballistics catalogs page](/documentation/apps/frontend/pages/administration/ballistics_catalogs/ballistics_catalogs_page.md)
  — the upload screen of step 11.
