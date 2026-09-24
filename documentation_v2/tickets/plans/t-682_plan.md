# T-682 — Plan

## Context

Fog, wind and view distance are refused by test until a mod reader exists: `author_env` (now `apps/website/frontend/src/editor/panels/env.rs`; `eden_chrome.rs` is gone) enforces reader-before-control. `ModEnvironment` in `flatten.rs:268-275` does not even serialise `windDirDeg`.

## Approach

1. Verify on main: `rg -n 'windDirDeg' crates/map-engine-core/src/mission/flatten.rs` shows no emit.
2. `flatten.rs`: serialise `windDirDeg`, fog and view distance in `ModEnvironment` (schema keys exist since T-706); `cargo test -p map-engine-core --all-features`.
3. `TBD_MissionLoader.c`: bind the environment fields; new `Backend/TBD_EnvironmentReader.c`: apply them at mission boot through the weather manager.
4. Leave the editor controls alone — the control half comes after this reader per `author_env`.

## Risks

- Weather-manager setters may be server-only; apply on the authority side and let replication carry it.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-682`
