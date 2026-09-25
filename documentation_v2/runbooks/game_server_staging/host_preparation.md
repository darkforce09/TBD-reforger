**Status:** live

# Prepare the staging host

Prepares a Linux host once to run the staging game server: the deploy settings on the development
machine, the host's folders, the Arma Reforger dedicated server, the platform API the
[mod](/documentation_v2/glossary.md#mod) talks to, and the firewall. Run the steps in order; after
them a server needs its credentials
([machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md))
before the first [staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md).

## Prerequisites

- On the development machine: a checkout, the Rust toolchain, `ssh` and `rsync`, and `sshpass`
  when the host takes a password rather than a key.
- On the host: an SSH account that owns the `tbd/` folder the deploy writes to, with `sudo` for
  the linger and firewall steps; Docker with the compose plugin; `steamcmd`; the Rust toolchain
  (the deploy runs `cargo run -p xtask` and `cargo build -p fleet-host-agent` there); `curl` and
  `sha256sum` for the deploy's game-runtime smoke.
- A Steam account that may download the dedicated server.

## Steps

1. Create the deploy settings file from its template. The file is gitignored and the deploy's
   rsync excludes it, so it never leaves the development machine.

   ```bash
   cp tools_v2/xtask/deploy/deploy.env.example tools_v2/xtask/deploy/deploy.env
   ```

   Expected: no output. Fill in `TBD_SSH_HOST` (`user@host`), then `TBD_SSH_PASS` or
   `TBD_SSH_IDENTITY_FILE` (neither means plain `ssh` with the agent's keys), and the four host
   paths. The template fills them for the deploy account as `~/tbd/repo`, `~/tbd/profile`,
   `~/tbd/addons-staging` and `~/steam/arma-reforger-server` (`TBD_REMOTE_DIR`, `TBD_PROFILE_DIR`,
   `TBD_ADDONS_STAGING`, `TBD_SERVER_DIR`). The other settings come later; the
   [staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md) lists them all.

2. Discover the host and create its folders. The command reads `TBD_SSH_HOST`, the three `tbd/`
   paths and the SSH settings from the environment, with `deploy.env` winning where it sets them;
   an unset path takes the template's default.

   ```bash
   cargo xtask mod bootstrap-staging
   ```

   Expected: `==> Discovery on <host>`, then the host's `--- disk ---`, the TCP listeners on 5432,
   8080 and 2001 under `--- ports 5432 8080 2001 ---` (or `(none listening on those TCP ports)`),
   and the Docker version under `--- docker ---`; then
   `==> Create TBD directories (not prairielearn)`, `OK: <repo> <profile> <addons-staging>`, and a
   five-line "Next steps" list. Its first line names Steam app 1890870; see step 3.

3. On the host, install the dedicated server into `TBD_SERVER_DIR`, replacing `<steam user>` and
   `<server app id>`.

   ```bash
   steamcmd +login <steam user> +force_install_dir "$HOME/steam/arma-reforger-server" +app_update <server app id> validate +quit
   ```

   Expected: steamcmd reports the app fully installed, and `ArmaReforgerServer` sits in the install
   folder. The repository names two server app ids: `debug direct-join` reads the server build
   from the manifest of app 1874900
   (`tools_v2/xtask/src/commands/debug/direct_join.rs`), while `mod bootstrap-staging` and the
   install hints of `mod compile`, `mod world-boot` and `mod playtest` name app 1890870. Whichever
   you install, the server's game version must match the clients' (see
   [client join](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md)).

4. Deploy the platform API to the same host. The staging deploy checks the game-runtime routes
   against `http://127.0.0.1:8080` on the host and starts only Postgres itself, so the API must
   already run there. The [website deployment](/documentation_v2/runbooks/website_deployment.md)
   runbook covers the API's `.env` on the host and the `tbd-website-api` unit; its dry run prints
   the plan first.

   ```bash
   cargo xtask deploy website --dry-run
   ```

   Expected: the deploy plan, with no command sent to the host. In the host's
   `apps/website/api_v2/.env`, which both deploys' rsync excludes, `SERVICE_TOKEN` must equal the
   `TBD_GAME_SERVER_TOKEN` of `deploy.env`: the mod sends that token for identity-link
   confirmation and match results.

5. On the host, let the deploy account's user services run while nobody is logged in; the game
   server and the host agent are user units.

   ```bash
   sudo loginctl enable-linger "$USER"
   ```

   Expected: no output; `loginctl show-user "$USER"` reports `Linger=yes`.

6. On the host, open the game port for UDP and TCP and the A2S query port for UDP, permanently.

   ```bash
   sudo firewall-cmd --permanent --add-port=2001/tcp --add-port=2001/udp --add-port=17777/udp
   ```

   Expected: `success`.

7. On the host, load the permanent firewall rules.

   ```bash
   sudo firewall-cmd --reload
   ```

   Expected: `success`.

## Verify

From the development machine, the API answers its health probe on the host:

```bash
ssh <TBD_SSH_HOST> curl -sf http://127.0.0.1:8080/healthz
```

Expected: `{"status":"ok"}`. Then go on to
[machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `TBD_SSH_HOST: set TBD_SSH_HOST in the environment or in tools_v2/xtask/deploy/deploy.env`, exit 1 | no host in the environment or the deploy file | step 1 |
| `could not read …deploy.env: …`, exit 1 | the deploy file exists but cannot be read | fix its permissions; an unreadable file is never treated as empty |
| `Refusing: TBD_REMOTE_DIR must not be under prairielearn/`, exit 1 | the remote path names the neighbouring project's tree | point `TBD_REMOTE_DIR` at the deploy account's `tbd/repo` |
| `ssh: command not found` or `sshpass: command not found`, exit 127 | the tool is missing on the development machine (`sshpass` only with `TBD_SSH_PASS`) | install it, or use `TBD_SSH_IDENTITY_FILE` instead of a password |
| the discovery shows 5432 already taken | another Postgres holds the port | free the port: `deploy website` exports `TBD_POSTGRES_HOST_PORT` to the staging compose file, but the staging deploy's compose step passes no port and so maps 5432 |
| `curl` on `/healthz` exits 7 | nothing listens on 127.0.0.1:8080 | step 4 |
| the game server stops when the SSH session ends | linger is off, so user units stop at logout | step 5 |

## Related

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index and
  the facts every step relies on.
- [Setup command group](/tools_v2/xtask/src/commands/setup/README.md) — what
  `mod bootstrap-staging` and `setup server-profile` do.
- [Deploy files](/tools_v2/xtask/deploy/README.md) — `deploy.env.example`, the Caddyfile and the
  systemd units.
