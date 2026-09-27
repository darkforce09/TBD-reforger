# Administration models

The rows of the administrator's console: the audit log line with its severity, the control
events of the live audit stream, the personnel roster page and the disciplinary warning, plus the
types generated from the personnel roster and audit log contracts. Keys are snake_case, absent
values are skipped, timestamps are RFC 3339, and each enum maps a Postgres enum.

## Contents

```text
apps/website/api_v2/src/administration/models/
├── audit_log.rs       `AuditLog`, one audit line, and `AuditSeverity`, the `audit_severity` enum
├── audit_stream.rs    `AuditStreamReady` and `AuditStreamReset`, the audit stream's control events
├── generated/         types generated from `personnel-roster.schema.json` and `audit-log.schema.json`
├── mod.rs             the module tree; re-exports `AuditLog`, `AuditSeverity` and `Warning`
├── personnel_page.rs  `PersonnelPage`, one roster page, and `PersonnelRow`, one member on it
└── warning.rs         `Warning`, one disciplinary warning, counted by the personnel roster
```

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx. `generated/` follows
  `contracts_v2/definitions/personnel-roster.schema.json` (`PersonnelPage`, `PersonnelRow`) and
  `contracts_v2/definitions/audit-log.schema.json` (`AuditLogEntry`, `AuditLogPage`,
  `AuditStreamReady`, `AuditStreamReset`).
- Used by: the domain's handlers and services; `command_center`, `community_content`,
  `match_telemetry`, `missions` and `server_infrastructure`, which pass `AuditSeverity` to the
  audit writer; the API's contract tests, which decode live answers into the generated types.
- Rules: `AuditSeverity` and the `audit_severity` enum in the migrations change together;
  `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it); every hand-written wire type carries its
  `@contract` tag (`AuditLog` → `AuditLogEntry`, `AuditStreamReady`, `AuditStreamReset`,
  `PersonnelPage`, `PersonnelRow`), which `cargo xtask schema citations` resolves.
