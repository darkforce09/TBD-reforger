# Authentication primitives

The credential building blocks every authenticated surface of the
[API](/documentation/glossary/a_to_f.md#api) shares: signed access tokens, opaque tokens stored only
as hashes, constant-time comparison, and the trait through which a verified token's session is
checked against the database.

## Contents

```text
crates/api/api_http_layer/src/authentication_primitives/
├── jwt_manager.rs        `Manager` and `Claims`: issues and verifies the HS256 access tokens
├── mod.rs                the module tree; re-exports `Claims`, `Manager` and the token helpers
├── session_authority.rs  `SessionAuthority`, the trait that turns verified claims into an `AuthUser`
├── tests/                unit tests for the token manager and the token helpers
└── token_hashing.rs      random opaque tokens, their SHA-256 digest, constant-time compare, codes
```

## How it works

An access token is an HS256 JSON Web Token that `Manager::issue_access` signs with `JWT_SECRET`.
`sub` is the member's Discord id, `sid` the id of the persisted session the token belongs to,
`role` and `arma_linked` the values current at issue, `iss` is `tbd-reforger`, `aud` is
`tbd-website`, and `exp` lies `JWT_ACCESS_TTL_MIN` minutes after `iat` (15 when the value is zero
or negative). `Manager::parse` accepts HS256 alone, requires the issuer, audience, subject and
expiry with no leeway, and refuses a nil session id, an empty subject, and an `iat` in the future
or at or after `exp`.

A verified token is not yet an identity. The `AuthUser` extractor in `crate::middleware`
hands the claims to the `SessionAuthority` that `AppState` holds, which reads the session and the
account from Postgres and answers with the member's current
[role](/documentation/glossary/n_to_z.md#role) and account state, or refuses. The trait lives here so
the extractor depends on this crate alone: `DatabaseSessionAuthority` in
`crates/api/api_caller_identity/src/session_authorization.rs` implements it,
and `crates/api/api_state/src/application_state.rs` wires it in.

Opaque tokens (the OAuth `state`, refresh tokens,
[machine credential](/documentation/glossary/g_to_m.md#machine-credential) secrets) come from
`random_token`, hex-encoded random bytes. The database stores only `hash_token`, their hex SHA-256,
so a leaked table holds nothing a caller can present. `constant_time_equal` compares secrets
without leaking timing, and `numeric_code` draws the zero-padded decimal code of the Arma identity
link.

## Boundaries

- Depends on: `jsonwebtoken` with its pure-Rust HS256 backend, `sha2`, `subtle`, `rand` and `hex`;
  `ApiError` from `api_foundation::error_handling` and `AuthUser` from `crate::middleware`, which
  the trait's signature names.
- Used by:
  - `crate::middleware` (bearer verification in `AuthUser`),
    `crate::observability::observability_auth` (the `OBSERVABILITY_TOKEN` bearer check) and
    `api_state`'s `AppState` (the `Manager` and the session authority it holds);
  - `api_identity_and_access`: the OAuth state and its host guard, session issue, storage and
    rotation, session authorization and link codes;
  - `api_server_infrastructure`: issuing and checking machine credential secrets;
  - the integration fixtures under `crates/api/api_server/tests/`, which sign tokens and hash
    secrets the same way.
- Rules: a stored credential is always its `hash_token` digest, never the raw token; secrets
  compare with `constant_time_equal`; the token validation above is pinned by
  `tests/jwt_manager.rs`, algorithm confusion and unsigned tokens included.

## Related documentation

- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — how access tokens, persisted sessions and refresh rotation fit together.
