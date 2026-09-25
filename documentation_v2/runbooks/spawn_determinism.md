**Status:** live

# Spawn determinism

Checks that Workbench Play, spawn and equip give the same player-visible outcome across several
fresh [Workbench](/documentation_v2/glossary.md#workbench) processes: every run restarts Workbench,
plays the framework world, grades the log, and the normalised outcomes of all runs must be
byte-identical. Run it after a change to the framework's spawn, [slot](/documentation_v2/glossary.md#slot)
or loadout code. It needs a live Workbench and has no headless or CI path: `cargo xtask ci
ci-local` and `cargo xtask platform wave gate` never run it. Five runs take about 15 minutes. What
the spawn code does is in the [Spawning README](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md).

## Prerequisites

- Arma Reforger Tools installed through Steam (app 1874910), with the Net API listening on
  `ENFUSION_WORKBENCH_PORT` (default 5775). Check: step 2.
- The MCP bridge working, as in [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md):
  each run drives Workbench with `cargo run -q -p xtask -- mcp call` (`wb_connect`,
  `wb_open_resource`, `wb_play`, `wb_stop`), so `cargo` must run xtask where the gate runs.
- A Workbench project that loads both the framework, whose `worlds/TBD_Dev_POC.ent` the gate opens,
  and `TBD_EMCP`, which serves the `wb_*` tools. The export addon that `cargo xtask mod
  dev-bootstrap` opens does not depend on the framework, so it cannot open that world. The gate
  relaunches Workbench with `steam -applaunch 1874910` and no project argument, so Workbench must
  come back on that project by itself.
- `ss`, `pkill`, `steam` and `sha256sum` on `PATH`.
- Workbench's console logs where the gate looks for the newest `logs_*` folder: under the Proton
  prefix of app 1874910 (`~/.local/share/Steam/steamapps/compatdata/1874910/pfx/…/ArmaReforgerWorkbench/logs`),
  else `~/Documents/Games/ArmaReforgerWorkbench/logs`.

## Steps

Run every command from the repository root.

1. Check the verdict logic offline, with no Workbench.

   ```bash
   cargo xtask mod spawn-determinism --selftest
   ```

   Expected: `ok   selftest healthy-run-passes -> 0`, `ok   selftest stale-strings-must-fail -> 1`,
   `ok   selftest mission-invalid-must-fail -> 1`, the two `ok   selftest extract-…` lines, then
   `SELFTEST: PASS` and exit 0; any `FAIL selftest …` line ends `SELFTEST: FAIL` with exit 1.

2. Check that the Workbench Net API is up; this takes seconds.

   ```bash
   cargo xtask mod spawn-determinism --preflight
   ```

   Expected: `preflight: Workbench Net API listening on :5775` and exit 0. With nothing listening
   it prints `FATAL: Workbench Net API not listening on :5775 — spawn-determinism cannot run.`,
   how to start Workbench, and exits 2.

3. Run the gate: five runs of the framework world by default. The first argument is the run count
   and the second a world resource path (`cargo xtask mod spawn-determinism 3`,
   `cargo xtask mod spawn-determinism 5 worlds/TBD_Dev_POC.ent`).

   ```bash
   cargo xtask mod spawn-determinism
   ```

   Expected: the preflight line, then for each run `── run <i>/<n> ──`, any `FAIL run <i>: …`
   lines, and `run <i> digest <12 hex digits> (<lines> lines)`; the run ends with the verdict
   under Verify. Each run kills Workbench, relaunches it through Steam when the port closed, waits
   up to 300 s for the port, connects, opens the world (up to three restart cycles when Workbench
   comes back without a working game), presses Play, waits up to `TBD_DET_TIMEOUT` seconds
   (default 120) for the `[TBD][Audit]` census line, then stops Play.

## Verify

The gate's last line is the verdict:

```text
DETERMINISM PASS: 5/5 identical (digest <12 hex digits>)
```

with exit 0. On a failure it prints `DETERMINISM FAIL — snapshots kept at <dir>`, where `<dir>` is
`tbd-spawn-det.<pid>.<nanoseconds>` in the temporary directory, holding each run's raw log and
normalised log, and exits 1. `TBD_DET_KEEP=1` keeps the snapshots on a pass too.

A run passes when its log shows:

1. a `[TBD][Audit] characters=N bodies=N players=N` census with characters equal to bodies;
2. a `[TBD][Slots] materialized <1-9>…` line;
3. no `path=vanilla-fallthrough`;
4. no `SCRIPT (E)` or `Virtual Machine Exception` line;
5. at most three `has switched from faction` lines, and none after the census;
6. no `bound player N` line twice, and no `[TBD][Loadout]` line with `FAILED` or `not worn`.

The digest is the SHA-256 of the run's outcome lines: those tagged `[TBD][Spawn]`, `[TBD][Slots]`,
`[TBD][Loadout]`, `[TBD][Audit]` and `[TBD][Stage]`, the line reporting the loaded
[mission](/documentation_v2/glossary.md#mission), the roster lines and
the `bound player` and `assigned slot` lines, with timestamps, entity ids, positions and timings
masked, an item already worn or equipped collapsed to `GEAR-ENSURED`, then sorted and
deduplicated. Every run's digest must equal run 1's; a difference prints
`FAIL: run <i> digest differs from run 1 — first divergence:` and the differing lines. The runs are
compared with each other, never with a stored digest, so a deliberate change to the spawn lines
needs no re-baselining.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `FATAL: Workbench Net API not listening on :5775 …`, exit 2 | Workbench is not running, or its Net API is off | start Workbench on the project in the prerequisites and enable the Net API |
| `FATAL: Workbench did not come back on :5775`, exit 2 | after the kill, Workbench did not reopen its Net API within 300 s, for example when it stops at the project picker | open the project by hand, then run step 3 again |
| `WARN: Workbench came up game-dead (try <n>) — cycling again`, then `FATAL: Workbench game-dead after 3 restart cycles` | Workbench reported "Can't initialize the game", which a restart racing Steam and a Game-module script compile error both produce | read the `Compiling Game scripts` section of Workbench's boot log first; `cargo xtask mod compile` |
| `FATAL: no console.log found`, exit 2 | no `logs_*` folder in either log location | check where Workbench writes its logs on this machine |
| `FATAL: wb_play failed`, exit 2 | the MCP call to Play failed | `cargo xtask mcp smoke`, then [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) |
| `FAIL run <i>: sentinel not seen within 120s` | the census line never appeared: the mission did not load or the round did not reach it | read the kept raw log; raise `TBD_DET_TIMEOUT` for a slow machine |
| `FAIL run <i>: faction switch AFTER census — churn loop alive` | vanilla's registration re-rolls the player's faction after the framework seated them | check the vanilla stand-down in the Spawning README |

### Why each run restarts Workbench

`TBD_MissionLoader` and `TBD_RosterLoader` keep static state across Play sessions inside one
Workbench process, so a second Play in the same process skips the fetch and settle the gate has to
exercise. The gate therefore kills Workbench (`pkill -f WorkbenchSteamDiag`) and relaunches it for
every run.

### Recorded results

[spawn_determinism_verify_log.md](/.ai/artifacts/spawn_determinism_verify_log.md) records two
Workbench runs of the gate: a 5-of-5 pass (digest `3e31fd8cf7c6`, 18 lines) for the spawn pipeline
that preceded the possess-route deploy, and a run stopped after 2 of 5 (both digest
`6abcbdd84e02`, 21 lines) for the possess-route deploy the framework uses. No five-run pass of the
current spawn code is recorded; the gate prints its verdict only to the terminal, so a full run
is added to that log by hand.

## Related

- [Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md) — slot bodies,
  the possess deploy, one life and the vanilla stand-down the gate exercises.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the `mcp call` path
  the gate drives Workbench through, and `cargo xtask mcp wb-logs`, a single Play log's spawn
  verdict.
- [Mod verification gates](/tools_v2/xtask/src/verifications/mod_scripts/README.md) — the gate's
  code beside the other mod script checks.
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the framework's spawn rules.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — where the gate sits in
  mod work.
