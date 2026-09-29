**Status:** live

# Prepare the staging host

Prepares an Ubuntu 24.04 host once to run the staging game server fleet and the website beside it:
the host's name on the network, the deploy settings on the development machine, the host's
folders, the build tools, the Arma Reforger experimental dedicated server, the platform API the
[mod](/documentation_v2/glossary/g_to_m.md#mod) talks to, and the firewall for the fleet's ports.
Run the steps in order; after them the fleet needs its credentials and join password
([machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md))
before the first [staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md).

## Prerequisites

- On the development machine: a checkout, the Rust toolchain, `ssh` and `rsync`, and `sshpass`
  when the host takes a password rather than a key; mDNS name resolution (`nss-mdns`), so
  `<host>.local` resolves.
- On the host: Ubuntu 24.04; an SSH account that owns the `tbd/` folder the deploy writes to, with
  `sudo` for the package, linger and firewall steps; Docker with the compose plugin, started at
  boot (`systemctl is-enabled docker` prints `enabled`) and usable by the deploy account without
  `sudo` (`groups` lists `docker`); `steamcmd`; `curl` and `sha256sum` for the deploy's
  game-runtime smoke.
- Access to the home router's DHCP settings.
- A Steam account that may download the experimental dedicated server.

The host is written `<host>` below: its name, which it announces on the network as
`<host>.local`. `<LAN interface>` is the host's network interface on the home network and `<LAN>`
that network's address with its last byte 0; `ip -4 route` on the host prints both, as
`<LAN>/24 dev <LAN interface> …`.

## Steps

1. On the host, install avahi, which answers for `<host>.local` on the home network.

   ```bash
   sudo apt install avahi-daemon
   ```

   Expected: apt installs `avahi-daemon`, or reports it is already the newest version; the
   service runs (`systemctl is-active avahi-daemon` prints `active`).

2. On the host, limit avahi to the home network's interface. Without it, avahi also announces the
   host's addresses on Docker's bridges, and `<host>.local` resolves to a `172.x` address the
   development machine cannot reach.

   ```bash
   sudo sed -i 's/^#\?allow-interfaces=.*/allow-interfaces=<LAN interface>/' /etc/avahi/avahi-daemon.conf
   ```

   Expected: no output; `grep '^allow-interfaces' /etc/avahi/avahi-daemon.conf` prints
   `allow-interfaces=<LAN interface>`.

3. On the host, restart avahi on the new setting.

   ```bash
   sudo systemctl restart avahi-daemon
   ```

   Expected: no output.

4. On the development machine, resolve the host's name.

   ```bash
   getent ahostsv4 <host>.local
   ```

   Expected: lines that all name the host's home-network address, one per socket type; no `172.x`
   address.

5. Reserve that address for the host in the home router's DHCP settings. No command: the router's
   web page lists the host's lease, and a reservation keeps it. The deploy writes the address it
   resolves into every instance's server config as the server's public address, so the address
   must not change between deploys.

   Expected: the router lists a reservation for the host at its current address.

6. Create the deploy settings file from its template. The file is gitignored and the deploy's
   rsync excludes it, so it never leaves the development machine.

   ```bash
   cp tools_v2/xtask/deploy/deploy.env.example tools_v2/xtask/deploy/deploy.env
   ```

   Expected: no output. Set `TBD_SSH_HOST` to `<user>@<host>.local`, then `TBD_SSH_PASS` or
   `TBD_SSH_IDENTITY_FILE` (neither means plain `ssh` with the agent's keys). The host paths
   default under the deploy account's home: `~/tbd/repo`, `~/tbd/addons-staging` and
   `~/steam/arma-reforger-server` (`TBD_REMOTE_DIR`, `TBD_ADDONS_STAGING`, `TBD_SERVER_DIR`), and
   `~/tbd/profile` (`TBD_PROFILE_DIR`), the single server's profile that the first fleet deploy
   archives; set them only when the host keeps them elsewhere. The `TBD_FLEET_*` settings keep
   the template's five instances. The machine credentials, RCON passwords and join password are
   files on the host, never settings: the deploy refuses the settings that would hold them. The
   other settings come later; the
   [staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md) lists them
   all.

7. Discover the host and create its folders. The command reads `TBD_SSH_HOST`, the three `tbd/`
   paths and the SSH settings from the environment, with `deploy.env` winning where it sets them;
   an unset path takes the template's default.

   ```bash
   cargo xtask mod bootstrap-staging
   ```

   Expected: `==> Discovery on <host>`, then the host's `--- disk ---`, the TCP listeners on 5432,
   8080 and 2001 under `--- ports 5432 8080 2001 ---` (or `(none listening on those TCP ports)`),
   and the Docker version under `--- docker ---`; then
   `==> Create TBD directories (not prairielearn)`, `OK: <repo> <profile> <addons-staging>`, and a
   "Next steps" list whose first step names Steam app 1890870 (step 11 here). Its credential
   step is what `cargo xtask staging provision-fleet` does
   ([machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)).

8. On the host, install the C toolchain the Rust builds link with. Both deploys build on the
   host: the website deploy the API, the app and the staging host tools, the staging deploy
   `xtask`, the host agent and the relay.

   ```bash
   sudo apt install build-essential
   ```

   Expected: apt installs `build-essential`, or reports it is already the newest version.

9. On the host, install rustup into `~/.cargo` for the deploy account. The deploys put only
   `$HOME/.cargo/bin` on `PATH`, and the checkout's `rust-toolchain.toml` makes rustup install the
   pinned toolchain and its `wasm32-unknown-unknown` target on the first build.

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   ```

   Expected: rustup ends with `Rust is installed now. Great!`; `~/.cargo/bin/cargo --version`
   prints a version.

10. On the host, install Trunk, which builds the website's app.

    ```bash
    ~/.cargo/bin/cargo install --locked trunk
    ```

    Expected: cargo ends with an `Installed package` line for `trunk`; `~/.cargo/bin/trunk --version`
    prints a version.

11. On the host, install Steam app 1890870, the Arma Reforger experimental dedicated server, into
    `TBD_SERVER_DIR`, replacing `<steam user>`. Every instance runs this one install.

    ```bash
    steamcmd +login <steam user> +force_install_dir "$HOME/steam/arma-reforger-server" +app_update 1890870 validate +quit
    ```

    Expected: steamcmd reports the app fully installed, and `ArmaReforgerServer` sits in the
    install folder. Later updates run the same `validate` through
    `cargo xtask staging update-game-server`. The server's game version must match the clients'
    (see [client join](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md));
    the "Server build" that `debug direct-join` prints is the build of server app 1874900 on the
    development machine (`tools_v2/xtask/src/commands/debug/direct_join.rs`), not the host's.

12. Deploy the platform API to the same host. The staging deploy starts nothing of the website:
    before it changes anything on the host it checks that the API answers `/healthz` at
    `TBD_BACKEND_URL` (default `http://127.0.0.1:8080`), and stops when it does not. The
    [website deployment](/documentation_v2/runbooks/website_deployment.md) runbook covers the API's
    `.env` on the host, the `tbd-website-api` unit, the Postgres and Caddy containers the website
    deploy starts, and the host tools `staging-fixtures` and `acknowledgement-dropping-relay` it
    builds into the checkout's `target/release/`; its dry run prints the plan first.

    ```bash
    cargo xtask deploy website --dry-run
    ```

    Expected: the deploy plan, with no command sent to the host. The host's
    `apps/website/api_v2/.env`, which both deploys' rsync excludes, needs `JWT_SECRET` and
    `OBSERVABILITY_TOKEN`; the mod authenticates every call with its machine credential.

13. On the host, let the deploy account's user services run while nobody is logged in; the game
    servers, the host agents and the relay are user units.

    ```bash
    sudo loginctl enable-linger "$USER"
    ```

    Expected: no output; `loginctl show-user "$USER"` reports `Linger=yes`.

14. On the host, see whether the firewall is on. Ubuntu installs ufw, but may leave it off.

    ```bash
    sudo ufw status
    ```

    Expected: `Status: active` and its rules, or `Status: inactive`. The rules the next steps add
    are kept either way, and take effect once ufw is enabled.

15. On the host, open the five instances' game ports for UDP and TCP and their A2S query ports
    for UDP: instance N's game port is 2000 + N and its A2S port 17776 + N. With other
    `TBD_FLEET_*` values in `deploy.env`, open those ports instead.

    ```bash
    sudo ufw allow 2001:2005/udp && sudo ufw allow 2001:2005/tcp && sudo ufw allow 17777:17781/udp
    ```

    Expected: `Rule added` and `Rule added (v6)` for each rule (`Rules updated` while ufw is
    inactive). RCON (19999 to 20003) and the relay (18085) listen on `127.0.0.1` only, so they get
    no rule.

16. On the host, let the home network reach avahi, so `<host>.local` keeps resolving with the
    firewall on.

    ```bash
    sudo ufw allow from <LAN>/24 to any port 5353 proto udp
    ```

    Expected: `Rule added` (`Rules updated` while ufw is inactive).

17. Only when step 14 printed `Status: inactive` and the firewall should run: allow SSH first, so
    enabling ufw does not cut the session, then enable it.

    ```bash
    sudo ufw allow OpenSSH && sudo ufw enable
    ```

    Expected: `Rules updated`, `Rules updated (v6)`, the question
    `Command may disrupt existing ssh connections. Proceed with operation (y|n)?`, answered `y`,
    then `Firewall is active and enabled on system startup`; `sudo ufw status` now lists every rule
    of steps 15 to 17.

18. Only on a host whose home-network interface is Wi-Fi: install a unit that turns the card's
    power saving off each time the interface appears, because a Wi-Fi card saving power delays
    and drops the game's UDP traffic. `<wifi interface>` is the name `iw dev` lists.

    ```bash
    sudo tee /etc/systemd/system/wifi-power-save-off.service > /dev/null <<'EOF'
    [Unit]
    Description=Wi-Fi power saving off on <wifi interface>
    BindsTo=sys-subsystem-net-devices-<wifi interface>.device
    After=sys-subsystem-net-devices-<wifi interface>.device

    [Service]
    Type=oneshot
    ExecStart=/usr/sbin/iw dev <wifi interface> set power_save off

    [Install]
    WantedBy=sys-subsystem-net-devices-<wifi interface>.device
    EOF
    ```

    Expected: no output.

19. Only on a Wi-Fi host: enable the unit and run it now.

    ```bash
    sudo systemctl enable --now wifi-power-save-off.service
    ```

    Expected: `Created symlink …/sys-subsystem-net-devices-<wifi interface>.device.wants/wifi-power-save-off.service → /etc/systemd/system/wifi-power-save-off.service`;
    `iw dev <wifi interface> get power_save` prints `Power save: off`.

## The fleet folder

Everything instance-specific lives in the deploy account's `~/tbd/fleet/`.
`cargo xtask staging provision-fleet` creates it with the credentials, the operator adds the join
password, and `cargo xtask deploy staging` writes the rest. Folders are mode 700 and files mode
600, and the deploy refuses a secret file that group or others may read.

```text
~/tbd/fleet/
├── join-password                  the join password every instance requires, written by the operator
└── instance-1/ … instance-5/      one folder per instance, 1 to TBD_FLEET_INSTANCES
    ├── server.config.json         the instance's -config, both passwords filled in on the host
    ├── profile/                   its -profile: profile/TBD_BackendConfig.json, logs/logs_<time>/console.log
    ├── relay.env                  the relay instance only: RELAY_LISTEN and RELAY_UPSTREAM, no secret
    └── secrets/
        ├── mod-runtime-credential the mod's machine credential, from provision-fleet
        ├── host-agent-credential  the host agent's machine credential, from provision-fleet
        └── rcon-password          generated once on the host by the deploy
```

Beside it, the deploy writes each host agent's `~/.config/fleet-host-agent/instance-N/agent.toml`,
and `--migrate-single-instance` moves the single server's files into
`~/tbd/retired/single-instance-<UTC time>/`.

- **The join password** is `game.password` of every instance: players type it when they join. It
  is 3 to 64 of `A-Z a-z 0-9 . _ ~ + = : @ % -`, characters that need no escaping in JSON or in the
  game's password box. The operator writes it by hand on the host once `provision-fleet` has made
  the folder
  ([machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
  step 2) and hands it to the players; no document, setting or printed line holds it. Every
  deploy reads it on the host and fills it into each instance's config there.
- **RCON** listens on `127.0.0.1` only. Each instance's `rcon` block grants the `admin`
  permission, which the console command needs, to the 32-hex-digit password the deploy generated
  on the host, and only the instance's host agent logs in with it.

## Verify

From the development machine, the host's name resolves to its home-network address alone, and
the API answers its health probe on the host:

```bash
getent ahostsv4 <host>.local && ssh <TBD_SSH_HOST> curl -sf http://127.0.0.1:8080/healthz
```

Expected: the host's reserved address on every line `getent` prints, then `{"status":"ok"}`. On a
Wi-Fi host,
`iw dev <wifi interface> get power_save` still prints `Power save: off` after a reboot. Then go on
to
[machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `getent ahostsv4 <host>.local` prints nothing | avahi is not running on the host, the firewall drops mDNS, or the development machine has no mDNS resolver | steps 1 to 3 and 16; on the development machine, `nss-mdns` with `mdns4_minimal` in the `hosts:` line of `/etc/nsswitch.conf` |
| `<host>.local` resolves to a `172.x` address | avahi announces the Docker bridges' addresses too | steps 2 and 3 |
| the servers' public address is stale after a router restart | the host took a new DHCP lease | step 5, then deploy staging again |
| `TBD_SSH_HOST is not set: add it to <path>`, exit 1 | no host in the deploy file or the environment | step 6 |
| `could not read …deploy.env: …`, exit 1 | the deploy file exists but cannot be read | fix its permissions; an unreadable file is never treated as empty |
| `Refusing: TBD_REMOTE_DIR must not be under prairielearn/`, exit 1 | the remote path names the neighbouring project's tree | point `TBD_REMOTE_DIR` at the deploy account's `tbd/repo` |
| `ssh: command not found` or `sshpass: command not found`, exit 127 | the tool is missing on the development machine (`sshpass` only with `TBD_SSH_PASS`) | install it, or use `TBD_SSH_IDENTITY_FILE` instead of a password |
| the discovery shows 5432 already taken | another Postgres holds the port | set `TBD_POSTGRES_HOST_PORT` in `deploy.env` to a free port; `deploy website` passes it to the staging compose file |
| `linker 'cc' not found` during a deploy's build | the C toolchain is missing on the host | step 8 |
| `trunk: command not found` during the website deploy | Trunk is not in `~/.cargo/bin` | step 10 |
| `curl` on `/healthz` exits 7 | nothing listens on 127.0.0.1:8080 | step 12 |
| the game servers stop when the SSH session ends | linger is off, so user units stop at logout | step 13 |
| clients reach instance 1 but not another instance | the firewall opens only some of the fleet's ports | step 15, with every instance's game and A2S port |
| SSH stops answering once ufw is enabled | ufw was enabled before SSH was allowed | on the host's console, `sudo ufw allow OpenSSH` |
| `iw dev <wifi interface> get power_save` prints `Power save: on` again after a reconnect | NetworkManager manages the Wi-Fi and applies its own `wifi.powersave` setting on each connection, after the unit ran | set `wifi.powersave = 2` under `[connection]` in a file in `/etc/NetworkManager/conf.d/`, then `sudo systemctl restart NetworkManager` |

## Related

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index and
  the facts every step relies on.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the API, Postgres,
  Caddy and the host tools on the same host.
- [Setup command group](/tools_v2/xtask/src/commands/setup/README.md) — what
  `mod bootstrap-staging` and `setup server-profile` do.
- [Deploy files](/tools_v2/xtask/deploy/README.md) — `deploy.env.example`, the Caddyfile and the
  systemd units.
