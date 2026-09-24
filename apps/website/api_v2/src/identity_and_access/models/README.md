# Identity and access models

The account row and the rows that hang off it, as the database stores them and the wire carries
them: snake_case keys, absent values skipped, RFC 3339 timestamps. `UserRole` maps the Postgres
enum `user_role`, ordered `guest` < `enlisted` < `leader` < `mission_maker` < `admin`.

## Contents

```text
apps/website/api_v2/src/identity_and_access/models/
├── current_profile.rs  the `GET` and `PATCH /api/v1/me` answers, with link and membership flags
├── generated/          types generated from `current-profile.schema.json`, read by the contract test
├── mod.rs              the module tree; re-exports the account types
└── user_account.rs     `User`, `UserRole` and the Discord role, link code and refresh token rows
```

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx; `generated/` follows
  `contracts_v2/definitions/current-profile.schema.json`.
- Used by: the domain's handlers and services; `administration` handlers and
  `operations::services::event_access`, which read `UserRole`; the contract test
  `apps/website/api_v2/tests/current_profile_contract.rs`, which decodes live answers into the
  generated types; the web app's `apps/website/frontend/src/v2/core/api/dto/auth.rs` mirrors the
  wire shape.
- Rules: soft-delete columns stay out of these structs, since the queries filter them;
  `RefreshToken.token_hash` never reaches the wire; `generated/` is written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks it).
