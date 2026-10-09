# API audit log source

The source of `api_audit_log`: the severity every audit line carries and the two ways any domain
appends a line to `audit_logs`. Reading, publishing and streaming the lines stay in
`crates/api/api_administration/src/`.

## Contents

```text
crates/api/api_audit_log/src/
├── audit_severity.rs  `AuditSeverity`, the `audit_severity` enum
├── audit_writer.rs    best-effort audit append, and the display name a row is attributed to
├── error.rs           `Error`: an audit append that failed, and `Result`
├── lib.rs             the crate root: module header, `mod` lines and the re-exports of `AuditSeverity` and `Error`
├── prelude.rs         `AuditSeverity` and every append for glob import
└── required_audit.rs  audit rows that commit inside the caller's business transaction
```

## How it works

A write that must not happen without its record appends through `required_audit.rs` on the
caller's own transaction (`append_actor_audit`, `append_system_audit` and their variants), so the
action and its audit row commit or fail together; every append, a system one included, stamps
`created_at` with `now()`, the time of that transaction. The appends hand back sqlx's own error,
so the caller's `?` treats an audit failure exactly like the business write beside it.
`audit_writer::write_audit` is the best-effort path: an audit failure is logged and never fails
the action. A trigger announces each insert, which the administration domain's publisher and live
stream pick up.

## Boundaries

- Depends on: `api_identifiers` (the actor and target ids), sqlx, serde, thiserror and tracing;
  no other API crate and no domain.
- Used by: every domain; the member activity aggregates in `crates/api/api_member_activity/src/`,
  whose maintenance wrappers record a warning line; the `staging-fixtures` host tool
  (`append_system_audit`); the integration tests in `crates/api/api_server/tests/`.
- Rules: `audit_logs` is append-only; in the API's Rust code these writers are its only insert
  (database functions in `crates/api/api_database/migrations/` write the others), so a domain that
  needs an audit row calls them rather than writing its own insert; `AuditSeverity` and the
  `audit_severity` enum in the migrations change together.
