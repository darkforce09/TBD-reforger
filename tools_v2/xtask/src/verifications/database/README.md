# Database verifications

Source checks that tie the [API](/documentation_v2/glossary.md#api)'s database inputs to the
code that uses them: the development seed list really applies the faction library and wiki seeds,
and the API's SQL never reads `*` from a table with nullable columns. Each is a
`cargo xtask verify` verb, and none needs a running database.

## Contents

```text
tools_v2/xtask/src/verifications/database/
├── faction_library_seeds/    the faction seed entry point, pins, broken variants and evidence output
├── faction_library_seeds.rs  the faction-library-seeds gate: what it pins, and the pinned names and paths
├── mod.rs                    the module tree
├── sql_deserialization.rs    the no-select-star gate over the API sources
├── tests/                    unit tests for the three gates
└── wiki_seeds.rs             the wiki-seeds gate: the seeder applies the wiki seed, which holds its slug
```

## How it works

| Verb | Checks | Exit codes |
|---|---|---|
| `wiki-seeds` | `SEEDS` holds `wiki_pages.sql` (by equality), and `apps/website/api_v2/seeds/wiki_pages.sql` exists, is not empty and holds the `field-manual` slug; the first failing check is reported | 0 pass, 1 any failure, a missing file included |
| `faction-library-seeds` | `apps/website/api_v2/seeds/faction_library.sql` holds a live `INSERT INTO user_factions` naming `'US Army 1980s'` once SQL comments are stripped; `SEEDS` holds `faction_library.sql` (by equality); the [wave](/documentation_v2/glossary.md#wave) gate's `VERIFY_STEPS` holds the `faction-library-seeds` row and both `gate_slice` and `cmd_gate` loop over it | 0 pass, 1 any failure, 2 a broken variant could not be built |
| `no-select-star` | every `.rs` file under `apps/website/api_v2/src`: a `SELECT * FROM <table>` or a `RETURNING *` line fails unless the table (for `RETURNING`, the line) names `modpack_mods` or `orbat_reservations`, the two tables with no nullable column | 0 clean, 1 a match, 2 the source tree could not be read |

`SEEDS` is the constant in `tools_v2/xtask/src/commands/db/operations.rs` that
`cargo xtask db seed` walks; both seed gates read it in process, so an entry is either applied or
absent, and no comment or echoed path can satisfy them.

The faction gate proves its own pins bite on every run: it runs the pin set on the live files,
then on four broken variants that must each fail (the starter name only in a comment, the seed
dropped from the list, a `faction_library.sql.bak` look-alike in its place, the `VERIFY_STEPS`
row deleted), then on the live files again, read afresh from disk.

## Public surface

- `wiki_seeds::verify_wiki_seeds`, `faction_library_seeds::verify_faction_library_seeds` and
  `sql_deserialization::verify_no_select_star`: the three gates, each taking the repository root
  and returning the exit status.

## Boundaries

- Depends on: `verification-core` (`Pattern`, `gate`, `scan`, `Verdict`, `NotRun`); `SEEDS` in
  `tools_v2/xtask/src/commands/db/operations.rs`;
  `tools_v2/xtask/src/verifications/architecture/wave_gate_sources.rs`; the `regex` crate.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify wiki-seeds`,
    `cargo xtask verify faction-library-seeds` and `cargo xtask verify no-select-star`;
  - `tools_v2/xtask/src/commands/ci/task_definitions/verification_dispatch.rs`, whose
    `no-select-star` step is part of `verify-coding-standards` and so of `ci-local`;
  - the wave gate's `VERIFY_STEPS` in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`, which runs `wiki-seeds` and
    `faction-library-seeds`.
- Rules:
  - seed membership is equality on the seed list, never a substring
    (`a_renamed_wiki_entry_does_not_satisfy_the_pin`,
    `the_seeder_must_apply_the_file_not_merely_name_it`);
  - a missing input never reads as a pass (`a_missing_seed_file_does_not_read_as_pass`,
    `a_missing_api_tree_does_not_read_as_clean`);
  - the two seed gates keep a binary exit for everything but the faction gate's variant setup,
    because the wave gate records pass or fail from the status.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — seeding the local
  database with `cargo xtask db seed`.
