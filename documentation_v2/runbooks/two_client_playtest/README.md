**Status:** live

# Two-client playtest

How to run the live end-to-end playtest of the [mod](/documentation_v2/glossary/g_to_m.md#mod): one
dedicated server on a development machine, started with `cargo xtask mod playtest`, and two real
Arma Reforger clients walking a [mission](/documentation_v2/glossary/g_to_m.md#mission) from join to
round end. It is the only check in the repository that puts a player in the world; developers
and the operator who closes the playtest tickets read it.

## Contents

```text
documentation_v2/runbooks/two_client_playtest/
├── headless_preflight.md          the solo world boots that rehearse the mission before the session
├── known_limitations.md           what is known broken or unproven going in, and the open tickets
├── pass_criteria_and_evidence.md  the PASS lists, the evidence to capture, the greps, the log lines
├── playtest_server.md             `mod playtest`: dry run, boot, admin restart, stop, join checks
├── session_join_to_deploy.md      session steps S1 to S10: join, admin, identity, lobby, briefing, deploy
├── session_live_round.md          session steps S11 to S16: objectives, live, death, respawn, end
└── stack_and_mission.md           code gates, API and SPA, the authored mission, the optional event
```

## How it works

Every compile and world-boot gate runs the server with zero clients, so the per-player code — the
lobby and briefing screens, the deploy, the loadout on a player body, one life and the admin
respawn — has no other observation. One session with two clients covers it.

```text
alone, the day before                        with the second player
─────────────────────                        ──────────────────────
stack_and_mission ──▶ headless_preflight ──▶ playtest_server ──▶ session_join_to_deploy
 gates, API, SPA,      world-boot the          joinable server,     S1-S10
 authored mission      mission's bytes         admin restart              │
                                                                          ▼
                       pass_criteria_and_evidence ◀── capture ◀── session_live_round
                                                                   S11-S16
```

| Order | Runbook | When | Time |
|---|---|---|---|
| 1 | [Stack and mission](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md) | alone, before booking the second player | about 30 min |
| 2 | [Headless preflight](/documentation_v2/runbooks/two_client_playtest/headless_preflight.md) | alone; catches a mission that will not open | about 10 min |
| 3 | [Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md) | alone, then again with the admin id | a few minutes |
| 4 | [Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md) | both players | about 25 min |
| 5 | [Session: live round](/documentation_v2/runbooks/two_client_playtest/session_live_round.md) | both players | about 20 min |
| 6 | [Pass criteria and evidence](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md) | while the server still runs, then to sign off | about 10 min |

Read [Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)
before booking the second player: the lobby and briefing screens render mock data, which decides
which PASS items the session can reach.

Facts every topic relies on:

- **Where commands run.** Every command runs on the host from the repository root. An agent
  container has no C toolchain and an older glibc, so `cargo` there fails with
  ``linker `cc` not found`` and host binaries with ``GLIBC_2.39 not found``
  (`tools_v2/xtask/src/core/host_execution.rs`); neither means anything is broken.
- **The run folder.** `cargo xtask mod playtest` stages everything under `--run-dir`, default
  `$HOME/tbd-playtest`: `addons/tbd-framework` (a link to the checkout), `server.json`,
  `server.out`, `server.pid` and `profile/`, whose game data sits one level down in
  `profile/profile/` (`TBD_BackendConfig.json`, `TBD_MissionArtifactCache/`).
- **The server log.** Each boot writes a new `profile/logs/logs_<time>/console.log`. The topics
  call the newest one `$LOG`:

  ```bash
  LOG="$(ls -td "$HOME"/tbd-playtest/profile/logs/logs_* | head -1)/console.log"
  ```

- **Read the log, never the exit code.** The server binary exits 0 even when script compilation
  fails, and with equal game and A2S ports.
- **Log format.** Every current mod line reads `[TBD][<channel>] …`; the stage machine also prints
  the flat `[TBD] Stage → <stage>`. Match a line's stable prefix, never a whole sentence.

## Code

- [Playtest server](/tools_v2/xtask/src/commands/mod_ops/playtest_server/README.md) —
  `cargo xtask mod playtest`: staging, the platform deployment, the boot verdict and the stop.
- [World boot](/tools_v2/xtask/src/commands/mod_ops/world_boot/README.md) —
  `cargo xtask mod world-boot`, the headless rehearsal.
- [Mod commands](/tools_v2/xtask/src/commands/mod_ops/README.md) — the `mod` group, its exit
  contract and `mod dev-server`.
- [Development server profile](/tools_v2/xtask/dedicated_server_profiles/README.md) — the server
  config the playtest renders from.
- [TBD Framework addon](/apps/mod/tbd-framework/README.md) — the scripts whose log lines the
  session reads.

## Boundaries

- Depends on: the [runbook template](/documentation_v2/standards/templates/runbook.md); the xtask
  `mod`, `db`, `mk`, `setup`, `verify` and `ticket` command trees; the website API's
  [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) and mission routes; the mod's log
  lines under `apps/mod/tbd-framework/Scripts/Game/TBD/`; the golden missions in
  `contracts_v2/fixtures/missions/valid/`.
- Used by: the two playtest tickets, which cite this README's path; the
  READMEs of `apps/mod/`, `apps/mod/tbd-framework/` and its `Missions/` and `worlds/` folders,
  the xtask `mod_ops/`, `playtest_server/`, `setup/` and `dedicated_server_profiles/` folders; a
  code comment in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/TBD_LoadoutEquipHelper.c`
  that relies on this runbook's `loadout INCOMPLETE` and `loadout DEGRADED` greps; the mod and
  game server staging docs.
- Rules: this README keeps its path, because tickets and code cite it; a topic file stays at or
  under 500 lines; every command is checked against the xtask source and only `--help`,
  `--dry-run` and `--selftest` runs back an `Expected:` line; no document here writes a host
  address or a secret.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the same
  server on the staging host, deployed with `cargo xtask deploy staging`.
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the event loop and the one-life
  rule the session exercises.
- [TBD Framework documentation](/documentation_v2/mod/tbd-framework/README.md) — the in-game
  screens, with the lobby and briefing specifications.
- [Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
  — authoring the loadouts the session checks on a player.
