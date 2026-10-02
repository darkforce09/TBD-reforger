# Database seeds

Development data for the [API](/documentation/glossary/a_to_f.md#api)'s Postgres database: the five
files `cargo xtask db seed` applies to a fresh local database, the golden content the frontend's
recorded API fixtures come from, and sample data applied by hand.

## Contents

```text
apps/website/api_v2/seeds/
├── content_golden.sql           the pinned content every recorded API fixture is captured from
├── discord_roles.sql            maps the guild's Discord role ids to web roles
├── faction_library.blufor.json  the BLUFOR faction document of `faction_library.sql`, as plain JSON
├── faction_library.sql          starter BLUFOR and OPFOR factions for the admin dev-login account
├── mock_data.sql                sample users, modpacks and missions, applied by hand
├── registry_dev.sql             the current modpack's item registry, vehicles included, ids pinned
├── vehicle_database.sql         the vehicle database rows the golden content also holds
└── wiki_pages.sql               the golden wiki pages and the wiki formatting guide, at revision 1
```

## How it works

`cargo xtask db seed` pipes five files, in this order, into
`psql -v ON_ERROR_STOP=1 -U tbd -d tbd_reforger` inside the compose `db` container:
`discord_roles.sql`, `registry_dev.sql`, `faction_library.sql`, `vehicle_database.sql`,
`wiki_pages.sql`. The order is the `SEEDS` list in `tools/xtask/src/commands/db/operations.rs`.
Each `psql` run stops at its first failed statement and exits 3, and the command stops at that
file with that code. The tables come from the migrations, which only the API applies, so seed
after the API has logged `migrations applied`; seeding earlier fails on the first statement of
`discord_roles.sql`. Each of the five upserts on its key, so running it again converges.

`discord_roles.sql` decides members' [roles](/documentation/glossary/n_to_z.md#role): a member's role
is the mapped role of their highest-priority matching Discord role, and `enlisted` when none
matches. Its role ids are the TBD guild's, so another guild replaces them. `registry_dev.sql`
upserts the current modpack itself, so it needs no other seed, and pins every item's id, so the
recorded `GET__registry.json` fixture reproduces on a fresh database. `faction_library.sql` owns its
factions by the Discord id the admin [dev login](/documentation/glossary/a_to_f.md#dev-login) signs
in as, so they show once that account exists. `wiki_pages.sql` also writes revision 1 of each of
its pages into `wiki_page_revisions`; running it again refreshes a page and its revision 1 only
while the page is still at revision 1, so a page saved through the wiki keeps its content and its
history. Its `wiki-formatting-guide` page is the authors' reference: it shows every heading level,
link form, image, aligned table, checklist, callout kind, quote, code block and rule the wiki
renders.

`content_golden.sql` pins every id and timestamp, so capturing the fixtures in
`apps/website/frontend/tests/fixtures/api/` again reproduces every one of them, the audit log and
its event stream included. Its closing comment holds the capture recipe: boot an API of its own on
a fresh database, take a dev-login token, apply `registry_dev.sql` and then this file, and request
each row of the fixtures' `_index.tsv` in order, reads first and writes last, uploading the
committed vanilla ballistics catalog pair (`contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json`
as part `catalog`, `contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json` as part
`calibration`) through `POST /api/v1/ballistics-catalogs` before the first catalog row. A write's JSON body
is the fixture's sibling `<file stem>.request.json`, and each captured body is written key-sorted
and two-space indented. Its sections follow foreign-key order, because `psql` runs each statement
on its own, except its audit section, which runs first: it owns audit ids 1 to 10, replacing the
lines a fresh database holds there (the migrations' own and the capture login's), so the event
audit trigger's lines follow as ids 11 to 15, and it pins their creation times to their
operations'. Its upcoming operations run from 2030-08 to 2031-02, so registration, the waitlist
promotion and the upcoming lists see them as upcoming. Its wiki pages are the five of
`wiki_pages.sql`, each with its revision 1 in `wiki_page_revisions`, so every page has a revision
row for its current revision. Beside the members, events, missions and matches the fixtures read,
it holds two server status rows: an active server that reports a telemetry queue reading (three
entries, the oldest 12 s old, none dropped) and an inactive one that never reported a queue, so
the fleet reads leave it out. It also holds the detailed events of one match, one of each of the
seven kinds at sequences 1 to 7, with each event's canonical-JSON `payload_sha256`, and the
`match_event_totals` rows and `matches.event_count` the ingest would have written for them.
Its fire mission section holds two saved fire missions on the golden operation: one saved before
the solution columns existed, with a null sight setting, charge and time of flight, and one with
the whole solution the solver answers for its inputs. Its last section holds what the writes that
answer server-generated values need: a development session whose fixture refresh token
`POST /auth/refresh` spends (a production API refuses to rotate a development session), and, on
the secondary server, a deployment in flight with its queued command and a live credential to
revoke. The fixtures' writes follow the reads and run in index order; its event access writes each
name the access revision the write before them left. Where a write answers an id, a secret or a
time the server generates per request, its fixture holds the placeholder the normalisation table
of `apps/website/api_v2/tests/contract_parity_support/normalised_fields.rs` names, never a live
value.
`mock_data.sql` is applied by hand with `psql`; nothing runs it.

## Format

- Encoding: UTF-8 Postgres SQL, one topic per file, named in snake_case after what it fills;
  `faction_library.blufor.json` is UTF-8 JSON. The `--` comments of the SQL files follow the
  crate's prose rules, which `apps/website/api_v2/src/tests/prose_rules.rs` checks.
- Schema: the tables the migrations in `apps/website/api_v2/migrations/` create. The OPFOR
  document of `faction_library.sql` is the faction library sample,
  `contracts/fixtures/registry/faction-library.sample.json`, and the BLUFOR one holds the same
  JSON as `faction_library.blufor.json`.
- Adding a file: write idempotent statements (`ON CONFLICT … DO UPDATE` or `DO NOTHING`) that name
  only parents inserted earlier; for `cargo xtask db seed` to apply it, add it to `SEEDS` in
  dependency order, then run `cargo xtask db seed` against a database the API has booted on and
  check the output for errors.

## Producers and consumers

- Producers: developers, by hand. `vehicle_database.sql` and `wiki_pages.sql`, the
  `wiki-formatting-guide` page and the revision rows included, repeat the matching rows of
  `content_golden.sql`, so a change to one changes the other.
- Consumers:
  - `cargo xtask db seed`, through `SEEDS` in `tools/xtask/src/commands/db/operations.rs`;
  - `cargo xtask verify wiki-seeds` and `cargo xtask verify faction-library-seeds`
    (`tools/xtask/src/verifications/database/`), which check that `SEEDS` lists the wiki and
    faction seeds and that the files hold the `field-manual` page and the `US Army 1980s` faction;
  - `apps/website/api_v2/tests/leaderboards_paging.rs`, which embeds `content_golden.sql`,
    applies it to its own scratch database and adds thirty tied players on top;
  - `apps/website/api_v2/src/community_content/services/wiki_markup/tests/wiki_markup.rs`, which
    reads the `wiki-formatting-guide` body out of `wiki_pages.sql` and checks it saves with no
    finding and shows every construct;
  - the migration step of `cargo xtask platform wave gate`
    (`tools/xtask/src/commands/platform/wave_execution/migrate.rs`), which applies
    `content_golden.sql` after each run so its database stays populated;
  - the capture recipe that closes `content_golden.sql`, which applies `registry_dev.sql` and then
    `content_golden.sql` to rebuild the frontend's API fixtures, and people applying
    `mock_data.sql`.

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

- [Local development](/documentation/runbooks/local_development.md) — seeding a local database
  and mapping the guild's roles.
- [Database commands](/tools/xtask/src/commands/db/README.md) — `cargo xtask db` and its seed
  step.
