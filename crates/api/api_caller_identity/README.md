# API caller identity

The `api_caller_identity` crate: who an [API](/documentation/glossary/a_to_f.md#api) request acts
as and with what authority. It holds the role ladder, the account authority read from the verified
Discord membership snapshot, the session authorization behind the `AuthUser` extractor, the
identity and account lock order, and the machine caller a game host authenticates as, so every
domain authorizes its caller without depending on the identity or server infrastructure domains.

## Contents

```text
crates/api/api_caller_identity/
├── Cargo.toml  the package: `api_http_layer`, `api_configuration`, `api_foundation`, `api_identifiers`, `fleet_wire_contract`, sqlx (`postgres`), axum, layout tier 4
└── src/        the role ladder, the authority read, the session check, the locks, the machine caller, the error and the prelude
```

## How it works

An access token names a persisted session. `DatabaseSessionAuthority` implements
`api_http_layer`'s `SessionAuthority`: for every request the `AuthUser` extractor presents, it
rechecks the session row and the account's current authority in one statement, so a revoked
session, a deleted account or a ban takes effect on the next request. The authority comes from the
configured guild's verified Discord membership snapshot, never from `users.role`. A business
transaction that needs the caller's authority at commit time locks the account and calls
`authorize_on_connection` on its own connection.

A game host or game runtime presents a machine secret instead. The `MachineCaller` extractor
selects the credential row by the id the secret names, compares the SHA-256 of the whole secret in
constant time and answers the server and executor the credential serves. The credential id accepts
upper- and lower-case hex digits; the random part lower-case only.

## Getting started

Run from the repository root:

```bash
cargo test -p api_caller_identity
cargo clippy -p api_caller_identity --all-targets -- -D warnings
cargo xtask db test-it --test auth_refresh --test fleet_command_ledger
```

The unit tests cover the permission decisions, the Arma link flag and the machine secret parse;
the session and credential reads are SQL the API's integration suites prove against Postgres.

## Configuration

No feature and no variable. The session check reads the configured guild and the development flag
from the `Config` the API loads at boot (`api_configuration`).

## Public surface

- `UserRole`, the Postgres enum `user_role`.
- `account_authority`: `AccountAuthority`, `load_account_authority`,
  `holds_administrator_authority`, `lock_account` and `ACCOUNT_AUTHORITY_QUERY`.
- `cached_membership_permissions`: `evaluate_cached_membership_permissions`,
  `can_manage_sync_override`, `CachedMembershipPermissionDecision` and the grace and freshness
  periods.
- `session_authorization`: `DatabaseSessionAuthority`, `authorize_session` and
  `authorize_on_connection`.
- `identity_ownership`: `lock_identities` and `lock_accounts`.
- `arma_identity_link::arma_id_is_linked`.
- `machine_caller`: `MachineCaller` (its `require_executor` and `require_server`) and
  `authenticate_machine`; `machine_authentication`: the `MachineCaller` extractor.
- `Error` and `Result` (a read or lock failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_http_layer`, `api_configuration`, `api_foundation`, `api_identifiers`,
  `fleet_wire_contract`, sqlx, axum, serde, chrono, futures, thiserror and uuid; the `users`,
  `authentication_sessions`, `discord_membership_snapshots`, `user_discord_roles`,
  `discord_roles`, `discord_membership_grace_overrides`, `arma_identity_serialization`,
  `server_machine_credentials` and `servers` tables of `crates/api/api_database/migrations/`.
- Used by: the API application (`crates/api/api_server`): its composition root, its domains, the
  `staging-fixtures` host tool and the integration suites; the application state crate
  (`api_state`) holds the authority as `Arc<dyn SessionAuthority>` and never names this crate.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing here names a domain.

## Related documentation

- [API caller identity source](/crates/api/api_caller_identity/src/README.md) — the files and how
  sessions, authority, locks and machine callers work.
- [API HTTP layer](/crates/api/api_http_layer/README.md) — the `AuthUser` extractor and the
  session authority trait this crate implements.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
