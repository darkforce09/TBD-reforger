**Status:** live

# Local development

Brings up the whole web platform on a developer machine: Postgres, the website
[API](/documentation_v2/glossary.md#api) on port 8080 and the single-page app on port 3000, signed
in through the [dev login](/documentation_v2/glossary.md#dev-login) or through Discord. Run it on
a fresh checkout or worktree, and after a reboot from step 2 on. The first API and app builds take
several minutes; later starts take seconds. Database work beyond the first seed (backups,
restores, the checksum repair, sample data) is in
[Database operations](/documentation_v2/runbooks/database_operations.md).

## Prerequisites

- The Rust toolchain through rustup. The root `rust-toolchain.toml` pins 1.95.0 with rustfmt,
  clippy and the `wasm32-unknown-unknown` target, and rustup installs it on the first `cargo`
  call in the checkout. Check: `rustc --version` from the repository root prints `1.95.0`.
- Trunk, for the app: `trunk --version`. The gate harness pins 0.21.14
  (`tools_v2/developer-tools/gate-env.json`); Trunk fetches the Tailwind 4.3.2 binary that
  `apps/website/frontend/Trunk.toml` names by itself.
- A container runtime with a compose provider. `cargo xtask db` takes `TBD_CONTAINER_RUNTIME`
  when set, then `podman`, then `docker`, then `distrobox-host-exec podman` or `docker` from
  inside a distrobox container. `podman compose` also needs `podman-compose` or the
  `docker-compose` plugin installed. Check: `podman compose version` or `docker compose version`.
- Git LFS, for the terrain binaries the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator) draws: `git lfs version`.
- Node is not needed. It serves only the Enfusion MCP tools
  ([Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md)).

## Steps

Run every command from the repository root unless a step says otherwise.

### Start the stack

1. Create the API's environment file. Its development values work as they stand:
   `APP_ENV=development`, `DATABASE_URL` on host port 5434, `FRONTEND_URL=http://localhost:3000`
   and a placeholder `JWT_SECRET`. The file is gitignored, so a new git worktree has none; copy
   the main checkout's `.env` into it instead when it holds real Discord values.

   ```bash
   cp apps/website/api_v2/.env.example apps/website/api_v2/.env
   ```

   Expected: no output; `apps/website/api_v2/.env` exists. Every variable, its default and its
   failure mode is in
   [API environment variables](/documentation_v2/website/api_v2/environment_variables.md).

2. Start Postgres 18 in the background.

   ```bash
   cargo xtask db up
   ```

   Expected: `cd apps/website/api_v2 && podman compose up -d db` (with the runtime it found),
   then compose starts the `tbd_reforger_db` container from `postgres:18-alpine`, listening on
   host port 5434 with the user, password and database `tbd`, `tbd` and `tbd_reforger`.

3. Run the API. It builds the `api` binary into `target-dev-api/` in this checkout, applies the
   pending migrations and stays in the foreground; leave it running and open a second terminal.

   ```bash
   cargo xtask mk rust-api
   ```

   Expected: the line
   `cd apps/website/api_v2 && CARGO_TARGET_DIR=<checkout>/target-dev-api cargo run --bin api`,
   the build, then the log lines `migrations applied` and `listening on 0.0.0.0:8080`. Restart it
   after a change to the API; `cargo run` does not reload.

4. Load the development seeds: the Discord role mapping, a small item
   [registry](/documentation_v2/glossary.md#registry), the starter factions, the vehicle database
   and the wiki pages. Run it only after step 3 has logged `migrations applied`: the tables come
   from the migrations.

   ```bash
   cargo xtask db seed
   ```

   Expected: one line per file, from
   `cd apps/website/api_v2 && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/discord_roles.sql`
   through `registry_dev.sql`, `faction_library.sql` and `vehicle_database.sql` to
   `wiki_pages.sql`, each followed by psql's command tags (`INSERT 0 3` for the roles) and no
   `ERROR:` line. psql carries on past a failed statement, so check the output, not only the
   exit code. The seeds upsert, so running them again converges.

5. Serve the app. Trunk builds a release build and stays in the foreground on
   `127.0.0.1:3000`, proxying `/api` and `/map-assets` to the API on `127.0.0.1:8080`, with the
   cross-origin isolation headers the Mission Creator needs. `cargo xtask mk leptos-debug`
   serves a faster unoptimized build instead; do not judge frame rates on it.

   ```bash
   cargo xtask mk leptos
   ```

   Expected: `cd apps/website/frontend && trunk serve --release`, the build, then Trunk serving
   on `http://127.0.0.1:3000`.

6. Sign in without Discord. Open the dev login in the browser; `role` is one of `guest`,
   `enlisted`, `leader`, `mission_maker` and `admin`, and any other value signs in as `admin`.
   The route exists only while `APP_ENV=development`.

   ```text
   http://localhost:3000/api/v1/auth/dev-login?role=admin
   ```

   Expected: a 302 to `http://localhost:3000/auth/callback` with the session in the URL fragment
   (`access_token`, `refresh_token`, `expires_at`, `arma_linked`); the app stores it in local
   storage under `tbd-auth` and loads `/` signed in as "Dev Operator". Each role signs in as its
   own fixed account.

### Call the API from a shell

1. Take an access token from the dev login's redirect, without a browser.

   ```bash
   curl -s -o /dev/null -w '%{redirect_url}\n' \
     'http://127.0.0.1:8080/api/v1/auth/dev-login?role=mission_maker'
   ```

   Expected: `http://localhost:3000/auth/callback#access_token=…&arma_linked=…` and so on; the
   value of `access_token` is the bearer token.

2. Call a route with it, here the item registry, which needs at least the mission maker
   [role](/documentation_v2/glossary.md#role).

   ```bash
   curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:8080/api/v1/registry | jq .
   ```

   Expected: the registry page of the current modpack, with a weak `ETag` header; the same
   request with `If-None-Match` answers 304. `GET /api/v1/registry/compat` takes an
   `?edge_type=` filter. A [mission](/documentation_v2/glossary.md#mission)'s compiled
   [artifact](/documentation_v2/glossary.md#artifact) reads the same way from
   `GET /api/v1/missions/{id}/reviews` and
   `GET /api/v1/missions/{id}/artifacts/{artifact_id}/document`; every route is listed in the
   [API overview](/documentation_v2/website/api_v2/api_overview.md).

### Load the full item registry

The seed holds 21 items and 4 vehicles. The committed Workbench export holds 1,857 items and
20,908 compatibility edges.

1. Import it into the database `DATABASE_URL` names.

   ```bash
   cargo xtask db registry-import
   ```

   Expected: cargo runs the API's `import-registry` binary over
   `contracts_v2/catalogs/registry-items.workbench.json` and
   `contracts_v2/catalogs/registry-compat.workbench.json`, applying pending migrations first.
   The import upserts, so it can run again. The binary itself takes `--items`, `--compat`,
   `--modpack <uuid>` and `--prune`
   (the [API README](/apps/website/api_v2/README.md#public-surface)).

### Fetch the terrain assets

The API serves `assets_v2/terrains/` at `/map-assets` (`MAP_ASSETS_DIR`, default
`../../../assets_v2/terrains` from the API's working directory) and Trunk proxies it to the app.
About 2,000 of its files live in Git LFS (`.gitattributes`): the Everon elevation raster
(`everon/dem/everon-dem-16bit.png`, 72 MB), the satellite container
(`everon/satellite/everon-sat.tbd-sat`, 153 MB), the object chunks (`*.bin`), the prefab
collision hierarchies (`*.bvh`) and the `*.rkyv` archives. The 625 forest-density tiles under
`objects/density/` are ordinary git blobs. A clone without the LFS objects has pointer files
instead: manifests and JSON load, and the elevation, satellite and object layers do not.

1. Pull the elevation raster, which the `website-map-engine` tests and the hillshade need.

   ```bash
   cargo xtask ci lfs-dem
   ```

   Expected: `git lfs pull --include assets_v2/terrains/everon/dem/everon-dem-16bit.png`, then
   the 72 MB file in place of its pointer.

2. Pull the satellite container.

   ```bash
   cargo xtask ci lfs-sat
   ```

   Expected: `git lfs pull --include assets_v2/terrains/everon/satellite/everon-sat.tbd-sat`.
   `git lfs pull` alone fetches every LFS object, the chunks and hierarchies included.

3. Optionally build the tile pyramids, which are gitignored build output under
   `assets_v2/terrains/**/tiles/`. The map basemap needs the cartographic one.

   ```bash
   cargo xtask ci map-cartographic-everon
   ```

   Expected: the cartographic ortho and its pyramid are built, the manifest patched, and
   `map-cartographic-verify` passes. `cargo xtask ci map-water-everon` rebuilds the water
   composite and needs the export scratch in `assets_v2/scratch/`, a sibling of the served tree
   that `/map-assets` cannot reach.

In the app, the satellite layer loads a preview and then the full container; the `?sat=preview`
query parameter stops at the preview, as the gates do. Each terrain's `manifest.json` follows
`contracts_v2/definitions/terrain-manifest.schema.json`, which `cargo xtask ci verify-terrain`
checks with the alignment (`verify-terrain-strict` is the strict form). The layout of the tree is
in the [terrains README](/assets_v2/terrains/README.md).

### Discord OAuth2 — live round-trip

The dev login needs none of this. This procedure proves the real Discord sign-in: the
`oauth_state` cookie, the token exchange and the guild role mapping. It needs a browser and a
person at Discord's consent screen. The request and response of each call are in
[account pages](/documentation_v2/website/frontend/pages/account/account_pages.md).

1. Register the redirect. In the Discord Developer Portal, under the application's OAuth2
   Redirects, add exactly the value of `DISCORD_REDIRECT_URL` in `apps/website/api_v2/.env`:

   ```text
   http://localhost:8080/api/v1/auth/discord/callback
   ```

   Expected: the portal lists it. Discord compares it byte for byte twice, on the authorize URL
   and on the token exchange: scheme, host, port and a trailing slash all count. The API builds
   its own authorize URL with the scopes `identify guilds.members.read` (`OAUTH_SCOPES` in
   `apps/website/api_v2/src/identity_and_access/services/discord_client.rs`), so the portal's URL
   generator and a bot are not involved.

2. Fill the credentials and align the hosts. Set `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET` and
   `DISCORD_GUILD_ID` in `apps/website/api_v2/.env`, and keep `FRONTEND_URL` on the same host as
   `DISCORD_REDIRECT_URL`: the template's `http://localhost:3000` and `http://localhost:8080/…`
   agree. The cookie is host-only (`Path=/; Max-Age=600; HttpOnly; SameSite=Lax`, no `Secure` in
   development), and `localhost` and `127.0.0.1` are different cookie hosts; ports do not matter.
   Change `FRONTEND_URL` rather than the redirect, which must stay equal to the portal entry.

   ```bash
   grep -E '^(FRONTEND_URL|DISCORD_REDIRECT_URL)=' apps/website/api_v2/.env
   ```

   Expected: both values name the same host. In development the API refuses to start the flow
   when they differ: it logs "REFUSING to start the Discord OAuth flow" and sends the browser to
   `/auth/callback#error=oauth_host_mismatch`.

3. Check the credential pair without a browser. The client-credentials grant validates the id and
   secret; the command reads both from `.env` and never puts them on the command line.

   ```bash
   cd apps/website/api_v2 && set -a && . ./.env && set +a && \
     curl -s -o /dev/null -w '%{http_code}\n' -u "$DISCORD_CLIENT_ID:$DISCORD_CLIENT_SECRET" \
     -d grant_type=client_credentials -d scope=identify https://discord.com/api/oauth2/token
   ```

   Expected: `200`. `401` means a wrong id or secret: regenerate the secret in the portal and
   update `.env`.

4. Sign in. With steps 2 to 5 of Start the stack running (restart the API after editing `.env`),
   open the app on the host `FRONTEND_URL` names, not the API port, and click "Sign in with
   Discord".

   ```text
   http://localhost:3000/login
   ```

   Expected: Discord's consent screen, then a 302 to `{FRONTEND_URL}/auth/callback` with the
   session in the fragment, and the top bar showing your Discord name and avatar. A failure
   arrives as `#error=<reason>` and the callback page shows its sentence (troubleshooting
   below). Confirm it in the database, as in
   [Database operations](/documentation_v2/runbooks/database_operations.md), step 1 of Query the
   local database:

   ```sql
   SELECT discord_id, username, role, last_login_at FROM users ORDER BY last_login_at DESC LIMIT 5;
   SELECT guild_id, membership_status, verified_at, last_error FROM discord_membership_snapshots;
   SELECT guild_id, discord_role_id FROM user_discord_roles WHERE discord_id = '<your id>';
   SELECT created_at, action FROM audit_logs WHERE action LIKE 'auth.%' ORDER BY id DESC LIMIT 5;
   ```

   A good sign-in upserts the `users` row, records a `member` snapshot for the guild with its
   `verified_at`, stores your guild role ids and writes an `auth.session_created` audit row.

5. Map the guild's roles. Permissions come from the verified Discord snapshot, never from
   `users.role`: a member takes the mapped role of the highest-priority `discord_roles` row that
   matches one of their guild roles, and `enlisted` when none matches; a non-member, or an
   account whose membership was never verified, is `guest`. `cargo xtask db seed` maps the TBD
   guild's Command Staff (`admin`), Mission Maker (`mission_maker`) and Player (`enlisted`)
   roles; no Squad Leader (`leader`) role id is committed. For another guild, or for `leader`,
   read your role ids from `user_discord_roles` after a sign-in and insert the mapping, as the
   header of `apps/website/api_v2/seeds/discord_roles.sql` shows.

   ```sql
   INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority)
   VALUES ('<role id>', 'Squad Leader', 'leader', 30)
   ON CONFLICT (discord_role_id) DO UPDATE SET name = EXCLUDED.name,
     mapped_role = EXCLUDED.mapped_role, priority = EXCLUDED.priority;
   ```

   Expected: `INSERT 0 1`. The permission check reads the mapping on every request, so the new
   role applies at once. The roster's displayed role follows at the nightly resync
   (`ROLE_RESYNC_INTERVAL_SECS`, 86400 by default), or at once with
   `POST /api/v1/admin/roles/sync` under an administrator's token. An administrator cannot set a
   role by hand: `PATCH /api/v1/admin/users/{discordId}` answers 409, since website roles are
   derived from Discord.

6. Optionally, sign out and in again with a second Discord account that is not in the guild.

   ```text
   http://localhost:3000/login
   ```

   Expected: the sign-in succeeds as `guest`, and the snapshot reads `nonmember`. Discord answers
   404 to the member lookup, and that 404 means "not a member".

### Stop

1. Stop the API and Trunk with Ctrl-C in their terminals, then stop Postgres.

   ```bash
   cargo xtask db down
   ```

   Expected: compose stops and removes the `tbd_reforger_db` container and keeps the data volume,
   so the next `db up` starts on the same data.

## Verify

```bash
curl -sf http://127.0.0.1:8080/healthz
```

Expected: `{"status":"ok"}` with status 200; `curl` exits 22 when the probe answers 503, which it
does while the database is down or `_sqlx_migrations` records a failed migration. `/healthz` and
`/metrics` sit at the root, outside `/api/v1`. Then `http://localhost:3000` shows the app, signed
in after step 6. Before pushing, `cargo xtask ci ci-local` replays the CI suite; it needs step 2
first (the [CI task commands](/tools_v2/xtask/src/commands/ci/README.md) list its steps).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `db up` stops with `FATAL:` and exit 1 | no container runtime resolved from `TBD_CONTAINER_RUNTIME`, podman, docker or `distrobox-host-exec` | install podman or docker, or set `TBD_CONTAINER_RUNTIME` |
| `db up` or `db seed` exits 125 with "looking up compose provider failed" | podman has no compose provider | install `podman-compose` or the `docker-compose` plugin; for an existing container, `podman start tbd_reforger_db` starts it, and [Database operations](/documentation_v2/runbooks/database_operations.md) seeds without compose |
| the API exits with `DATABASE_URL is required` or `JWT_SECRET is required` | no `apps/website/api_v2/.env`, as in a new worktree | step 1 |
| the API exits with `migration N was previously applied but has been modified` | an applied migration file changed, comments included | [Database operations](/documentation_v2/runbooks/database_operations.md), Repair a migration checksum; never reset the volume for this |
| `db seed` prints `relation "discord_roles" does not exist` and exits 0 | the seeds ran before the API migrated the database | step 3, then step 4 again |
| the map shows no elevation or satellite layer | the LFS objects are pointer files | Fetch the terrain assets |
| a cartographic basemap is missing | the tile pyramid is gitignored and not built | Fetch the terrain assets, step 3 |
| Discord shows "Invalid OAuth2 redirect_uri" and never returns to the app | `DISCORD_REDIRECT_URL` is not registered, or differs by a byte | Discord OAuth2, step 1 |
| `#error=oauth_host_mismatch` ("Something went wrong completing sign-in.") | `FRONTEND_URL` and `DISCORD_REDIRECT_URL` name different hosts | Discord OAuth2, step 2 |
| `#error=oauth_unconfigured` | `DISCORD_CLIENT_ID` is blank | fill it, restart the API |
| `#error=invalid_state` | no cookie: more than 10 minutes at the consent screen, cookies blocked, or a host mismatch; a cookie that does not match: a replayed or forged callback | the API log says which; retry from `/login` |
| `#error=discord_unreachable` | the token exchange or the profile read failed | the API log line `discord token_exchange failed [config]` means Discord refused the call (401 `invalid_client`: wrong id or secret; 400 `invalid_grant`: wrong redirect, or a used code); `[outage]` is the network or Discord, so retry; `[protocol]` is an undecodable reply, usually a proxy; Discord OAuth2, step 3 |
| `#error=missing_code` | consent was denied, or the callback was opened by hand | sign in again and approve |
| `#error=banned` | `users.is_banned` is true for the account | lift the ban in the personnel page |
| `#error=server_error` | a database write failed during the callback | check that Postgres is up (step 2) and the API migrated (step 3), then retry |
| signed in as `enlisted` | a guild member whose roles have no mapping | Discord OAuth2, step 5 |
| signed in as `guest` | not a guild member, `DISCORD_GUILD_ID` blank (no membership is read), or a membership last verified over 48 hours ago | join the guild, fill `DISCORD_GUILD_ID`, or sign in again; an administrator can extend cached permissions with `POST /api/v1/admin/users/{discordId}/membership-grace` |

## Related

- [Database operations](/documentation_v2/runbooks/database_operations.md) — psql access, sample
  data, the checksum repair, backups and restores.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — `cargo xtask mk gate-doctor` and
  the browser gates of the Mission Creator.
- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md) — every
  variable `.env` can set.
- [Website API](/apps/website/api_v2/README.md) — the crate, its binaries and its configuration.
- [Database commands](/tools_v2/xtask/src/commands/db/README.md) and
  [build and development-server commands](/tools_v2/xtask/src/commands/build/README.md) — every
  `cargo xtask db` and `cargo xtask mk` command.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the same stack on the
  home server.
