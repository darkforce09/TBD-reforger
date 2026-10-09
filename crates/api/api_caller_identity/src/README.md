# API caller identity source

The source of `api_caller_identity`: who a request acts as and with what authority. It holds the
role ladder, the account authority read from the verified Discord membership snapshot, the session
authorization behind the `AuthUser` extractor, the identity and account lock order, and the
machine caller a game host authenticates as.

## Contents

```text
crates/api/api_caller_identity/src/
├── account_authority.rs              an account's permissions from its verified Discord snapshot
├── arma_identity_link.rs             whether an account counts as linked to an Arma identity
├── cached_membership_permissions.rs  grace, staleness and override decisions over a cached snapshot
├── error.rs                          `Error`: a read or lock that failed, its `ApiError`, and `Result`
├── identity_ownership.rs             the identity and account locks, in their fixed order
├── lib.rs                            the crate root: module header, `mod` lines and the re-exports of `UserRole` and `Error`
├── machine_authentication.rs         the `MachineCaller` extractor every machine route takes
├── machine_caller.rs                 verifies a machine secret into the `MachineCaller`
├── prelude.rs                        the role, the checks, the locks and the machine caller for glob import
├── session_authorization.rs          decides a session's current authority in PostgreSQL
├── tests/                            unit tests for the permission decisions, the link flag and the machine caller
└── user_role.rs                      `UserRole`, the `user_role` enum
```

## How it works

- **Sessions.** An access token names a persisted session; `session_authorization.rs` rechecks
  that session and the account's authority in PostgreSQL on every request, through
  `DatabaseSessionAuthority`, which the API's composition root builds and the application state
  holds for the `AuthUser` extractor; `authorize_on_connection` rechecks inside a business
  transaction.
- **Authority from Discord.** `account_authority.rs` reads the account's verified, guild-scoped
  Discord snapshot and never `users.role`; `cached_membership_permissions.rs` turns it into the
  effective role, a stale flag and the override state.
- **Locks.** `identity_ownership.rs` takes sorted Arma identity locks, then sorted account locks;
  every identity, reservation and telemetry transaction takes them in that order.
- **Machines.** A machine presents `Authorization: Bearer tbdm_<credential id>_<64 hex digits>`.
  `machine_caller.rs` selects the credential row by the id and compares the SHA-256 of the whole
  secret in constant time, so malformed, unknown and mismatched secrets answer the same 401. The
  `MachineCaller` names the one server and the executor kind (`host_agent` or `mod_runtime`) the
  credential serves. The extractor in `machine_authentication.rs` works on any router state that
  exposes a `PgPool` through `FromRef`.

## Boundaries

- Depends on: `api_http_layer` (the access token claims, the token hash and its constant-time
  compare, the `AuthUser` extractor and the session authority trait), `api_configuration`
  (`Config`), `api_foundation` (`ApiError`) and `api_identifiers`; `fleet_wire_contract` for
  `ExecutorKind` and the credential format; sqlx, axum, serde, chrono, uuid.
- Used by: the API application (`crates/api/api_server`): its composition root (`DatabaseSessionAuthority`);
  the `api_identity_and_access`, `api_administration`, `api_missions`, `api_operations`, `api_server_infrastructure`
  and `api_match_telemetry` domains (`authorize_on_connection`, `lock_accounts`, `lock_identities`,
  `holds_administrator_authority`, `evaluate_cached_membership_permissions`, `UserRole`,
  `MachineCaller`); the `staging-fixtures` host tool (`lock_account`,
  `holds_administrator_authority`); the integration tests in `crates/api/api_server/tests/`.
- Rules: permissions come only from the configured guild's verified snapshot; every writer that
  touches identities or accounts takes the `identity_ownership.rs` lock order; a credential id
  accepts upper- and lower-case hex digits and its random part lower-case only; a machine caller
  acts only as its own executor for its own server.
