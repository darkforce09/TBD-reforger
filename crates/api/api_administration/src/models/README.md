# Administration models

The rows of the administrator's console: the audit log line, the control
events of the live audit stream, the personnel roster page and the disciplinary warning. Keys are snake_case, absent
values are skipped, timestamps are RFC 3339, and each enum maps a Postgres enum.

## Contents

```text
crates/api/api_administration/src/models/
├── audit_log.rs       `AuditLog`, one audit line
├── audit_stream.rs    `AuditStreamReady` and `AuditStreamReset`, the audit stream's control events
├── mod.rs             the module tree; re-exports `AuditLog` and `Warning`
├── personnel_page.rs  `PersonnelPage`, one roster page, and `PersonnelRow`, one member on it
└── warning.rs         `Warning`, one disciplinary warning, counted by the personnel roster
```

## Boundaries

- Depends on: `fleet_wire_contract::rfc3339_timestamps` for timestamps; `api_audit_log::AuditSeverity`, the
  severity of a line; serde and sqlx. The types generated from
  `contracts/definitions/personnel-roster.schema.json` (`PersonnelPage`, `PersonnelRow`) and
  `contracts/definitions/audit-log.schema.json` (`AuditLogEntry`, `AuditLogPage`,
  `AuditStreamReady`, `AuditStreamReset`) are `contract_schema_types::administration`.
- Used by: the domain's handlers and services; the API's contract tests, which decode live
  answers into the generated types.
- Rules: the generated types are written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks them); every hand-written wire type carries its
  `@contract` tag (`AuditLog` → `AuditLogEntry`, `AuditStreamReady`, `AuditStreamReset`,
  `PersonnelPage`, `PersonnelRow`, `Warning` → `personnel-actions.schema.json#/definitions/Warning`).
