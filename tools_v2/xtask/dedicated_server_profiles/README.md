# Dedicated server profiles

The Arma Reforger dedicated-server configuration that the local mod commands start a server from.
`cargo xtask mod playtest` and `cargo xtask mod world-boot` read the profile, override the
per-run fields and write the result as the run's `server.json`.

## Contents

```text
tools_v2/xtask/dedicated_server_profiles/
└── tbd-dev-server.config.json  the development server profile: ports, the mission header, game properties
```

## Configuration

`tbd-dev-server.config.json` is a dedicated-server `-config` file in the engine's own format:

- Network: `bindAddress` and `publicAddress`, `bindPort` and `publicPort` (2001), and the `a2s`
  query block (port 17777), which must differ from the game port.
- `game`: `name` (`TBD Dev POC`), an empty join `password`, the development `passwordAdmin`,
  `scenarioId`, the [mission header](/documentation_v2/glossary.md#mission-header) the server boots
  (`{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`), `maxPlayers` (8), `visible` and
  `crossPlatform` (both false), an empty `mods` list, and `gameProperties` (view distances,
  BattlEye off, fast validation, the voice UI switches).
- `operating`: `lobbyPlayerSynchronise` on, and no navmesh streaming exclusions.

What each command overrides before the server sees the file:

| Command | Fields it writes | Reader |
|---|---|---|
| `cargo xtask mod world-boot` | `bindPort` and `publicPort` (21000 plus the process id modulo 4000), `a2s.port` (26000 plus the same), `game.mods` (the `TBD_Framework` addon GUID) | `write_server_json` in `tools_v2/xtask/src/commands/mod_ops/world_boot/compiled_lane.rs` |
| `cargo xtask mod playtest` (and `cargo xtask mod dev-server`, which forwards to it) | the bind and public addresses and ports, the `a2s` block, `game.name`, `game.scenarioId` (the profile's own unless `--scenario=<id>` is given), `game.maxPlayers`, `game.visible` (true), `game.mods` and `game.admins` | `render_server_json` in `tools_v2/xtask/src/commands/mod_ops/playtest_server/render.rs` |

Both commands also read `game.scenarioId` from the profile to name the mission header they boot.

## Installed by

- `tbd-dev-server.config.json`: never installed as it stands. `cargo xtask mod world-boot` and
  `cargo xtask mod playtest` render it into `server.json` in their run directory and start
  `ArmaReforgerServer` with `-config` pointing there. `cargo xtask deploy staging` does not read
  it: the staging server's config is rendered from `tools_v2/xtask/deploy/deploy.env`.

## Boundaries

- Depends on: the Arma Reforger dedicated server's config format, and
  `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf`, the mission header `scenarioId` names.
- Used by: `cargo xtask mod world-boot` and `cargo xtask mod playtest`, through
  `DEV_SERVER_PROFILE` in `tools_v2/xtask/src/core/repository_layout.rs`
  (`tools_v2/xtask/src/commands/mod_ops/world_boot/execution.rs`,
  `tools_v2/xtask/src/commands/mod_ops/playtest_server/usage_fail.rs`).
- Rules: the file keeps its name and folder, which the layout constants spell and
  `every_committed_location_exists_in_the_checkout` in
  `tools_v2/xtask/src/tests/repository_layout_tests.rs` checks; `mods` stays present as a list,
  because the playtest render replaces it in place and keeps the key order; the profile carries
  development values only.

## Related documentation

- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — running
  `cargo xtask mod playtest` with a second client.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  server, whose config comes from `deploy.env` instead.
