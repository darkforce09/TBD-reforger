# Workspace Task Runner (`tools_v2/xtask`)

## Phase-two implementation

The task runner delegates blueprint and map verification through `commands/map` and `verifications/map_assets` to `developer-tools`. Its manifest has no direct map-engine dependency. Execution-receipt fixtures remain in `tests/fixtures`; blueprint fixtures belong to `developer-tools/test_fixtures/blueprint`. Ticket orchestration and most flat modules remain here until phases three and four.

## Target architecture

Lightweight, declarative command-line task dispatcher (`cargo xtask ...`) and repository integrity verification suite.

Relocated from root `xtask/`. Stripped of all heavy 3D CAD mesh raymarching (moved to `developer-tools`) and raw ticket database logic (moved to `ticket-engine`).

---

## 1. Directory Structure

```text
tools_v2/xtask/
├── Cargo.toml                           <-- Lean manifest: verification-core, ticket-engine, developer-tools
├── test_fixtures/                       <-- Golden test assets & JSON receipts (renamed from tests/)
└── src/
    ├── main.rs                          <-- Declarative CLI entrypoint (<150 LOC)
    ├── core/                            <-- Shared root discovery, constants, hostrun bridge
    ├── commands/                        <-- Domain CLI command handlers (dispatch & orchestration)
    │   ├── ticket/                      <-- Thin forwarder delegating to ticket-engine
    │   ├── wave/                        <-- Thin forwarder delegating to ticket-engine wave_lock
    │   ├── mod_ops/                     <-- Mod compilation, playtest server, dev server
    │   ├── db/                          <-- PostgreSQL container lifecycle & test migrations
    │   ├── deploy/                      <-- Staging and production deployment drivers
    │   ├── map/                         <-- Thin forwarder delegating to developer-tools
    │   ├── ci/                          <-- CI task suite and test harness runner
    │   ├── setup/                       <-- Linux Workbench and client addon symlinking
    │   ├── fetch/                       <-- Upstream Bohemia script & API scrapers
    │   ├── mcp/                         <-- Workbench NetAPI tool call forwarders
    │   ├── mk/                          <-- Makefile emulation targets
    │   ├── debug/                       <-- Network A2S probes & direct-join diagnostics
    │   ├── repro/                       <-- Mission upload HTTP payload reproducers
    │   └── ai/                          <-- Token guards and agent output filtering
    └── verifications/                   <-- Categorized repository assertions (formerly "gates")
        ├── architecture/                <-- Engine layer isolation (Law 6) & route parity
        ├── licensing/                   <-- Upstream APL & third-party code leak guards
        ├── language_bans/               <-- LANG-1 (no-shell), LANG-2 (no-python), LANG-3 (no-node)
        ├── database/                    <-- SQL safety & database seed integrity
        ├── deployment/                  <-- Staging compose file path pins
        ├── ci/                          <-- CI workflow script hygiene
        ├── registry/                    <-- Object palette ↔ spawn registry census
        ├── mod_scripts/                 <-- Script compilation, layout parsing, determinism
        ├── map_assets/                  <-- Semantic object golden gates & label declutter
        └── schemas/                     <-- Decomposed schema verification gates (<500 LOC)
```

---

## 2. Invariants & Responsibilities

1. **Lightweight & Fast-Compiling**: Contains zero heavy 3D graphics or voxel dependencies. Modifying game assets or 3D meshes will not invalidate the `xtask` build cache.
2. **Declarative CLI Router (<150 LOC)**: `main.rs` contains solely the top-level Clap enum and delegates execution to domain modules.
3. **No Inline Tests**: Unit tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>_tests.rs"] mod tests;`.
