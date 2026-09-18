# Identity and Access Subsystem (`identity_and_access/`)

Discord OAuth2 authentication, session management, rotating refresh tokens (Gate G7a), user profiles, and Arma Reforger account linking.

---

## 1. Subsystem Topology & Responsibilities

The `identity_and_access/` domain decomposes the 1,100 LOC `oauth.rs` and 669 LOC `me.rs` files into modular components while isolating auth crypto primitives:

```text
src/identity_and_access/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/auth & /api/v1/me sub-router (<70 LOC)
│
├── models/
│   ├── mod.rs
│   └── auth_session.rs                 <-- UserProfile, JwtClaims, SessionToken (<100 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── discord_oauth.rs                <-- OAuth2 consent redirect, code exchange & user upsert (<380 LOC)
│   ├── oauth_host_guard.rs             <-- Host alignment check & CSRF cookie validation (<150 LOC)
│   ├── session_tokens.rs               <-- Single-use rotating refresh tokens (Gate G7a) & logout (<240 LOC)
│   ├── member_profile.rs               <-- User profile retrieval and update (<100 LOC)
│   ├── arma_linking.rs                 <-- 6-digit link code issue, status polling, unlink & confirm (<380 LOC)
│   └── developer_login.rs              <-- Local development bypass login shortcut (<125 LOC)
│
├── services/
│   ├── discord_client.rs               <-- Discord OAuth2 & guild member API client (<370 LOC)
│   ├── role_synchronizer.rs            <-- Discord role snapshot synchronization (<190 LOC)
│   └── tests/
│       ├── discord_client.rs           <-- Sibling unit tests (<280 LOC)
│       └── role_synchronizer.rs        <-- Sibling unit tests (<130 LOC)
│
├── auth_primitives/
│   ├── jwt_manager.rs                  <-- HS256 JWT encoding and decoding (<85 LOC)
│   ├── token_generator.rs              <-- CSPRNG tokens, SHA-256 hashes, constant time (<40 LOC)
│   └── tests/
│       ├── jwt_manager.rs              <-- Sibling unit tests (<50 LOC)
│       └── token_generator.rs          <-- Sibling unit tests (<45 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── discord_oauth.rs
    ├── session_tokens.rs
    ├── member_profile.rs
    └── arma_linking.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/auth/discord/login` | `discord_oauth::discord_login` | Public | Initiates OAuth flow: checks host alignment, sets CSRF cookie, redirects 307. |
| `GET` | `/api/v1/auth/discord/callback`| `discord_oauth::discord_callback`| Public | Validates state, exchanges code, upserts user, syncs roles, redirects 302 with tokens. |
| `POST` | `/api/v1/auth/refresh` | `session_tokens::refresh` | Public | Single-use rotating refresh token exchange; revokes on reuse detection. |
| `POST` | `/api/v1/auth/logout` | `session_tokens::logout` | Public | Revokes active refresh token. |
| `GET` | `/api/v1/auth/dev-login` | `developer_login::dev_login` | Dev Mode | Dev login shortcut bypassing Discord. |
| `GET` | `/api/v1/me` | `member_profile::get_me` | `AuthUser` | Profile information, current role, and `arma_linked` boolean flag. |
| `PATCH`| `/api/v1/me` | `member_profile::update_me` | `AuthUser` | Profile updates. |
| `POST` | `/api/v1/me/link` | `arma_linking::create_link_code`| `AuthUser` | Generates short-lived 6-digit linking code (10m TTL). |
| `DELETE`| `/api/v1/me/link` | `arma_linking::unlink` | `AuthUser` | Unlinks Arma Reforger profile and releases claimed player stats. |
| `GET` | `/api/v1/me/link/status` | `arma_linking::link_status` | `AuthUser` | Polls whether account has linked Arma ID or pending codes. |
| `POST` | `/api/v1/ingest/link-confirm` | `arma_linking::ingest_link_confirm`| `ServiceAuth` | Game server confirms 6-digit code, binds `arma_id`, and triggers backfills. |

---

## 3. Key Invariants & Security Controls

### 3.1 Single-Use Rotating Refresh Tokens (Gate G7a)
`POST /auth/refresh` atomically revokes the presented refresh token and issues a new access/refresh pair:
```sql
UPDATE refresh_tokens SET revoked_at = now()
WHERE id = $1 AND revoked_at IS NULL
```
If a revoked token is presented, the system detects a token theft event and revokes the **entire token family** for that user snowflake.

### 3.2 Host-Alignment Guard (`oauth_host_guard.rs`)
In development, if the browser origin does not match `FRONTEND_URL`, the flow is refused before setting cookies. In split-host production environments (SPA on `app.example.com`, API on `api.example.com`), host mismatch logs a one-time advisory without breaking login.

### 3.3 CDN Avatar URL Validation
To prevent URL injection, `discord_client::is_cdn_path_segment` validates all avatar hashes against `^[A-Za-z0-9_]+$` before building Discord CDN image URLs.
