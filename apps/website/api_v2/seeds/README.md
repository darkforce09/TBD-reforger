# Database seeds

Development data for the [API](/documentation_v2/glossary.md#api)'s Postgres database: the five
files `cargo xtask db seed` applies to a fresh local database, the golden content the frontend's
recorded API fixtures come from, and sample data applied by hand.

## Contents

```text
apps/website/api_v2/seeds/
├── content_golden.sql           the pinned content the recorded API fixtures were captured from
├── discord_roles.sql            maps the guild's Discord role ids to web roles
├── faction_library.blufor.json  the BLUFOR faction document of `faction_library.sql`, as plain JSON
├── faction_library.sql          starter BLUFOR and OPFOR factions for the admin dev-login account
├── mock_data.sql                sample users, modpacks and missions, applied by hand
├── registry_dev.sql             a small item registry for the current modpack, vehicles included
├── vehicle_database.sql         the vehicle database rows the golden content also holds
└── wiki_pages.sql               the doctrine wiki pages the golden content also holds
```

## How it works

`cargo xtask db seed` pipes five files, in this order, into `psql -U tbd -d tbd_reforger` inside
the compose `db` container: `discord_roles.sql`, `registry_dev.sql`, `faction_library.sql`,
`vehicle_database.sql`, `wiki_pages.sql`. The order is the `SEEDS` list in
`tools_v2/xtask/src/commands/db/operations.rs`, and the command stops at the first file whose
`psql` exits non-zero. The tables come from the migrations, which only the API applies, so seed
after the API has logged `migrations applied`: `psql` runs without `ON_ERROR_STOP`, so a statement
against a missing table prints an error, the rest of the file still runs, and the command still
exits 0. Each of the five upserts on its key, so running it again converges.

`discord_roles.sql` decides members' [roles](/documentation_v2/glossary.md#role): a member's role
is the mapped role of their highest-priority matching Discord role, and `enlisted` when none
matches. Its role ids are the TBD guild's, so another guild replaces them. `registry_dev.sql`
upserts the current modpack itself, so it needs no other seed. `faction_library.sql` owns its
factions by the Discord id the admin [dev login](/documentation_v2/glossary.md#dev-login) signs
in as, so they show once that account exists.

`content_golden.sql` pins every id and timestamp, so capturing the fixtures in
`apps/website/frontend/tests/fixtures/api/` again reproduces them byte for byte; its closing
comment holds the capture recipe, which applies it by hand after `registry_dev.sql`. Its sections
follow foreign-key order, because `psql` runs each statement on its own. `mock_data.sql` is applied
by hand with `psql`; nothing runs it.

## Format

- Encoding: UTF-8 Postgres SQL, one topic per file, named in snake_case after what it fills;
  `faction_library.blufor.json` is UTF-8 JSON. The `--` comments of the SQL files follow the
  crate's prose rules, which `apps/website/api_v2/src/tests/prose_rules.rs` checks.
- Schema: the tables the migrations in `apps/website/api_v2/migrations/` create. The OPFOR
  document of `faction_library.sql` is the faction library sample,
  `contracts_v2/fixtures/registry/faction-library.sample.json`, and the BLUFOR one holds the same
  JSON as `faction_library.blufor.json`.
- Adding a file: write idempotent statements (`ON CONFLICT … DO UPDATE` or `DO NOTHING`) that name
  only parents inserted earlier; for `cargo xtask db seed` to apply it, add it to `SEEDS` in
  dependency order, then run `cargo xtask db seed` against a database the API has booted on and
  check the output for errors.

## Producers and consumers

- Producers: developers, by hand. `vehicle_database.sql` and `wiki_pages.sql` repeat the matching
  rows of `content_golden.sql`, so a change to one changes the other.
- Consumers:
  - `cargo xtask db seed`, through `SEEDS` in `tools_v2/xtask/src/commands/db/operations.rs`;
  - `cargo xtask verify wiki-seeds` and `cargo xtask verify faction-library-seeds`
    (`tools_v2/xtask/src/verifications/database/`), which check that `SEEDS` lists the wiki and
    faction seeds and that the files hold the `field-manual` page and the `US Army 1980s` faction;
  - `apps/website/api_v2/tests/leaderboards_paging.rs`, which embeds `content_golden.sql` and
    applies it to its own scratch database;
  - the migration step of `cargo xtask platform wave gate`
    (`tools_v2/xtask/src/commands/platform/wave_execution/migrate.rs`), which applies
    `content_golden.sql` after each run so its database stays populated;
  - people recapturing the frontend's API fixtures, and people applying `mock_data.sql`.

## Boundaries

- Depends on: the schema of `apps/website/api_v2/migrations/`; the dev-login account ids in
  `apps/website/api_v2/src/identity_and_access/handlers/developer_login.rs`; the TBD guild's
  Discord role ids.
- Used by: the consumers above; the recorded fixtures in `apps/website/frontend/tests/fixtures/api/`
  depend on `content_golden.sql` staying as it is.
- Rules: a migration that makes `content_golden.sql` fail to load breaks every fresh environment
  and the wave gate, so the two change together; the ids and timestamps of `content_golden.sql`
  stay pinned, never `now()` or generated; `SEEDS` keeps listing the wiki and faction seeds
  (`cargo xtask verify wiki-seeds`, `cargo xtask verify faction-library-seeds`).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — seeding a local database
  and mapping the guild's roles.
- [Database commands](/tools_v2/xtask/src/commands/db/README.md) — `cargo xtask db` and its seed
  step.
