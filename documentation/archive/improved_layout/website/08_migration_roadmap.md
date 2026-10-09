**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# Phased migration roadmap

**Status:** Proposed design  
**Scope:** Execution roadmap for `apps/website` reorganization  
**Context:** TBD Reforger platform monorepo  

A step-by-step, phased implementation plan designed to execute the reorganization in future sessions
without breaking CI or any repository verification gates.

---

## 1. Migration Philosophy & Gate Safety

The TBD Reforger monorepo enforces rigorous automated gates (`ci-local`, `engine-layers`,
`staging-compose-paths`, `readme-coverage`, `markdown-placement`). To ensure zero downtime and clean
pull requests, the reorganization is split into four isolated, independently verifiable phases:

```text
┌─────────────────────────────────┐
│  Phase 1: Deploy Consolidation  │ -> Move Docker/compose configs & update xtask pins atomically
└────────────────┬────────────────┘
                 │ (Verified clean by xtask verify staging-compose-paths)
                 ▼
┌─────────────────────────────────┐
│  Phase 2: Establish API Types   │ -> Create website-api-types crate, absorb shared/, group tests
└────────────────┬────────────────┘
                 │ (Verified clean by cargo check --target wasm32 and native tests)
                 ▼
┌─────────────────────────────────┐
│  Phase 3: Frontend Transport    │ -> Rename core/transport, extract shell/, decouple tokens
└────────────────┬────────────────┘
                 │ (Verified clean by cargo check, clippy wasm32, trunk build)
                 ▼
┌─────────────────────────────────┐
│  Phase 4: Docs & Cruft Cleanup  │ -> Delete map_engine cruft, verify all monorepo gates
└─────────────────────────────────┘
```

---

## 2. Phase 1: Deploy Layout Consolidation

### Objective
Create `<apps/website/deploy/>`, relocate container files, and update downstream xtask pins
simultaneously.

### Step-by-Step Execution
1. **Create directory**: Create `deploy/` under `apps/website/`.
2. **Move files**:
   - `git mv apps/website/Dockerfile apps/website/deploy/Dockerfile.api`
   - `git mv apps/website/docker-compose.staging.yml apps/website/deploy/docker-compose.staging.yml`
   - `git mv apps/website/api_v2/docker-compose.yml apps/website/deploy/docker-compose.dev.yml`
3. **Update compose paths**:
   - In `<apps/website/deploy/docker-compose.staging.yml>`:
     - Change `build: context:` from `../..` to `../../..`.
     - Change `dockerfile:` from `apps/website/Dockerfile` to `deploy/Dockerfile.api`.
     - Change volume mounts from `../../assets_v2/...` to `../../../assets_v2/...`.
4. **Update xtask pins & runners**:
   - In `tools_v2/xtask/src/verifications/deployment/staging_compose_paths.rs`:
     - Update `const GOOD_PATH: &str = "apps/website/deploy/docker-compose.staging.yml";`.
   - In `tools_v2/xtask/src/verifications/deployment/tests/staging_compose_paths/tests.rs`:
     - Update test assertions to match the new `GOOD_PATH`.
   - In `tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`:
     - Update dry-run and live SSH compose invocation strings.
   - In `tools_v2/xtask/src/commands/deploy/website/remote_steps.rs`:
     - Update `compose_up` invocation.
   - In `tools_v2/xtask/src/commands/db/operations.rs` & `recipes.rs`:
     - Update `cargo xtask db` commands to point to `<apps/website/deploy/docker-compose.dev.yml>`.
   - In `tools_v2/xtask/src/commands/db/operations/selftest.rs`:
     - Update recipe parity test expectations.
5. **Add `<apps/website/deploy/README.md>`**:
   - Include `## Contents` block matching direct files.
6. **Verify Phase 1**:
   ```bash
   cargo xtask verify staging-compose-paths
   cargo xtask db selftest
   cargo xtask deploy staging --dry-run
   cargo xtask verify readme-coverage
   ```

---

## 3. Phase 2: Establish `website-api-types` Shared Crate & Reorganize Tests

### Objective
Establish the lightweight wire types crate under `<apps/website/api_v2/types/>`, absorb the shared URL
table, and consolidate the 95 flat integration tests.

### Step-by-Step Execution
1. **Initialize crate**:
   - Create `<apps/website/api_v2/types/>` with `Cargo.toml` (`name = "website-api-types"`).
   - Register `"apps/website/api_v2/types"` in root `Cargo.toml` `[workspace] members`.
2. **Absorb `apps/website/shared/`**:
   - Move `apps/website/shared/is_http_url_cases.rs` into `<apps/website/api_v2/types/src/is_http_url_cases.rs>`.
   - Delete `apps/website/shared/` directory.
3. **Update xtask schema codegen**:
   - In `tools_v2/xtask/src/commands/generate/schema_types.rs`:
     - Update `API_SOURCE_DIR` or target output paths to point to `<apps/website/api_v2/types/src/generated/>`.
   - Run `cargo xtask schema codegen` and verify `cargo xtask ci verify-codegen-fresh`.
4. **Move canonical wire DTOs**:
   - Move hand-maintained DTOs (`events.rs`, `servers.rs`, `missions.rs`, etc.) into `types/src/`.
   - Export all types flatly from `types/src/lib.rs`.
5. **Update `website-api`**:
   - Add `website-api-types = { path = "types" }` to `apps/website/api_v2/Cargo.toml`.
   - Re-export or import wire types from `website_api_types`.
6. **Consolidate `api_v2/tests/` into domain test suites**:
   - Move 95 flat tests into subdirectories: `tests/operations/`, `tests/missions/`, `tests/auth/`, etc.
   - Introduce suite runner files (`operations_suite.rs`, etc.) to cut Cargo link steps.
7. **Verify Phase 2**:
   ```bash
   cargo check -p website-api-types
   cargo check -p website-api-types --target wasm32-unknown-unknown
   cargo test -p website-api-types
   cargo test -p website-api
   ```

---

## 4. Phase 3: Frontend Transport & Shell Migration

### Objective
Rename `core/api/` to `core/transport/`, extract `src/v2/shell/`, decouple tokens, and eliminate duplicate DTOs.

### Step-by-Step Execution
1. **Add crate dependency**:
   - In `apps/website/frontend/Cargo.toml`, add:
     ```toml
     website-api-types = { path = "../api_v2/types" }
     ```
2. **Decouple inverted dependencies**:
   - Move `DISABLED_GLYPH` and `HOVER_FILL` to `core/ui/tokens.rs`.
   - Add `register_logout_hook` to `core/auth/store.rs`; replace hardcoded call to editor purge.
3. **Extract `src/v2/shell/`**:
   - Move `layout.rs`, `sidebar.rs`, `top_nav.rs`, and `nav_config.rs` from `pages/navigation/` into
     `<apps/website/frontend/src/v2/shell/>`.
   - Update `main.rs` to import `AppLayout` from `crate::v2::shell::layout`.
4. **Rename `core/api` to `core/transport`**:
   - `git mv apps/website/frontend/src/v2/core/api apps/website/frontend/src/v2/core/transport`
5. **Delete duplicate DTOs & tests**:
   - Delete `<apps/website/frontend/src/v2/core/transport/dto/>` and its `tests/` directory.
   - In `core/transport/mod.rs`, re-export `website_api_types::*`.
6. **Rename `endpoints/` to `actions/`**:
   - Rename `core/transport/endpoints/` to `core/transport/actions/`.
7. **Provide zero-churn re-export in `apps/website/frontend/src/v2/core/mod.rs`**:
   ```rust
   pub mod auth;
   pub mod transport;
   // Backward-compatibility alias during phased migration:
   pub use transport as api;
   ```
8. **Verify Phase 3**:
   ```bash
   cargo fmt -p website-frontend --check
   cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets
   cargo test -p website-frontend
   cd apps/website/frontend && trunk build --release
   ```

---

## 5. Phase 4: Cruft Cleanup & Documentation Cross-References

### Objective
Remove empty legacy engine directories and update all developer documentation.

### Step-by-Step Execution
1. **Remove orphaned directory**:
   - Delete `frontend/src/v2/map_engine/` and all its empty subdirectories (`camera/`,
     `renderer/`, `terrain/`, `tools/`).
2. **Update developer runbooks**:
   - `documentation_v2/runbooks/local_development.md`: Reference `deploy/docker-compose.dev.yml`.
   - `documentation_v2/runbooks/website_deployment.md`: Reference `deploy/docker-compose.staging.yml`.
3. **Verify all gates**:
   ```bash
   cargo xtask verify markdown-placement
   cargo xtask verify readme-coverage
   cargo xtask verify link-check
   cargo xtask verify file-length
   cargo xtask verify engine-layers
   ```

---

## 6. Rollback Strategies

- **Phase 1 Rollback**: Revert the single commit modifying compose files and xtask pins atomically.
- **Phase 2 & 3 Rollback**: Because `pub use transport as api;` maintains backward-compatibility,
  reverting is cleanly localized to crate manifests and the `types/` sub-crate without affecting
  page rendering code.
