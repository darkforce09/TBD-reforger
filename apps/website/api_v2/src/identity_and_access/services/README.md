# Identity & Access Services (`identity_and_access/services/`)

Internal business logic for Discord OAuth integration, session lifecycle persistence, and role synchronization.

---

## 1. Domain Services

### `discord_oauth_client.rs` (<380 LOC)
- **Purpose**: Encapsulates external HTTP interactions with the Discord API v10.
- **Key Functions**:
  - `exchange_code(code: &str, redirect_uri: &str) -> Result<DiscordTokenResponse, AuthError>`: Exchanges OAuth code for Discord bearer tokens.
  - `fetch_user_and_guild_member(access_token: &str, guild_id: &str) -> Result<(DiscordUserPayload, Option<DiscordGuildMember>), AuthError>`: Queries Discord user identity and community guild role array.
- **Invariants**:
  - Implements exponential backoff and rate-limit backpressure for Discord API requests.

### `session_manager.rs` (<350 LOC)
- **Purpose**: Issues, rotates, and revokes cryptographic session credentials.
- **Key Functions**:
  - `create_session(pool: &PgPool, discord_id: &str, role: UserRole) -> Result<TokenPair, AuthError>`: Mints access token and stores hashed refresh token in database.
  - `rotate_refresh_token(pool: &PgPool, raw_token: &str) -> Result<TokenPair, AuthError>`: Implements single-use rotation; invalidates entire family if a token is reused.
  - `revoke_all_user_sessions(pool: &PgPool, discord_id: &str) -> Result<(), DbError>`: Wipes all active sessions on ban or credential reset.

### `role_mapper.rs` (<280 LOC)
- **Purpose**: Maps community Discord guild role IDs to internal platform `UserRole` enums.
- **Key Functions**:
  - `determine_user_role(discord_roles: &[String], config: &DiscordConfig) -> UserRole`: Evaluates highest matching role from configured environment mappings.
