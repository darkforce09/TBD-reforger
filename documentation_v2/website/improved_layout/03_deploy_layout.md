# Unified deploy architecture

**Status:** Proposed design  
**Scope:** `deploy/` directory and repository deployment tooling  
**Context:** TBD Reforger platform monorepo  

Detailed specification for creating a unified deployment layout under `deploy/`,
consolidating container configurations, and updating downstream xtask verification gates.

---

## 1. Current File Mapping & Fragmentation

Currently, container and deployment files are scattered across the codebase:

```text
CURRENT LOCATIONS:
apps/website/
├── Dockerfile                  <- Production multi-stage image build for website-api
├── docker-compose.staging.yml  <- Staging Postgres + API container stack
└── api_v2/
    └── docker-compose.yml      <- Local development Postgres on host 5434

tools_v2/xtask/deploy/
├── Caddyfile.website           <- Reverse proxy (SPA static files + /api proxy)
├── deploy.env.example          <- Deployment host secret template
└── systemd/
    └── tbd-website-api.service <- User-systemd service template
```

### The Problem
- Developers looking to run local Postgres find `api_v2/docker-compose.yml`.
- Developers looking to test staging find `apps/website/docker-compose.staging.yml`.
- The `Dockerfile` sits at the website root, while the Caddy reverse-proxy sits under `tools_v2/xtask`.
- This fragmentation creates confusion about which compose file applies to which environment and
  which ports (`5434` vs `5432`) are used.

---

## 2. Proposed Target Layout (`deploy/`)

```text
apps/website/deploy/
├── README.md                      # Guide for running local dev, staging, and container builds
├── Dockerfile.api                 # Multi-stage release image build for website-api
├── docker-compose.dev.yml         # Local dev Postgres 18 on host port 5434
├── docker-compose.staging.yml     # Staging Postgres 18 (port 5432) + optional API container
└── caddy/                         # [Optional documentation] Reference to Caddy reverse-proxy
```

### Direct Relocation Mapping

| Original Location | New Location | Description |
|---|---|---|
| `apps/website/Dockerfile` | `deploy/Dockerfile.api` | Multi-stage Rust 1.95 build for `website-api --bin api`. |
| `apps/website/api_v2/docker-compose.yml` | `deploy/docker-compose.dev.yml` | Dev Postgres on host port 5434. |
| `apps/website/docker-compose.staging.yml` | `deploy/docker-compose.staging.yml` | Staging Postgres (5432) + API under `--profile api`. |

---

## 3. Configuration Adjustments & Relative Paths

### 3.1. `docker-compose.staging.yml` Relative Paths
Because the compose file moves from `apps/website/` (2 levels down from repo root) to
`deploy/` (3 levels down), all relative paths must be adjusted:

```yaml
services:
  postgres:
    image: docker.io/library/postgres:18-alpine
    container_name: tbd_staging_db
    restart: unless-stopped
    ports:
      - "127.0.0.1:${TBD_POSTGRES_HOST_PORT:-5432}:5432"
    volumes:
      - tbd_staging_pgdata:/var/lib/postgresql

  api:
    profiles: ["api"]
    build:
      context: ../../..               # Was ../..; now 3 levels to repository root
      dockerfile: apps/website/deploy/Dockerfile.api # Was apps/website/Dockerfile
    image: tbd-website-api:local
    container_name: tbd_staging_api
    volumes:
      # Relative paths adjusted from ../.. to ../../..
      - ../../../assets_v2/terrains:/srv/assets/terrains:ro
      - ../../../assets_v2/glyphs:/srv/assets/glyphs:ro
      - tbd_staging_api_state:/srv/state
```

### 3.2. `docker-compose.dev.yml`
```yaml
# Local development Postgres stack. Host port 5434 avoids collisions with system Postgres.
services:
  db:
    image: docker.io/library/postgres:18-alpine
    container_name: tbd_reforger_db
    environment:
      POSTGRES_USER: tbd
      POSTGRES_PASSWORD: tbd
      POSTGRES_DB: tbd_reforger
    ports:
      - "5434:5432"
    volumes:
      - tbd_pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U tbd -d tbd_reforger"]
      interval: 5s
      timeout: 3s
      retries: 10

volumes:
  tbd_pgdata:
```

---

## 4. Downstream Ripple Effects & xtask Gate Updates

Moving these files touches several critical repository automation tools and verification gates.
These updates must be applied **atomically** in the future migration PR:

### 4.1. Verification Gate: `staging-compose-paths`
* **File**: `tools_v2/xtask/src/verifications/deployment/staging_compose_paths.rs`
* **Current Const**:
  ```rust
  const GOOD_PATH: &str = "apps/website/docker-compose.staging.yml";
  const BAD_PATH: &str = "apps/website/api_v2/docker-compose.staging.yml";
  ```
* **Required Update**:
  ```rust
  const GOOD_PATH: &str = "apps/website/deploy/docker-compose.staging.yml";
  const BAD_PATHS: &[&str] = &[
      "apps/website/docker-compose.staging.yml",
      "apps/website/api_v2/docker-compose.staging.yml",
  ];
  ```
* **Gate Unit Tests**: `tools_v2/xtask/src/verifications/deployment/tests/staging_compose_paths/tests.rs`
  must be updated to assert the new `GOOD_PATH`.

### 4.2. Staging Deploy Runner
* **File**: `tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`
* **Update Lines 217 & 221**:
  ```rust
  // Dry run echo:
  println!("[dry-run] cd $TBD_REMOTE_DIR && docker compose -f apps/website/deploy/docker-compose.staging.yml up -d --build");
  // Live execution:
  runner.ssh_ok(&base, &host, &[format!(
      "cd '$TBD_REMOTE_DIR' && docker compose -f apps/website/deploy/docker-compose.staging.yml up -d --build"
  )], None)
  ```

### 4.3. Website Deploy Remote Steps
* **File**: `tools_v2/xtask/src/commands/deploy/website/remote_steps.rs`
* **Update Line 32 (`compose_up`)**:
  ```rust
  format!(
      "cd '{remote_dir}' && \
       export TBD_POSTGRES_HOST_PORT='{postgres_port}' && \
       if command -v docker >/dev/null 2>&1; then \
         docker compose -f apps/website/deploy/docker-compose.staging.yml up -d postgres; \
       else \
         podman compose -f apps/website/deploy/docker-compose.staging.yml up -d postgres; \
       fi"
  )
  ```

### 4.4. Database Operations (`cargo xtask db ...`)
* **File**: `tools_v2/xtask/src/commands/db/operations.rs` & `recipes.rs`
* **Update**:
  Currently, `cargo xtask db up` (and `down`, `logs`) runs:
  ```text
  cd apps/website/api_v2 && podman/docker compose up -d db
  ```
  With the dev compose file relocated, `xtask db` recipes should point to:
  ```text
  docker compose -f apps/website/deploy/docker-compose.dev.yml up -d db
  ```
* **Acceptance Test Selftest**: `tools_v2/xtask/src/commands/db/operations/selftest.rs` compares
  recipe strings byte-for-byte against a frozen baseline. When updating `xtask db`, `selftest.rs`
  and `recipes.rs` must be adjusted simultaneously.

---

## 5. Summary of Verification Commands

After applying the deploy layout migration in a future execution session:
```bash
# 1. Verify staging compose path gate passes
cargo xtask verify staging-compose-paths

# 2. Run database command selftest
cargo xtask db selftest

# 3. Test dry-run staging deploy
cargo xtask deploy staging --dry-run
```
