# Core Configuration (`core/configuration/`)

Strongly typed configuration loaders, environment variable bindings, and runtime operational modes.

---

## 1. Modules

### `app_env.rs` (<180 LOC)
- **Enum**: `AppEnv` (`Development`, `Staging`, `Production`).
- **Behaviors**:
  - Dictates activation of developer bypass routes (e.g., `dev-login`).
  - Enforces strict TLS, CORS origin validation, and secure cookie attributes in non-development modes.

### `settings.rs` (<420 LOC)
- **Sub-Configs**:
  - `DatabaseConfig`: Postgres host, port, credentials, pool sizes, and acquisition timeouts.
  - `ServerConfig`: HTTP bind address, port (default `:8080`), and public base URL.
  - `AuthConfig`: JWT Ed25519/HS256 signing secret, access token TTL, and refresh token cookie settings.
  - `DiscordConfig`: Client ID, client secret, redirect URI, guild ID, and operational bot token.
- **Invariants**:
  - Validates all mandatory environment variables on startup; fails fast with actionable error messages if any required variable is missing.
