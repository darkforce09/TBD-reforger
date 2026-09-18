# Identity & Access Models (`identity_and_access/models/`)

Domain models, database entities, wire DTOs, and JWT claim definitions for user identity, sessions, and role credentials.

---

## 1. Domain Entities & Schemas

### `user_session.rs` (<350 LOC)
- **Database Table**: `users`, `refresh_tokens`
- **Fields**:
  - `discord_id: String`: Primary Discord snowflake ID.
  - `username: String`: Discord username.
  - `discriminator: Option<String>`: Legacy discriminator or empty.
  - `avatar_hash: Option<String>`: Discord CDN avatar identifier.
  - `role: UserRole`: Domain role enum (`enlisted`, `leader`, `mission_maker`, `admin`).
  - `is_banned: bool`: Immediate access denial flag.
  - `created_at: DateTime<Utc>`, `last_login_at: DateTime<Utc>`.

### `jwt_claims.rs` (<220 LOC)
- **Token Claims**:
  - `sub: String`: Discord snowflake identifier.
  - `role: UserRole`: Embedded caller role for zero-lookup middleware authorization.
  - `exp: usize`: UNIX expiration timestamp (short-lived, 15 minutes).
  - `iat: usize`: Token issuance timestamp.
  - `jti: Uuid`: Unique token ID for replay attack prevention.

### `oauth_contracts.rs` (<250 LOC)
- **Wire Payloads**:
  - `DiscordTokenResponse`: `access_token`, `token_type`, `expires_in`, `refresh_token`, `scope`.
  - `DiscordUserPayload`: Discord `/users/@me` user response.
  - `DiscordGuildMember`: Discord `/users/@me/guilds/{id}/member` role list and nickname.
  - `DevLoginParams`: Role selection parameter for local offline testing.
