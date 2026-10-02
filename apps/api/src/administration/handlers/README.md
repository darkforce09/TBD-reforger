# Administration handlers

The HTTP handlers of the administrator's console: the member roster, bans and warnings, the
Discord [role](/documentation/glossary/n_to_z.md#role) resync, the membership grace extension and the
audit log with its live feed.

## Contents

```text
apps/api/src/administration/handlers/
├── audit_logs.rs                  the audit console: filtered list, CSV export, live SSE feed
├── disciplinary.rs                bans, ban lifts and warnings against a member
├── membership_grace_overrides.rs  extends a member's cached Discord permissions during an outage
├── mod.rs                         the module tree
├── personnel_roster.rs            the paginated, searchable roster with warning and deployment counts
├── role_management.rs             refuses website role edits, and re-applies the Discord role mapping
└── tests/                         unit tests for the CSV export, the stream events, and the roster paging
```

## How it works

Every handler takes `AdminUser` except `extend_grace`, which takes `AuthUser` and leaves the check
to `identity_and_access::services::membership_grace_overrides`: a previously verified
administrator may extend a member's cached permissions by 1 to 48 hours, with a reason, even while
Discord is unreachable and their own snapshot is stale.

- **Roster.** `GET /api/v1/admin/users?q&page&per_page` answers `{items, page, per_page, total}`:
  one page of `users` with each member's warning count and `total_deployments`, ordered by
  `lower(username)` then `discord_id`. `page` defaults to 1 and `per_page` to 20, and a `per_page`
  above 100 is served as 100; a value below 1 or not a number answers 400 in the
  `{error, details?}` envelope, and a page past the end answers no items with the real total.
  A non-blank `q` matches username, Discord handle, Arma character or Arma id literally, ignoring
  case.
- **Discipline.** A ban requires a non-blank reason; it locks both accounts, sets the ban, revokes
  the member's refresh tokens, queues a re-evaluation of their
  [event](/documentation/glossary/a_to_f.md#event) reservations and appends the `user.ban` audit in one
  transaction. A ban lift clears the ban, queues the same re-evaluation and appends `user.unban` the
  same way; a warning writes a `warnings` row and a best-effort audit line.
- **Roles.** A member's role follows their Discord roles: `PATCH /api/v1/admin/users/{discordId}`
  validates the requested role and then always answers 409, and `POST /api/v1/admin/roles/sync`
  re-applies the `discord_roles` mapping to every account and audits `roles.resync`.
- **[Audit logs](/documentation/glossary/a_to_f.md#audit-logs).** The list reads newest first with
  `?severity=` (`info`, `warn`, `crit`), `?q=` over the message and `?before=` keyset paging. The
  CSV export prefixes cells that would open as spreadsheet formulas. The stream is
  [SSE](/documentation/glossary/n_to_z.md#sse) and opens with `event: ready`, whose id is the start
  cursor (the `Last-Event-ID`, or the tail without one) and whose data is
  `{resume_after, retained_after}`. Each audit row follows as an unnamed event whose id is its
  publication sequence and whose data is the list route's row JSON. A cursor beyond the tail
  (`cursor_ahead`) or below the retained floor (`history_unavailable`), at open or when the floor
  later passes it, becomes `event: reset` with the tail as its id and
  `{reason, resume_after, retained_after}` as its data, and the stream continues from the tail. A
  `Last-Event-ID` that is not a non-negative integer answers 400.

## Boundaries

- Depends on: the domain's models and services (`audit_writer`, `required_audit`,
  `audit_delivery`, `audit_notifier`); `identity_and_access` (`UserRole`, `lock_accounts`,
  `resync_all_roles`, `extend_membership_grace`); the reservation re-evaluation queue in
  `operations::services::event_reservations`; `core` for the extractors, pagination and the
  authorized SSE stream.
- Used by: the domain's `routes.rs`; over HTTP, the personnel and audit log pages in
  `apps/frontend/src/v2/pages/administration/` and the membership status control in
  `apps/frontend/src/v2/pages/navigation/membership_status.rs`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/api/src/tests/architecture_rules.rs`); a ban
  reason and a role are required fields, never defaulted, so a malformed body cannot erase what an
  earlier write recorded.
- Body decoding: every JSON body is read through `ApiError::from_json_rejection`: 413 with
  `details.code = request_too_large` over the body limit, 415 without a JSON content type, and 400
  with the decoder's message (which names the failing field) otherwise.
- Path decoding: every path segment is read through `core::http::path_parameters::PathParams`: a
  segment that does not decode into its type answers 400 in the `{error}` envelope with a message
  naming the parameter, never axum's plain-text rejection.
