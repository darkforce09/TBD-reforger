# Equipment data viewer requests

The viewer's API reads: every panel request is bounded, can be cancelled, and caches its settled
answer per generation.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/data/
├── loading.rs        panel reads that abort when superseded, through the cache and `public_get`
├── mod.rs            the module tree
└── request_cache.rs  the least-recently-used page cache: 32 entries and 4 MiB of text at most
```

## Boundaries

- Depends on: `frontend_transport::client::public_reads::public_get`; `web_sys::AbortController`;
  `serde_json`.
- Used by: the tab components of `crates/frontend/workspaces/debug_benches/src/data_viewer/`.
- Rules: a read that a newer location supersedes is aborted and never overwrites the newer panel;
  `status` answers, `generation=latest` answers and bodies over 256 KiB are never cached, since
  they change between exports or would crowd the cache out.
