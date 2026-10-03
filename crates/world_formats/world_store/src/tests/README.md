# World store tests

Unit tests of the world store: its prefab, chunk and manifest loads, its road lanes, its refusal
of truncated payloads and the census of the committed Everon export.

## Contents

```text
crates/world_formats/world_store/src/tests/
└── store_tests.rs  the loads, the road sniff and its refusals, the Everon census
```

## Boundaries

- Depends on: `crate::store`, `prefab_catalog`'s test fixtures (gzip), `road_network`'s class
  codec and `world_file_formats`' road archive codec and density grid decoder.
- Used by: `cargo test -p world_store`.
- Rules: the cases keep their assertions and fixtures; the census reads
  `assets/terrains/everon/manifest.json` and `assets/terrains/everon/objects/`.
