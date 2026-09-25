**Status:** live

# Prepare the stack and the mission for a playtest

Proves the mod compiles and boots, starts the website [API](/documentation_v2/glossary.md#api)
and single-page app on the development machine, and authors the
[mission](/documentation_v2/glossary.md#mission) the session plays: one whose
[slots](/documentation_v2/glossary.md#slot) carry loadouts authored in the
[Arsenal](/documentation_v2/glossary.md#arsenal). An optional last part attaches the mission to an
[event](/documentation_v2/glossary.md#event). Run it alone, from the repository root, the day
before the session; it takes about 30 minutes.

## Prerequisites

- The host shell, not an agent container (see the [index](/documentation_v2/runbooks/two_client_playtest/README.md)).
- The Linux dedicated server installed at
  `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`, which every mod gate runs.
  Which Steam app installs it is an open decision: the gates' install hints name app `1890870`,
  while `cargo xtask debug direct-join` reads the server build from app `1874900`'s manifest
  ([Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)).
- A container runtime for Postgres and the Rust toolchain
  ([local development](/documentation_v2/runbooks/local_development.md)).
- `apps/website/api_v2/.env`, copied from `.env.example`.
- Two Arma Reforger clients on the same game version as the server, each able to load
  `tbd-framework`.

## Steps

1. Compile the mod's scripts headless.

   ```bash
   cargo xtask mod compile
   ```

   Expected: exit 0. A non-zero exit names a `file:line` in a `.c` file, and exit 3 means
   `apps/mod/tbd-framework/resourceDatabase.rdb` is stale; do not go on.

2. Boot the world headless with no mission and read the component roll-call.

   ```bash
   cargo xtask mod world-boot
   ```

   Expected: the last line is `WORLD BOOT: PASS`, and the log carries
   `[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok`,
   printed by `TBD_FrameworkManager.PrintComponentRollCall`. A `=MISSING` entry is a component of
   `Prefabs/Systems/TBD_GameMode.et` whose class did not resolve; nothing else reports it.

3. Check the tree holds only the changes you mean to test.

   ```bash
   git -c filter.lfs.process= status --short
   ```

   Expected: only files you expect. A dirty `apps/mod/` tree means the session tests something
   other than `main`.

4. Start Postgres on host port 5434.

   ```bash
   cargo xtask db up
   ```

   Expected: compose starts the `tbd_reforger_db` container.

5. In a second terminal, run the API on port 8080; it applies the migrations at boot and stays in
   the foreground.

   ```bash
   cargo xtask mk rust-api
   ```

   Expected: the API logs `migrations applied` and `listening on 0.0.0.0:8080`.

6. In a third terminal, run the single-page app on port 3000; Trunk proxies `/api` to the API.

   ```bash
   cargo xtask mk leptos
   ```

   Expected: Trunk serves `http://127.0.0.1:3000`.

7. Load the development seeds, which carry the item catalogue the Arsenal reads; the step is safe
   to repeat.

   ```bash
   cargo xtask db seed
   ```

   Expected: one line per seed file and no `ERROR:` line.

8. Check the service token the mod sends with an identity link.

   ```bash
   grep '^SERVICE_TOKEN=' apps/website/api_v2/.env
   ```

   Expected: one non-empty value. `cargo xtask mod playtest` reads it from this file (or
   `--token`) and writes it into the server's backend config as `serverToken`.

9. Log in to the app without Discord: open `http://127.0.0.1:3000`, then open
   `http://127.0.0.1:8080/api/v1/auth/dev-login?role=admin` in the same browser.

   Expected: the API mints a session and redirects to the app's callback; you land logged in as an
   administrator.

10. Author the mission in the [Mission Creator](/documentation_v2/glossary.md#mission-creator).
    Open `http://127.0.0.1:3000/missions`, press "New Mission", choose Everon, and in
    `/missions/:id/edit` place at least four character slots across two factions and two squads.
    Open the Arsenal on a slot (the Attributes dialog's Arsenal tab, or "OPEN ARSENAL" in the ORBAT
    Manager's slot inspector) and author, on different slots:

    - slot A: primary weapon, optic, magazine, uniform, vest, helmet, trousers, boots and
      backpack, plus cargo rows into `vest` and `backpack`;
    - slot B: gear only, no cargo;
    - slot C: cargo only, no gear;
    - slot D: nothing, the bare kit.

    Expected: the same coverage as `contracts_v2/fixtures/missions/valid/slot-loadout-coverage.json`,
    which shows every field's shape.

11. Press "Save Version", then "Submit for review" in the mission library's dossier.

    Expected: submission compiles the saved version into an immutable
    [artifact](/documentation_v2/glossary.md#artifact). A refusal names its code:
    `NO_PLACED_SLOTS`, `UNCOMPILABLE_VERSION`, `DOCUMENT_CONTRACT_VIOLATION`, or
    `UNSUPPORTED_AUTHORED_DATA` with every authored path the compiler cannot carry (for example a
    kit prefab missing from `contracts_v2/rules/kit-aliases.json`).

12. Copy the mission id out of the address bar and take an administrator's access token from the
    dev login's redirect.

    ```bash
    curl -s -D - -o /dev/null 'http://127.0.0.1:8080/api/v1/auth/dev-login?role=admin' | grep -i '^location'
    ```

    Expected: a `Location` URL whose fragment carries `access_token=<token>`. Keep it as `TOKEN`,
    and the mission id as `MID`. Approval is not needed here: `cargo xtask mod playtest --mission`
    submits and approves the current version itself when no approved artifact exists; an
    administrator can also approve it in `/admin/approvals`.

13. Read the newest review's artifact id.

    ```bash
    curl -s -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:8080/api/v1/missions/$MID/reviews"
    ```

    Expected: `{"reviews":[{…,"artifact_id":"<uuid>",…}]}`; keep the first review's `artifact_id` as
    `ART`.

14. Fetch the compiled document the game server will load.

    ```bash
    curl -s -o /tmp/artifact.json -w '%{http_code}\n' -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:8080/api/v1/missions/$MID/artifacts/$ART/document"
    ```

    Expected: `200`, and `/tmp/artifact.json` has `slots[].loadout.gear` on slots A and B and
    `slots[].loadout.cargo` on slots A and C. No gear anywhere means the Arsenal edits never
    reached the saved version: save and submit again. `403` or `404` is a wrong token or id;
    `500` means the stored payload is unreadable or violates `mission.schema.json` (the body names
    the reason) — versions are immutable, so save a new one.

### Optional: an event, for roster seating and deployment authorization

Skip this part for a session without an event: the mod then seats players round-robin and asks
the platform nothing at spawn time. With an event, the platform's roster seats linked players
into their reserved slots and authorizes every deployment into an event seat.

15. In `http://127.0.0.1:3000/admin/events`, create an event and attach the mission; the event's
    ORBAT is built from the mission's slots. Keep the event id as `EID`, and the event mission id
    (the `:emid` of the event's `/events/:id/missions/:emid/orbat` page) as `EMID`.

    Expected: the event page lists the mission and its slots.

16. Bind the event to the server the playtest deploys to. `cargo xtask mod playtest` deploys to the
    server row named "TBD Playtest", creating it on first use, or to `--server=<uuid>`; the
    platform refuses an event mission whose event is bound to another server ("the event mission
    must run this mission on an event bound to this server"). Take the row's id from
    `/admin/server` after a first playtest run, or pass `--server` with a server you choose, and
    keep it as `SID`.

    ```bash
    curl -s -X PATCH -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d "{\"server_id\":\"$SID\"}" "http://127.0.0.1:8080/api/v1/events/$EID"
    ```

    Expected: the updated event, with `server_id` set to `SID`.

## Verify

```bash
curl -s -w '\n%{http_code}\n' http://127.0.0.1:8080/healthz
```

Expected: `{"status":"ok"}` and `200` once the database answers and the migrations are applied
(`apps/website/api_v2/src/core/observability/health_probe.rs`); `unavailable` comes with `503`. Together with step 14's `200`, the
platform is ready to deploy the mission.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `WORLD BOOT: FAIL` with a `=MISSING` roll-call entry | a component class on `TBD_GameMode.et` did not resolve | read `WORLD (E): Unknown class` in the printed log; fix before the session |
| `WORLD BOOT: ENV FAIL — …` | no host bridge, no server binary or no dev profile | run on the host; install the server at the path above |
| `/healthz` answers non-200 or the connection is refused | the API is down; it refuses to boot without `DATABASE_URL` and `JWT_SECRET` | read the `mk rust-api` terminal; check `apps/website/api_v2/.env` |
| the artifact has no `loadout.gear` | the Arsenal edits were not saved into the submitted version | "Save Version", then submit again (step 11) |
| the event binding answers `server_id does not name a known server` | `SID` is not a server row | take the id from `/admin/server` |

## Related

- [Headless preflight](/documentation_v2/runbooks/two_client_playtest/headless_preflight.md) —
  the next runbook: boot this mission's bytes headless.
- [Local development](/documentation_v2/runbooks/local_development.md) — the API and app stack in
  full.
- [Arsenal loadout editor](/documentation_v2/website/frontend/apps/editor/arsenal/arsenal_loadout_editor.md)
  — the Arsenal's fields and rules.
- [Machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
  — the same credential, event binding and deployment steps against the staging server.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
