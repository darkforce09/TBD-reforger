# API audit log

The `api_audit_log` crate: the write side of the [API](/documentation/glossary/a_to_f.md#api)'s
audit log. It holds the severity every audit line carries and the two ways a domain appends a
line to `audit_logs`, so every domain records what it did without depending on the
administration domain that reads, streams and exports the lines.

## Contents

```text
crates/api/api_audit_log/
├── Cargo.toml  the package: `api_identifiers`, sqlx (`postgres`), serde, layout tier 2
└── src/        the severity, the best-effort and transactional appends, the error and the prelude
```

## How it works

A required line is appended on the connection of the caller's business transaction, so the line
and the change it records commit or roll back together; an actor append whose account does not
exist fails with `sqlx::Error::RowNotFound`, so no line is attributed to nobody. A best-effort line
(`write_audit`) runs on the pool and only logs its own failure. Every line stamps `created_at` with
`now()` and fires the publication trigger the administration domain's outbox reads.

## Getting started

Run from the repository root:

```bash
cargo clippy -p api_audit_log --all-targets -- -D warnings
cargo xtask db test-it --test audit_publication --test audit_notify
```

The crate has no unit tests of its own: every append is SQL against the `audit_logs` table, which
the API's integration suites prove against Postgres.

## Configuration

No feature and no variable; the caller passes the pool or the connection.

## Public surface

- `AuditSeverity` (`Info`, `Warn`, `Crit`), the Postgres enum `audit_severity`.
- `audit_writer`: `write_audit` (best effort) and `actor_display_name`.
- `required_audit`: `append_required_audit`, `append_actor_audit`,
  `append_actor_audit_with_severity` and `append_system_audit`.
- `Error` and `Result`, and `prelude` (the severity and every append).

## Boundaries

- Depends on: `api_identifiers` (`DiscordUserId`, `AuditTargetId`), sqlx, serde, thiserror and
  tracing; the `audit_logs` and `users` tables of `crates/api/api_database/migrations/`.
- Used by: the API application (`crates/api/api_server`): its domains, its member activity aggregates, the
  `staging-fixtures` host tool and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing here names a domain
  or another API crate above `api_identifiers`.

## Related documentation

- [API audit log source](/crates/api/api_audit_log/src/README.md) — the files and how the appends
  commit.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
