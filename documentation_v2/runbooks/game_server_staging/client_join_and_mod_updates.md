**Status:** live

# Join a staging instance and update its mod

Joins an Arma Reforger client to one instance of the staging fleet by Direct Join, diagnoses a
join that fails, and gets a script change of the [mod](/documentation_v2/glossary/g_to_m.md#mod)
to the fleet and to the players. The servers need only a redeploy; players need a Workshop
publish.

## Prerequisites

- The fleet deployed with every instance's boot verdict passed
  ([staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md)): every
  instance starts with `-config`, which registers the backend room a client joins.
- The host's firewall open on UDP and TCP 2001 to 2005 and UDP 17777 to 17781
  ([host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md)).
- The join password, which the operator keeps in `~/tbd/fleet/join-password` on the host.
- A client on the same game version as the server: the fleet runs the experimental dedicated
  server (Steam app 1890870), so the client runs the game's Experimental branch.

## Steps

1. Read instance N's join address and Direct Join Code from its newest log. The code is minted on
   every boot, so it always comes from the current log.

   ```bash
   ssh <TBD_SSH_HOST> 'grep -h -E "Server registered with address:|Direct Join Code:" "$(ls -1d ~/tbd/fleet/instance-<N>/profile/logs/logs_* | tail -1)/console.log"'
   ```

   Expected: `BACKEND      : Server registered with address: <address>:<game port>`, the game port
   being 2000 + N, and `BACKEND      : Direct Join Code: <code>`. The address is the server
   config's `publicAddress`: `TBD_PUBLIC_ADDRESS` when set, else the IPv4 address `TBD_SSH_HOST`
   resolved to when the deploy ran. When players reach the host at another address, set
   `TBD_PUBLIC_ADDRESS` in `deploy.env` and redeploy.

2. In the client, open Multiplayer, then Direct Join, and enter `<address>:<game port>` or the
   instance's Direct Join Code, then the join password when the client asks for it. Instance 1 is
   also listed in the server browser as "TBD Staging 1"; instances 2 to 5 are not listed and take
   Direct Join only. Then, as a listed admin, type `#tbd` in chat.

   Expected: the client loads into the instance's lobby, and `#tbd` answers with the
   [missions](/documentation_v2/glossary/g_to_m.md#mission) the platform lets this server deploy,
   numbered from 1; a player not in `game.admins[]` gets "TBD: admin only.".

3. When the join fails, probe the join path from a development machine on the same network before
   the attempt. The argument is a run id written into the report.

   ```bash
   cargo xtask debug direct-join before-join --instance <N>
   ```

   Expected: `Wrote debug log: <checkout>/.cursor/debug-8fc1e0.log`, then a `--- summary ---`
   with the Steam build ids of the client and dedicated server installed on this machine, the
   client addon link, the ping and A2S answers of the host's IPv4 address, and the host's unit
   state, UDP listeners and the last listen, A2S and client lines of the instance's newest log.
   Without `TBD_SSH_HOST` the host's probes read `skipped` and one line on stderr names the
   settings file; an instance outside the fleet exits 1 before any probe.
   On the fleet host add `--instance N` (1 to 5): the probes then read `tbd-reforger@N.service`,
   game port 2000 + N and A2S port 17776 + N, the ports the log's H1 row names (`udp_2003` and
   `udp_17779` for instance 3); fleet settings that `cargo xtask deploy staging` refuses skip the
   host's probes as a missing host does. Without `--instance` the remote section reads
   `service=fleet_host` and names `--instance`.
   Repeat with `after-join` right after a failed attempt and compare the two blocks.

4. On the client machine, list the join flow of the client's newest log. Under Steam's Proton the
   client log sits in the compatdata folder of the client's Steam app; the command takes the
   newest log of any Arma Reforger client there.

   ```bash
   grep -E 'SEARCHING_SERVER|SERVER_NOT_FOUND|MANUAL_CONNECT|connect' "$(ls -td ~/.local/share/Steam/steamapps/compatdata/*/pfx/drive_c/users/steamuser/Documents/My\ Games/ArmaReforger*/logs/logs_* | head -1)/console.log"
   ```

   Expected: the client's search and connect lines; `SERVER_NOT_FOUND` means it reached no
   registered room.

## Verify

The client stands in the lobby and the instance's log shows it:

```bash
cargo xtask mod remote-logs --instance <N>
```

Expected: once the player has taken a [slot](/documentation_v2/glossary/n_to_z.md#slot), `VERDICT: PASS — boot healthy and at least one player
was seated.`, exit 0.

## Launch flags and ports

The deploy writes the launch line into the template unit `tbd-reforger@.service`
([staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md)); for
instance N it reads:

```text
ArmaReforgerServer -addonsDir <TBD_ADDONS_STAGING> -config ~/tbd/fleet/instance-N/server.config.json -profile ~/tbd/fleet/instance-N/profile -maxFPS 60 -logStats 30000 -nothrow
```

Instance N listens on game port 2000 + N (2001 to 2005), A2S port 17776 + N (17777 to 17781) and
[RCON](/documentation_v2/glossary/n_to_z.md#rcon) port 19998 + N (19999 to 20003, on `127.0.0.1`
only); instance 1 keeps the single server's ports 2001, 17777 and 19999. Instance 1 is `visible`
in the server browser; instances 2 to 5 carry `visible: false`. Every instance's `game.password`
is the join password, so every client is asked for it.

The A2S port is a separate UDP socket from the game port and must differ from it: equal ports log
`Starting RPL server, listening on …:<game port>`, then `NETWORK (E): Unable to start replication`,
`Unable to initialize the game` and `Game destroyed`, and the process exits with status 0, so
`Restart=on-failure` does not restart it; the deploy refuses a port two instances or two roles
share. A server started with `-addons` and `-server` instead of `-config` loads the checkout and
reaches LOBBY but registers no room (no `Server registered with address:`, no `Direct Join Code:`,
no `Loading dedicated server config`) and has no admins; the fleet's unit never passes `-addons`,
which the engine refuses beside `-config`.

Client and server must run the same game version: compare the version string both logs print,
not Steam build ids, which differ between the client's app and the server app, 1890870.

## Getting a script change to the server and the players

Every instance loads the synced checkout through `-addonsDir`, so a script change reaches the
fleet with a redeploy:

1. Compile the mod headless; exit 3 means `apps/mod/tbd-framework/resourceDatabase.rdb` is stale
   and does not register a new `.c` file, which [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
   fixes when it next loads the addon.

   ```bash
   cargo xtask mod compile
   ```

   Expected: exit 0.

2. Redeploy; each instance's boot verdict proves the synced checkout is what loaded.

   ```bash
   cargo xtask deploy staging
   ```

   Expected: `==> deploy complete: 5 instance(s)`, exit 0.

A joining client, though, resolves `game.mods[]` (`B2C3D4E5F6A78901`) from the Workshop, so it can
run a different build of the mod from the server it joins; no check in the repository covers what
a client loads. When a change must reach players, publish `tbd-framework` from Workbench (the
Workshop publish, under the same id as the gproj GUID; set the licence field to a real licence
file). Publishing packs files into the addon folder, which makes Workbench show the addon
read-only; delete them and restart the launcher:

```bash
rm -f apps/mod/tbd-framework/data.pak apps/mod/tbd-framework/meta apps/mod/tbd-framework/ServerData.json apps/mod/tbd-framework/*_manifest.json
```

All four are gitignored. A publish does not update the servers, and it can hide a broken deploy:
with the Workshop copy current, a server that loaded it instead of the checkout looks right in
every log line. The boot verdict's gproj path is the check that tells them apart.

To load the checkout in a local client instead of the Workshop copy,
`cargo xtask setup client-addons` links `apps/mod/tbd-framework/` into
`~/.local/share/tbd-server-addons/` and prints the Steam launch options
(`-addonsDir "<that folder>" -addons B2C3D4E5F6A78901`); its last line,
`Restart the game, then Direct Join → <host> (<IPv4 address>) port 2001`, names instance 1 on the
host of `TBD_SSH_HOST`, or says to set it when `deploy.env` names none.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Direct Join answers "No server found" | the instance registered no room: a boot that died, or a server started without `-config` | check `Server registered with address:` in the instance's log; `systemctl --user cat tbd-reforger@<N>.service` on the host shows `-config` |
| an instance is missing from the server browser | only instance 1 is listed; instances 2 to 5 carry `visible: false` | Direct Join `<address>:<2000 + N>` or the instance's Direct Join Code |
| the client refuses the password entered | it is not the host's current `~/tbd/fleet/join-password` | ask the operator for it; a changed file reaches every instance with the next deploy |
| the server "passes" but behaves like old code | a Workshop copy won over the checkout | [boot verdict](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md); the unit must carry `-addonsDir` and `-config` |
| `#tbd` answers "TBD: admin only." to every player | `game.admins[]` is empty | set `TBD_ADMIN_IDENTITY_IDS` and redeploy; `passwordAdmin` does not feed that list |
| the join is refused for a version mismatch | client and server game versions differ | update the server with `cargo xtask staging update-game-server`, or the client through Steam |
| `Unknown class TBD_…` on the server | the synced `resourceDatabase.rdb` does not register the class | `cargo xtask mod compile` (exit 3 names a stale rdb), then redeploy |
| "High ping server" warning from a server on Wi-Fi | the host is on Wi-Fi | harmless on the same subnet when `ping` answers |
| no client `console.log` | the client runs under Proton, so its log sits in its app's compatdata prefix | the path in step 4 |
| Workbench shows `tbd-framework` read-only | a publish left `data.pak` and `meta` in the addon folder | the `rm` above, then restart the launcher |

## Related

- [Debug command group](/tools_v2/xtask/src/commands/debug/README.md) — every probe
  `debug direct-join` runs.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — a joinable
  server on a development machine with `cargo xtask mod playtest`.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the Workbench and gate
  cycle before a deploy.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index.
