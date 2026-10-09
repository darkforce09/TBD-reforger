# Contract crates

Shapes and policies that two programs must agree on, kept in one crate so both compile the same
decision instead of copies of it.

## Contents

```text
crates/contracts/
├── contract_schema_types/  `contract_schema_types`: the serde types generated from the contract JSON Schemas, one module per API domain
├── fleet_wire_contract/   `fleet_wire_contract`: the fleet command shapes and credential format the API and host agent share
└── offline_cache_policy/  `offline_cache_policy`: the caches, request classes and pack the worker and page share
```

## Boundaries

- Depends on: foundation crates and external crates only.
- Used by: the offline service worker and the single-page app (`offline_cache_policy`); the API,
  the game server host agent, `staging_fixtures` and `mod_operations` (`fleet_wire_contract`); the API and its contract tests
  (`contract_schema_types`).
- Rules: a contracts crate declares `category = "crates/contracts"` and depends on foundation
  crates only (`cargo xtask verify crate-tiers`); `contract_schema_types/src/generated/` is
  written by `cargo xtask schema codegen` and never edited by hand.
