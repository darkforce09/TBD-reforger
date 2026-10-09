**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# API v2 & backend architecture specification

**Status:** Proposed design  
**Scope:** `apps/website/api_v2/` and the shared `website-api-types` crate  
**Context:** TBD Reforger platform monorepo  

Detailed architectural specification for `apps/website/api_v2/`, establishing it as the single
authoritative home for all API contracts, shared wire types, backend services, database migrations,
and consolidated integration test suites.

---

## 1. The Single Source of Truth for the API

A central objective of the website reorganization is ending the fragmentation of API ownership:
* In the legacy layout, backend models lived in `api_v2`, while frontend wire DTOs lived in
  `frontend/src/v2/core/api/dto/`.
* When an engineer or AI agent needed to update an API endpoint or payload, they risked changing the
  backend while completely missing the mirrored structs in the frontend.

Under the improved architecture, **`apps/website/api_v2/` is the single source of truth for the API**:
* All wire models, contract schemas, and shared types are housed in `<apps/website/api_v2/types/>`.
* Both the Axum server (`website-api`) and the browser frontend (`website-frontend`) import these
  types directly.
* If you are working on the API, its routes, its data models, or its database, you work in `api_v2/`.

---

## 2. Directory Layout & Subsystem Topology

```text
apps/website/api_v2/
├── .env.example                # Runtime configuration template (ports, database, JWT, Discord)
├── Cargo.toml                  # Backend server crate (`website-api`)
├── README.md                   # Backend architecture, domain table, & API conventions
├── migrations/                 # 39 PostgreSQL SQL migrations (immutable SHA-384 hashes)
├── seeds/                      # 5 dev SQL seeds (applied in strict contractual order)
├── src/                        # Axum server implementation
│   ├── lib.rs                  # Module tree declaring core and 8 domains
│   ├── bin/
│   │   ├── api.rs              # Server binary entrypoint
│   │   └── import_registry.rs  # Mod item envelope ingest tool
│   ├── core/                   # Shared backend foundations
│   │   ├── application_state.rs
│   │   ├── authentication_primitives/  # JWT manager, password hashing, token crypto
│   │   ├── configuration/      # Strongly-typed environment parsing
│   │   ├── database/           # SQLx connection pool & error mapping
│   │   ├── error_handling/     # ApiError enum & HTTP response conversions
│   │   ├── http_router.rs      # Merges all 8 domain route tables under /api/v1
│   │   └── middleware/         # Auth extractors, rate limiting, CORS, tracing
│   ├── background_workers/     # Periodic intervals (Discord sync, token purge, fleet status)
│   └── (8 domain modules)      # administration, command_center, community_content,
│                               # identity_and_access, match_telemetry, missions,
│                               # operations, server_infrastructure
├── tests/                      # [CONSOLIDATED] Domain-grouped integration test suites
│   ├── administration/         # Audit logs, disciplinary, roster tests
│   ├── command_center/         # Announcements, dashboard read tests
│   ├── community_content/      # Equipment viewer, wiki, modpack tests
│   ├── core/                   # Migrations immutability, rate limit, auth middleware
│   ├── identity_and_access/    # Profile, Discord sync, identity link tests
│   ├── match_telemetry/        # Ingest, attendance, leaderboard tests
│   ├── missions/               # Review, deployment, compiled document tests
│   ├── operations/             # ORBAT, event reservations, access policy tests
│   └── server_infrastructure/  # Fleet commands, machine credentials, runtime tests
└── types/                      # [NEW CRATE] Shared wire types (`website-api-types`)
    ├── Cargo.toml              # Minimal dependencies (serde, chrono, uuid)
    ├── README.md               # Guide to shared wire contracts & typify codegen
    └── src/
        ├── lib.rs              # Flat re-exports of all wire types
        ├── is_http_url_cases.rs# [ABSORBED] Shared URL scheme guard test table
        ├── generated/          # Direct output from `cargo xtask schema codegen`
        ├── events.rs           # EventListItem, OrbatSlot, Squad, etc.
        ├── servers.rs          # ServerStatusDto, ServerListItem, etc.
        └── missions.rs         # MissionDetail, ArmoryFaction, etc.
```

---

## 3. The `website-api-types` Shared Crate

### 3.1. Why a Sub-Crate is Required
The backend server crate (`website-api`) depends on heavy native runtimes:
- `tokio = { features = ["full"] }` (epoll, processes, signals)
- `sqlx = { features = ["runtime-tokio", "postgres"] }` (native TCP network sockets)
- `axum`, `tower-http`, `ammonia`, `governor`

None of these compile to WebAssembly (`wasm32-unknown-unknown`). To allow the Leptos frontend to share
types with the backend without pulling in server runtimes, `types/` is isolated as its own Cargo crate.

### 3.2. Crate Configuration (`<apps/website/api_v2/types/Cargo.toml>`)
```toml
[package]
name = "website-api-types"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "UNLICENSED"
publish = false
description = "Shared wire types and contract models for the TBD Reforger platform API."

[lib]
name = "website_api_types"
path = "src/lib.rs"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0", features = ["raw_value"] }
uuid = { version = "1.0", features = ["serde", "v4"] }
```

### 3.3. Integration with `contracts_v2` and xtask Codegen
* The single source of truth for contract definitions is `contracts_v2/definitions/*.schema.json`.
* `tools_v2/xtask/src/commands/generate/schema_types.rs` is updated to output generated Rust code
  into `<apps/website/api_v2/types/src/generated/>` instead of deep inside server domain folders.
* `cargo xtask schema codegen` generates the types, and `cargo xtask ci verify-codegen-fresh`
  guarantees in CI that committed code matches schemas.

### 3.4. Absorption of `apps/website/shared/`
* `is_http_url_cases.rs` is moved into `<apps/website/api_v2/types/src/is_http_url_cases.rs>`.
* Both `website-api` and `website-frontend` access this test table from `website_api_types`,
  allowing the deletion of the ad-hoc single-file directory at the website root.

---

## 4. Consolidation of the Integration Test Suite (95 Flat Test Files)

### 4.1. The Flat Test Binary Overhead Problem
Currently, `apps/website/api_v2/tests/` contains **95 separate `.rs` files** dumped flatly in the root.
* In Cargo, every independent `.rs` file in `tests/` is built as an **independent test executable**.
* Compiling and linking 95 separate binaries on every test run incurs massive linking overhead,
  slows down development iteration, and consumes gigabytes in `target/debug/deps/`.

### 4.2. Domain-Aligned Test Architecture
The 95 test files are reorganized into subdirectories mirroring the backend's 8 domains, backed by
top-level domain runner files:
- `tests/operations_suite.rs` (compiles all `tests/operations/*.rs` in a single link step)
- `tests/missions_suite.rs` (compiles all `tests/missions/*.rs` in a single link step)
- `tests/telemetry_suite.rs` (compiles all `tests/match_telemetry/*.rs`)
- `tests/server_infra_suite.rs` (compiles all `tests/server_infrastructure/*.rs`)
- `tests/auth_suite.rs` (compiles all `tests/identity_and_access/*.rs`)
- `tests/core_suite.rs` (compiles all `tests/core/*.rs`)

*Benefits*:
1. Reduces Cargo link steps from 95 down to ~8 suite binaries (up to an 80% link-time reduction).
2. Tests are organized alongside their corresponding business domains.

---

## 5. Database Invariants: Migrations & Seeds

### 5.1. Migration Immutability & Compile-Time Embedding
- **Location**: `apps/website/api_v2/migrations/` (39 SQL files).
- **Compile-Time Embedding**: `src/core/database/connection_pool.rs` executes:
  ```rust
  sqlx::migrate!("./migrations")
  ```
  This embeds the migration SQL text directly into the compiled binary.
- **SHA-384 Hash Checksum Invariant**:
  `tests/migrations_are_immutable.rs` pins the exact SHA-384 cryptographic hash of every migration
  from `0001_initial_schema.sql` to `0039_...`. Any modification to an existing migration file is
  refused by CI. New database changes must always be appended as `0040_...`.

### 5.2. Seed Application Sequencing
- **Location**: `apps/website/api_v2/seeds/` (5 SQL files).
- **Contractual Order**: Applied by `cargo xtask db seed` in strict sequence:
  1. `discord_roles.sql`: Establishes initial Discord role ID mappings.
  2. `registry_dev.sql`: Seeds basic equipment items for local test environments.
  3. `faction_library.sql`: Populates operational factions and unit hierarchies.
  4. `vehicle_database.sql`: Populates military vehicle manifests and capacities.
  5. `wiki_pages.sql`: Seeds core handbook and documentation articles.
- **Validation**: Enforced by Class-R verification gates (`cargo xtask verify faction-library-seeds`,
  `cargo xtask verify wiki-seeds`).

---

## 6. Configuration & Environment Boundaries

Runtime secrets and environment variables are strictly partitioned:
1. **Application Configuration (`api_v2/.env`)**:
   - Variables parsed by `src/core/configuration/`: `PORT` (default 8080), `DATABASE_URL`,
     `JWT_SECRET`, Discord OAuth credentials (`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`),
     and rate-limiting knobs.
   - Template provided in `.env.example`.
2. **Deployment Secrets (`tools_v2/xtask/deploy/deploy.env`)**:
   - Host SSH keys, remote target IPs, and production container registry tokens. Kept separate
     from application code.

---

## 7. Verification Commands

Following future migration of API components:
```bash
# 1. Verify shared types compile for both native and WASM
cargo check -p website-api-types
cargo check -p website-api-types --target wasm32-unknown-unknown

# 2. Run backend test suite
cargo test -p website-api

# 3. Verify migration immutability
cargo test -p website-api --test migrations_are_immutable

# 4. Verify schema codegen freshness
cargo xtask ci verify-codegen-fresh
```
