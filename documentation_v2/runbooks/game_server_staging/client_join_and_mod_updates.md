**Status:** live

# Join the staging server and update its mod

Joins an Arma Reforger client to the staging server by Direct Join, diagnoses a join that fails,
and gets a script change of the [mod](/documentation_v2/glossary/g_to_m.md#mod) to the server and to the
players. The server side needs only a redeploy; players need a Workshop publish.

## Prerequisites

- A server deployed in config mode whose boot verdict passed
  ([staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md)): only config
  mode registers the backend room a client joins.
- The host's firewall open on UDP and TCP 2001 and UDP 17777
  ([host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md)).
- A client on the same game version as the server.

## Steps

1. Read the join address and the Direct Join Code from the server's newest log. The code is minted
   on every boot, so it always comes from the current log.

   ```bash
   ssh <TBD_SSH_HOST> 'grep -h -E "Server registered with address:|Direct Join Code:" "$(ls -1d <TBD_PROFILE_DIR>/logs/logs_* | tail -1)/console.log"'
   ```

   Expected: `BACKEND      : Server registered with address: <address>:2001` and
   `BACKEND      : Direct Join Code: <code>`. The address is `TBD_PUBLIC_ADDRESS`; when it is not
   the host's LAN address, set it in `deploy.env` and redeploy.

2. In the client, open Multiplayer, then Direct Join, and enter `<address>:2001` or the Direct
   Join Code. Then, as a listed admin, type `#tbd` in chat.

   Expected: the client loads into the server's lobby, and `#tbd` answers with the [missions](/documentation_v2/glossary/g_to_m.md#mission) the
   platform lets this server deploy, numbered from 1; a player not in `game.admins[]` gets
   "TBD: admin only.".

3. When the join fails, probe the join path from a development machine on the same network before
   the attempt. The argument is a run id written into the report.

   ```bash
   cargo xtask debug direct-join before-join
   ```

   Expected: `Wrote debug log: <checkout>/.cursor/debug-8fc1e0.log`, then a `--- summary ---`
   with the Steam build ids of the client and dedicated server installed on this machine, the
   client addon link, the host's `tbd-reforger.service` state and UDP listeners on 2001 and
   17777, the last listen, A2S and client lines of its newest log, and the ping and A2S answers.
   Repeat with `after-join` right after a failed attempt and compare the two blocks.

4. On the client machine, list the join flow of the client's newest log. Under Steam's Proton the
   client log sits in the compatdata folder of app 1874880.

   ```bash
   grep -E 'SEARCHING_SERVER|SERVER_NOT_FOUND|MANUAL_CONNECT|connect' "$(ls -td ~/.local/share/Steam/steamapps/compatdata/1874880/pfx/drive_c/users/steamuser/Documents/My\ Games/ArmaReforger/logs/logs_* | head -1)/console.log"
   ```

   Expected: the client's search and connect lines; `SERVER_NOT_FOUND` means it reached no
   registered room.

## Verify

The client stands in the lobby and the server's log shows it:

```bash
cargo xtask mod remote-logs
```

Expected: once the player has taken a [slot](/documentation_v2/glossary/n_to_z.md#slot), `VERDICT: PASS — boot healthy and at least one player
was seated.`, exit 0.

## Launch flags and ports

The deploy writes the launch line into the unit
([staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md)); by hand the
two modes read:

```text
config:  ArmaReforgerServer -addonsDir <dir> -config <server.config.json> -profile <dir> -maxFPS 60 -logStats 30000 -nothrow
addons:  ArmaReforgerServer -profile <dir> -addonsDir <dir> -addons <GUID> -server "<scenario>" -bindIP 0.0.0.0 -bindPort 2001 -a2sPort 17777 -maxFPS 60 -logStats 30000 -nothrow
```

The standard layout is game 2001, A2S 17777, [RCON](/documentation_v2/glossary/n_to_z.md#rcon) 19999.
The A2S port is a separate UDP socket from the game port and must differ from it: equal ports log
`Starting RPL server, listening on …:2001`, then `NETWORK (E): Unable to start replication`,
`Unable to initialize the game` and `Game destroyed`, and the process exits with status 0, so
`Restart=on-failure` does not restart it. Addons mode loads the checkout and reaches LOBBY, which
suits headless log checks, but registers no room (no `Server registered with address:`, no
`Direct Join Code:`, no `Loading dedicated server config`) and has no admins.

Client and server must run the same game version: compare the version string both logs print,
not Steam build ids, which differ between the client app (1874880) and the server app.

## Getting a script change to the server and the players

The server loads the synced checkout through `-addonsDir`, so a script change reaches it with a
redeploy:

1. Compile the mod headless; exit 3 means `apps/mod/tbd-framework/resourceDatabase.rdb` is stale
   and does not register a new `.c` file, which [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
   fixes when it next loads the addon.

   ```bash
   cargo xtask mod compile
   ```

   Expected: exit 0.

2. Redeploy; the boot verdict proves the synced checkout is what loaded.

   ```bash
   cargo xtask deploy staging
   ```

   Expected: `==> deploy complete`, exit 0.

A joining client, though, resolves `game.mods[]` (`B2C3D4E5F6A78901`) from the Workshop, so it can
run a different build of the mod from the server it joins; no check in the repository covers what
a client loads. When a change must reach players, publish `tbd-framework` from Workbench (the
Workshop publish, under the same id as the gproj GUID; set the licence field to a real licence
file). Publishing packs files into the addon folder, which makes Workbench show the addon
read-only; delete them and restart the launcher:

```bash
rm -f apps/mod/tbd-framework/data.pak apps/mod/tbd-framework/meta apps/mod/tbd-framework/ServerData.json apps/mod/tbd-framework/*_manifest.json
```

All four are gitignored. A publish does not update the server, and it can hide a broken deploy:
with the Workshop copy current, a server that loaded it instead of the checkout looks right in
every log line. The boot verdict's gproj path is the check that tells them apart.

To load the checkout in a local client instead of the Workshop copy,
`cargo xtask setup client-addons` links `apps/mod/tbd-framework/` into
`~/.local/share/tbd-server-addons/` and prints the Steam launch options
(`-addonsDir "<that folder>" -addons B2C3D4E5F6A78901`); ignore the host address its last line
prints.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Direct Join answers "No server found" | the server registered no room: addons mode, or a boot that died | deploy in config mode; check `Server registered with address:` in the log |
| the server "passes" but behaves like old code | a Workshop copy won over the checkout | [boot verdict](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md); the unit must carry `-addonsDir` and `-config` |
| `#tbd` answers "TBD: admin only." to every player | `game.admins[]` is empty, or the server is in addons mode, which loads no config | set `TBD_ADMIN_IDENTITY_IDS` and use config mode; `passwordAdmin` does not feed that list |
| the join is refused for a version mismatch | client and server game versions differ | update the server app with steamcmd, or the client through Steam |
| `Unknown class TBD_…` on the server | the synced `resourceDatabase.rdb` does not register the class | `cargo xtask mod compile` (exit 3 names a stale rdb), then redeploy |
| "High ping server" warning from a server on Wi-Fi | the host is on Wi-Fi | harmless on the same subnet when `ping` answers |
| no client `console.log` | the Proton prefix of app 1874880 holds it | the path in step 4 |
| Workbench shows `tbd-framework` read-only | a publish left `data.pak` and `meta` in the addon folder | the `rm` above, then restart the launcher |

## Related

- [Debug command group](/tools_v2/xtask/src/commands/debug/README.md) — every probe
  `debug direct-join` runs.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — a joinable
  server on a development machine with `cargo xtask mod playtest`.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the Workbench and gate
  cycle before a deploy.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index.
