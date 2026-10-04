# Identity and access models

The account row and the rows that hang off it, as the database stores them and the wire carries
them: snake_case keys, absent values skipped, RFC 3339 timestamps. The role an account holds is
the caller identity crate's `UserRole` (`crates/api/api_caller_identity/src/user_role.rs`).

## Contents

```text
crates/api/api_identity_and_access/src/models/
├── current_profile.rs  the `GET` and `PATCH /api/v1/me` answers, with link and membership flags
├── mod.rs              the module tree; re-exports the account types
└── user_account.rs     `User` and the Discord role, link code and refresh token rows
```

## Boundaries

- Depends on: `fleet_wire_contract::rfc3339_timestamps` for timestamps; `api_caller_identity::UserRole`; serde and
  sqlx; the types generated from
  `contracts/definitions/current-profile.schema.json` are
  `contract_schema_types::identity_and_access::current_profile`. `current_profile.rs` and
  `user_account.rs` carry `@contract` tags for `current-profile.schema.json` (the root and
  `UserAccount`) and `profile-update.schema.json` (`UpdatedProfile`, the `PATCH /api/v1/me`
  answer), which `cargo xtask schema citations` resolves.
- Used by: the domain's handlers and services; the contract test
  `apps/api/tests/current_profile_contract.rs`, which decodes live answers into the
  generated types; the web app's `crates/frontend/foundation/frontend_api_dtos/src/auth.rs` mirrors the
  wire shape.
- Rules: soft-delete columns stay out of these structs, since the queries filter them;
  `RefreshToken.token_hash` never reaches the wire; the generated types are written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks them).
