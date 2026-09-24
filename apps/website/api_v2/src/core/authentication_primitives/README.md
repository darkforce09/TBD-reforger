# Authentication primitives

The credential building blocks every authenticated surface of the
[API](/documentation_v2/glossary.md#api) shares: signed access tokens, opaque tokens stored only
as hashes, constant-time comparison, and the trait through which a verified token's session is
checked against the database.

## Contents

```text
apps/website/api_v2/src/core/authentication_primitives/
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

A verified token is not yet an identity. The `AuthUser` extractor in `crate::core::middleware`
hands the claims to the `SessionAuthority` that `AppState` holds, which reads the session and the
account from Postgres and answers with the member's current
[role](/documentation_v2/glossary.md#role) and account state, or refuses. The trait lives here so
the extractor depends on `core` alone: `DatabaseSessionAuthority` in
`apps/website/api_v2/src/identity_and_access/services/session_authorization.rs` implements it,
and `apps/website/api_v2/src/core/application_state.rs` wires it in.

Opaque tokens (the OAuth `state`, refresh tokens,
[machine credential](/documentation_v2/glossary.md#machine-credential) secrets) come from
`random_token`, hex-encoded random bytes. The database stores only `hash_token`, their hex SHA-256,
so a leaked table holds nothing a caller can present. `constant_time_equal` compares secrets
without leaking timing, and `numeric_code` draws the zero-padded decimal code of the Arma identity
link.

## Boundaries

- Depends on: `jsonwebtoken` with its pure-Rust HS256 backend, `sha2`, `subtle`, `rand` and `hex`;
  `ApiError` from `crate::core::error_handling` and `AuthUser` from `crate::core::middleware`, which
  the trait's signature names.
- Used by:
  - `crate::core::middleware` (bearer verification in `AuthUser`, the `ServiceAuth` comparison),
    `crate::core::observability::health_probe` (the service-token check) and
    `crate::core::application_state` (the `Manager` and the session authority it holds);
  - `identity_and_access`: the OAuth state and its host guard, session issue, storage and
    rotation, session authorization and link codes;
  - `server_infrastructure`: issuing and checking machine credential secrets;
  - the integration fixtures under `apps/website/api_v2/tests/`, which sign tokens and hash
    secrets the same way.
- Rules: a stored credential is always its `hash_token` digest, never the raw token; secrets
  compare with `constant_time_equal`; the token validation above is pinned by
  `tests/jwt_manager.rs`, algorithm confusion and unsigned tokens included.

## Related documentation

- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — how access tokens, persisted sessions and refresh rotation fit together.
