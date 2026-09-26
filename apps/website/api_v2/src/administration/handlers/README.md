# Administration handlers

The HTTP handlers of the administrator's console: the member roster, bans and warnings, the
Discord [role](/documentation_v2/glossary/n_to_z.md#role) resync, the membership grace extension and the
audit log with its live feed.

## Contents

```text
apps/website/api_v2/src/administration/handlers/
├── audit_logs.rs                  the audit console: filtered list, CSV export, live SSE feed
├── disciplinary.rs                bans, ban lifts and warnings against a member
├── membership_grace_overrides.rs  extends a member's cached Discord permissions during an outage
├── mod.rs                         the module tree
├── personnel_roster.rs            the paginated, searchable roster with warning and deployment counts
├── role_management.rs             refuses website role edits, and re-applies the Discord role mapping
└── tests/                         unit tests for the audit log and the roster
```

## How it works

Every handler takes `AdminUser` except `extend_grace`, which takes `AuthUser` and leaves the check
to `identity_and_access::services::membership_grace_overrides`: a previously verified
administrator may extend a member's cached permissions by 1 to 48 hours, with a reason, even while
Discord is unreachable and their own snapshot is stale.

- **Roster.** `GET /api/v1/admin/users` pages `users` (`limit`, `offset`, `?q=` search) with each
  member's warning count and `total_deployments`.
- **Discipline.** A ban requires a non-blank reason; it locks both accounts, sets the ban, revokes
  the member's refresh tokens, queues a re-evaluation of their
  [event](/documentation_v2/glossary/a_to_f.md#event) reservations and appends the `user.ban` audit in one
  transaction. A ban lift clears the ban, queues the same re-evaluation and appends `user.unban` the
  same way; a warning writes a `warnings` row and a best-effort audit line.
- **Roles.** A member's role follows their Discord roles: `PATCH /api/v1/admin/users/{discordId}`
  validates the requested role and then always answers 409, and `POST /api/v1/admin/roles/sync`
  re-applies the `discord_roles` mapping to every account and audits `roles.resync`.
- **[Audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs).** The list reads newest first with
  `?severity=` (`info`, `warn`, `crit`), `?q=` over the message and `?before=` keyset paging. The
  CSV export prefixes cells that would open as spreadsheet formulas. The stream is
  [SSE](/documentation_v2/glossary/n_to_z.md#sse): each message's id is the row's publication sequence,
  `Last-Event-ID` replays what came later, and a cursor older than the retained history answers
  409.

## Boundaries

- Depends on: the domain's models and services (`audit_writer`, `required_audit`,
  `audit_delivery`, `audit_notifier`); `identity_and_access` (`UserRole`, `lock_accounts`,
  `resync_all_roles`, `extend_membership_grace`); the reservation re-evaluation queue in
  `operations::services::event_reservations`; `core` for the extractors, pagination and the
  authorized SSE stream.
- Used by: the domain's `routes.rs`; over HTTP, the personnel and audit log pages in
  `apps/website/frontend/src/v2/pages/administration/` and the membership status control in
  `apps/website/frontend/src/v2/pages/navigation/membership_status.rs`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); a ban
  reason and a role are required fields, never defaulted, so a malformed body cannot erase what an
  earlier write recorded.
