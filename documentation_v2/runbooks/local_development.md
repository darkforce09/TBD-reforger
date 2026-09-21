# Local Development Runbook

This runbook documents local environment setup, container vs host execution semantics, cargo task wrappers, and authentication bypasses for the TBD Reforger Platform monorepo.

---

## 1. Host vs Container Execution (`distrobox-host-exec`)

When developing inside isolated Linux environments (such as Debian 12 development containers where `/run/.containerenv` exists), container runtimes (`docker`, `podman`, `psql`, `pg_dump`) are not installed inside the container itself.

To support transparent execution:
- `xtask` automatically detects the container environment via `crate::deploy_db_common::resolve_runtime`.
- Invocations requiring Docker or PostgreSQL client utilities are routed through `distrobox-host-exec` via `crate::hostrun`.
- All `cargo xtask` commands operate identically whether executed directly on a bare-metal Linux host or within a nested development container.

---

## 2. Environment Configuration

Configuration for the backend REST API lives in `apps/website/api_v2/.env`.

```env
APP_ENV=development
DATABASE_URL=postgres://postgres:postgres@localhost:5434/tbd_reforger
FRONTEND_URL=http://127.0.0.1:3000
PORT=8080
JWT_SECRET=development_only_jwt_secret_key_change_in_production
DISCORD_CLIENT_ID=dev_dummy_id
DISCORD_CLIENT_SECRET=dev_dummy_secret
DISCORD_REDIRECT_URI=http://localhost:3000/auth/callback
```

> [!IMPORTANT]
> **Worktree Invariant**: Git worktrees do not carry untracked `.env` files. Whenever creating a new git worktree, manually copy the `.env` file into `apps/website/api_v2/.env` within that worktree.

---

## 3. Canonical Task Wrappers (`cargo xtask`)

The repository uses `cargo xtask` as its primary orchestration tool. Never invoke raw `docker` or `sqlx` commands directly.

### Database Lifecycle (PostgreSQL on host port :5434)
```bash
# Start local Postgres container
cargo xtask db up

# Seed local database with development fixture data
cargo xtask db seed

# View PostgreSQL container logs
cargo xtask db logs

# Stop local Postgres container (persists data volume)
cargo xtask db down

# Execute integration tests against local test DB
cargo xtask db test-it
```

### Development Servers
```bash
# Run Axum REST & SSE API on :8080 (runs migrations automatically on boot)
cargo xtask mk rust-api

# Run Leptos CSR SPA on :3000 (Trunk release build with proxying)
cargo xtask mk leptos
```

Trunk is configured to proxy all `/api/*` and `/map-assets/*` requests from `:3000` to `:8080`.

---

## 4. Developer Authentication Bypass (`dev-login`)

When `APP_ENV=development`, the backend API provides a Discord-free developer authentication bypass:

`GET http://localhost:8080/api/v1/auth/dev-login?role=admin|mission_maker|leader|enlisted`

### Usage:
1. Open the URL in your browser with the desired role parameter (e.g. `?role=admin`).
2. The server creates or updates a mock user account and responds with an HTTP 302 redirecting to `http://localhost:3000/auth/callback#access_token=<JWT>`.
3. The frontend extracts the token, stores it in `sessionStorage`, and authorizes the session immediately without requiring real Discord OAuth credentials.
