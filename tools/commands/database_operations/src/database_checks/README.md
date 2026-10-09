# Database checks

A source check that ties the [API](/documentation/glossary/a_to_f.md#api)'s database inputs to the
code that uses them: the API's SQL never reads `*` from a table with nullable columns. It is a
`cargo xtask verify` verb, and it needs no running database.

## Contents

```text
tools/commands/database_operations/src/database_checks/
├── sql_deserialization.rs  the no-select-star gate over the API sources
└── tests/                  unit tests for the gate
```

## How it works

| Verb | Checks | Exit codes |
|---|---|---|
| `no-select-star` | every file under `crates/api` (the API crates, the API server among them; one root, so no file is read twice): a `SELECT * FROM <table>` or a `RETURNING *` line fails unless the table (for `RETURNING`, the line) names `modpack_mods` or `orbat_reservations`, the two tables with no nullable column | 0 clean, 1 a match, 2 the source tree could not be read |

## Public surface

- `sql_deserialization::verify_no_select_star`: the gate, taking the repository root and
  returning the exit status.

## Boundaries

- Depends on: `verification_core` (`Pattern`, `scan`, `Verdict`, `NotRun`).
- Used by: `tools/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify no-select-star`.
- Rules:
  - a missing input never reads as a pass (`a_missing_api_tree_does_not_read_as_clean`).

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — the local database lane.
