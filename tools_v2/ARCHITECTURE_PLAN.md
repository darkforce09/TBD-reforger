# Unified Tooling Architecture Blueprint & Execution Plan

Comprehensive technical specification and phased migration roadmap for transitioning from the legacy tooling layout to the unified `tools_v2/` architecture.

## Current phase-three layout

All four `tools_v2` crates are live. `developer-tools` owns the heavy CLI, blueprint compiler, shared PAK reader, and engine-backed map verifications. The six executable names remain unchanged. `xtask` delegates these operations without a direct map-engine dependency. See [PHASE_TWO_HANDOFF.md](./PHASE_TWO_HANDOFF.md) for validation.

Ticket consolidation is implemented: `ticket-engine` owns validation, command services, generated views, compatibility readers, maintenance, wave scheduling/history, and metrics. `xtask` retains platform execution and delegates ticket behavior. See [PHASE_THREE_HANDOFF.md](./PHASE_THREE_HANDOFF.md) for validation. Broad `xtask` and `developer-tools` decomposition remains phase four.

---

## 1. Context & Motivation

Prior to this reorganization, the workspace tooling was divided across three uncoordinated locations:
1. `crates/`: A root directory containing only two crates (`tbd-gate` and `tbd-tickets`).
2. `xtask/`: A root directory holding a 109-item flat dump of mixed task commands, ticket validation rules, markdown view synchronizers, and a 13.4k LOC 3D mesh CAD compiler.
3. `tools/`: A collection containing `tbd-tools` (a heavy async CLI), `editor-capture` (an obsolete README), and `pbo` (orphaned Python `.pyc` bytecode).

### Core Problem: Overloaded "Gate" Jargon
The word "gate" was overloaded across three completely distinct systems:
- An assertion library checking file text patterns (`tbd-gate`).
- A headless Chrome browser runner driving editor interactions over CDP (`tools/tbd-tools/src/bin/gate.rs`).
- Repository integrity checks and operational scripts in `xtask` (`xtask/src/gate_*.rs`).

This reorganization eliminates the ambiguity by renaming every component strictly by its concrete domain function under **Law 4 (Zero Context Needed)**.

---

## 2. Monorepo Integration & Cross-Cutting Changes

### 2.1 Workspace Manifest (`Cargo.toml`)
Root `Cargo.toml` updates its workspace `members` array:
```toml
[workspace]
resolver = "3"
members = [
    "apps/ticketboard",
    "apps/website/api_v2",
    "apps/website/frontend",
    "apps/website/map-engine",
    "apps/website/graphics-engine",
    "tools_v2/verification-core",
    "tools_v2/ticket-engine",
    "tools_v2/developer-tools",
    "tools_v2/xtask",
]
```
Root `crates/` is emptied and completely deleted.

### 2.2 Cargo Task Alias (`.cargo/config.toml`)
`.cargo/config.toml` specifies:
```toml
[alias]
xtask = "run --package xtask --"
```
Because Cargo resolves `--package xtask` by the `package.name` declared in `tools_v2/xtask/Cargo.toml`, `cargo xtask ...` commands continue to function identically with zero developer workflow interruption.

### 2.3 Inter-Crate Dependencies & Path Pins
1. **`apps/ticketboard/Cargo.toml`**:
   - `ticket-engine = { path = "../../tools_v2/ticket-engine" }`
2. **`tools_v2/xtask/Cargo.toml`**:
   - `verification-core = { path = "../verification-core" }`
   - `ticket-engine = { path = "../ticket-engine" }`
   - `developer-tools = { path = "../developer-tools" }`
   - Direct `website-map-engine` 3D mesh dependencies are dropped, substantially improving `xtask` compilation times.
3. **`schema_gates.rs` (now `tools_v2/xtask/src/verifications/schemas/`):**
   - Update `SCAN_ROOTS`: `const SCAN_ROOTS: [&str; 4] = ["apps", "packages", "tools", "tools_v2"];` (replacing `"crates"`).
4. **`node_free.rs` (now `tools_v2/xtask/src/verifications/language_bans/no_node_scripts.rs`):**
   - Update `FILE_LENGTH_PINS` from `["xtask", "tools", "crates", ...]` to `["tools_v2/xtask", "tools_v2/verification-core", "tools_v2/ticket-engine", "tools_v2/developer-tools", ...]`.
5. **`mk_ci_tests.rs`:**
   - Update `root()` path resolution: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap()` to cleanly resolve repository root from `tools_v2/xtask/`.

---

## 3. Four-Crate Architectural Breakdown

```text
tools_v2/
├── verification-core/                   <-- Static assertion engine (Verdict, flock, process isolation)
├── ticket-engine/                       <-- Ticket database, schema validation, view sync, wave.lock
├── developer-tools/                     <-- Heavy async CLI suite, CDP browser runner & asset compiler
└── xtask/                               <-- Declarative task dispatcher (<150 LOC) & repo verifications
```

### 3.1 `verification-core`
- **Identity:** Fail-closed, dependency-light static analysis assertion library.
- **Responsibilities:**
  - Defines the non-collapsing four-outcome `Verdict` enum: `Held`, `Failed(Finding)`, `DidNotRun(NotRun, Finding)`.
  - Fail-closed text scanning and multi-line regex matching.
  - Process execution with process group signal isolation (`libc::killpg`) and concurrent I/O pipe drains.
  - Repository-level flocking (`target/.tbd-gate.lock`) to serialize concurrent CI runs.
- **Invariants:**
  - Zero internal monorepo dependencies.
  - Never implements `From<bool>` for `Verdict` (preventing "did not run" from collapsing into "passed").

### 3.2 `ticket-engine`
- **Identity:** Complete, self-contained ticket domain database and validation engine.
- **Responsibilities:**
  - Strong domain typing for tickets: `Domain`, `ScopeV2`, `Status`, `Ticket`.
  - Canonical-order TOML serialization/deserialization for `.ai/tickets/T-*.toml`.
  - Transactional mutation engine with pre-commit post-image validation and atomic file swapping.
  - Complete schema, scope surface, character limit, and ship gate validation (formerly `xtask/src/check.rs`).
  - Markdown lead/queue view generation and roadmap marker synchronization (formerly `xtask/src/sync.rs`).
  - The `wave.lock` dependency DAG compiler and drift verification (formerly `xtask/src/wave_lock.rs`).
  - Run metrics calculation, token estimates, and elapsed timing.
  - High-level ticket CLI operations (formerly `xtask/src/cmds.rs`).
- **Invariants:**
  - Zero internal monorepo dependencies.
  - Owns all logic touching `.ai/tickets/`.

### 3.3 `developer-tools`
- **Identity:** Heavy async CLI suite, asset compiler, and browser automation test harness.
- **Responsibilities:**
  - **6 Dedicated Binaries** (`src/bin/`):
    - `browser_test_runner`: Headless Chrome DevTools Protocol test runner.
    - `screen_capture`: ANGLE/Vulkan editor canvas snapshot capture.
    - `enfusion_oracle`: Enfusion script reverse engineering and symbol indexer.
    - `mcp_broker`: Persistent AF_UNIX socket broker for Workbench NetAPI.
    - `map_pipeline`: 2D satellite orthophoto, water detection, and cartographic map generator.
    - `world_pipeline`: 512m chunk spatial partitioning, road networks, and macro terrain export.
  - **3D Blueprint Compiler** (`src/blueprint/`):
    - Relocated from `xtask/src/map_blueprint/`.
    - Parses Enfusion `.xob` 3D meshes, raymarches voxel grids, extracts walls/slabs/plates/roofs, and compiles 3D BVH collision acceleration trees.
  - **Unified Enfusion Archive Reader** (`src/enfusion_pak/`):
    - Unifies the duplicate `.pak` archive parsers previously split between `tbd-tools` and `map_blueprint`.
- **Invariants:**
  - All production files <500 LOC; test files <1,000 LOC.
  - Modularized smoke tests: decomposes the 4,071 LOC `smokes.rs` into focused submodules.

### 3.4 `xtask`
- **Identity:** Lightweight command-line router and repository verification harness.
- **Responsibilities:**
  - Declarative CLI router (<150 LOC `main.rs`) dispatching commands to domain modules.
  - Thin command forwarders (`commands/ticket/`, `commands/wave/`, `commands/map/`) delegating to `ticket-engine` and `developer-tools`.
  - Local database and container lifecycle (`commands/db/`).
  - Staging and website deployment orchestration (`commands/deploy/`).
  - Categorized repository verification gates (`verifications/`).
- **Invariants:**
  - Carries zero heavy 3D engine or voxel raymarching dependencies.
  - Does not duplicate ticket storage or mutation logic.

---

## 4. Phased Migration & Rollout Plan

To ensure zero downtime and prevent interference with parallel engine work on `main`, execution follows four sequential phases:

### Phase 1: Crate Relocation & Root Cleanliness
1. Move `crates/tbd-gate` to `tools_v2/verification-core` (update `[package] name = "verification-core"`).
2. Move `crates/tbd-tickets` to `tools_v2/ticket-engine` (update `[package] name = "ticket-engine"`).
3. Move `xtask/` to `tools_v2/xtask/`.
4. Delete the emptied root `crates/` directory.
5. Update root `Cargo.toml` and `apps/ticketboard/Cargo.toml` paths.
6. Delete dead directories: `tools/pbo/` and `tools/editor-capture/` (migrating capture documentation to `docs/tools/editor_capture.md`).
7. Update path pins in `node_free.rs`, `schema_gates.rs`, and `mk_ci_tests.rs`.
8. **Gate**: `cargo check --workspace` passes cleanly; `cargo xtask --help` functions.

### Phase 2: Relocate the Blueprint Compiler and Heavy Tooling

1. Activate `tools_v2/developer-tools` from the live heavy-tooling crate, updating package selectors and imports while retaining the six executable names.
2. Move the complete blueprint compiler, ingestion, parity reporting, and associated fixtures into that crate.
3. Consolidate both PAK readers behind shared bounded parsing and payload code with explicit caller policies.
4. Move engine-backed map goldens, label checks, terrain-manifest checks, BLAS validation, and world-LOS verification into `developer_tools::map_verification`.
5. Route existing commands through thin adapters and remove `xtask`'s direct map-engine dependency.
6. Preserve existing module-size exemptions without extending their expiry dates; new consolidated modules and adapters follow the file-size/test-placement rules.
7. **Gate**: complete tooling test suites retain the baseline results, blueprint/PAK regression suites pass, fixtures and dependency versions remain unchanged, CLI routes work, and citation/file-length checks pass.

### Phase 3: Consolidate Ticket Subsystem into `ticket-engine`
1. Relocate `tools_v2/xtask/src/check.rs` (ticket validation) into `tools_v2/ticket-engine/src/validation/` (split <500 LOC, tests extracted).
2. Relocate `tools_v2/xtask/src/cmds.rs` (ticket operations) into `tools_v2/ticket-engine/src/cli/` (split <500 LOC, tests extracted).
3. Relocate `tools_v2/xtask/src/sync.rs` (markdown views) into `tools_v2/ticket-engine/src/sync/` (split <500 LOC).
4. Relocate `tools_v2/xtask/src/wave_lock.rs` into `tools_v2/ticket-engine/src/wave_lock/` (split <500 LOC, tests extracted).
5. Relocate `tools_v2/xtask/src/metrics.rs` into `tools_v2/ticket-engine/src/metrics/`.
6. Update `tools_v2/xtask/src/commands/ticket/` and `commands/wave/` to be thin delegators calling `ticket-engine`.
7. **Gate**: `cargo xtask ticket check` and `cargo xtask ticket sync` run with zero behavioral drift.

### Phase 4: `xtask` & `developer-tools` File Decomposition
1. Rewrite `tools_v2/xtask/src/main.rs` into the declarative router (<150 LOC).
2. Decompose `schema_gates.rs` (4,764 LOC) into `tools_v2/xtask/src/verifications/schemas/` (<500 LOC per file).
3. Rename all 9 `gate_t*.rs` files in `xtask` to self-describing domain verification files.
4. Move operational scripts misnamed as gates to `tools_v2/xtask/src/commands/`.
5. Decompose `smokes.rs` (4,071 LOC) in `developer-tools` into `browser_testing/editor_smoke_tests/` (<450 LOC each).
6. Decompose `aux.rs` (1,644 LOC) in `developer-tools` into 5 focused modules.
7. Rename cryptic modules in `developer-tools` (`sap.rs` → `aerial_orthophoto/`, `tbds_v2.rs` → `satellite_archive_container.rs`, `edds.rs` → `enfusion_texture_decoder.rs`, `jsval.rs` → `json_number_formatting.rs`).
8. Decompose `build.rs`, `gates.rs`, and `forest_smooth.rs` to satisfy the 500 LOC production limit.
9. Extract inline tests into sibling test files across both crates.
10. **Gate**: Full CI check suite (`cargo xtask ci ci-local`) passes with 100% green status.
