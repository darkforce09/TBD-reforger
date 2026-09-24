# Administration models

The rows of the administrator's console: the audit log line with its severity, and the
disciplinary warning. Keys are snake_case, absent values are skipped, timestamps are RFC 3339, and
each enum maps a Postgres enum.

## Contents

```text
apps/website/api_v2/src/administration/models/
├── audit_log.rs  `AuditLog`, one audit line, and `AuditSeverity`, the `audit_severity` enum
├── mod.rs        declares the modules; re-exports `AuditLog`, `AuditSeverity` and `Warning`
└── warning.rs    `Warning`, one disciplinary warning, counted by the personnel roster
```

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx.
- Used by: the domain's handlers and services; `command_center`, `community_content`,
  `match_telemetry`, `missions` and `server_infrastructure`, which pass `AuditSeverity` to the
  audit writer.
- Rules: `AuditSeverity` and the `audit_severity` enum in the migrations change together.
