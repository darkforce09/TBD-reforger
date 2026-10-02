# Contract crates

Shapes and policies that two programs must agree on, kept in one crate so both compile the same
decision instead of copies of it.

## Contents

```text
crates/contracts/
└── offline_cache_policy/  `offline_cache_policy`: the caches, request classes and pack the worker and page share
```

## Boundaries

- Depends on: foundation crates and external crates only.
- Used by: the offline service worker and the single-page app.
- Rules: a contracts crate declares `category = "crates/contracts"` and depends on foundation
  crates only (`cargo xtask verify crate-tiers`).
