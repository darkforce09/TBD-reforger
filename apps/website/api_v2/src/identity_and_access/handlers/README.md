# Identity & Access Handlers (`identity_and_access/handlers/`)

HTTP endpoint controllers for Discord OAuth2 authentication, JWT session lifecycle, authenticated user profiles, and development login gates.

---

## 1. Handlers & Route Mappings

### `oauth_flow.rs` (<420 LOC)
- **`GET /api/v1/auth/discord`** (`start_discord_oauth`):
  - Generates secure random OAuth state and PKCE challenge.
  - Redirects browser to Discord's authorize endpoint with required guild scopes.
- **`GET /api/v1/auth/discord/callback`** (`handle_discord_callback`):
  - Validates OAuth state token.
  - Exchanges authorization code for Discord access token.
  - Queries Discord user profile and guild membership.
  - Evaluates Discord guild roles to assign initial platform `UserRole`.
  - Issues JWT access token and HTTP-only refresh token cookie.

### `session_lifecycle.rs` (<350 LOC)
- **`POST /api/v1/auth/refresh`** (`refresh_session`):
  - Validates HTTP-only refresh token against `refresh_tokens` database table.
  - Issues rotated token pair (refresh token rotation).
- **`POST /api/v1/auth/logout`** (`logout`):
  - Revokes current refresh token in database.
  - Clears authentication cookies.

### `user_profile.rs` (<380 LOC)
- **`GET /api/v1/me`** (`get_current_user`): Returns caller's profile, role, avatar URL, and permissions.
- **`PATCH /api/v1/me/preferences`** (`update_user_preferences`): Updates UI theme, notification settings, and tactical callsigns.

### `dev_auth.rs` (<220 LOC)
- **`GET /api/v1/auth/dev-login`** (`dev_login`):
  - Developer bypass endpoint active exclusively when `APP_ENV=development`.
  - Mints immediate valid JWTs for testing roles (`admin`, `mission_maker`, `leader`, `enlisted`).
  - Returns 404 in staging and production environments.
