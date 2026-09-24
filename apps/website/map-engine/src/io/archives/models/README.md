# Archive model facade

One flat import point for every rkyv archive type the map data uses: the roads, map labels,
water vectors, prefab catalogue, forest regions, building blueprints and satellite index, with
their generated `Archived…` and `…Resolver` types, and `ARCHIVE_SCHEMA_VERSION`. It declares
nothing of its own and hosts the archives' round-trip tests.

## Contents

```text
apps/website/map-engine/src/io/archives/models/
├── mod.rs  the module tree; re-exports every archive type and `ARCHIVE_SCHEMA_VERSION`
└── tests/  unit tests: each archive round-trips, rejects corruption and rejects another type
```

## Boundaries

- Depends on: `crate::io::archives::version`, `roads`, `labels`, `water`, `prefabs`, `forest`,
  `blueprints` and `satellite`, whose items it re-exports; the tests also use
  `crate::io::archives::codec` and `crate::io::containers::tbds`.
- Used by: nothing outside the folder; readers and writers import the defining module
  (`crate::io::archives::roads` and the rest).
- Rules: a re-export only; the tests pin the archive contract: each archive type serialises with
  `to_bytes`, reads back equal through `access_checked`, and is refused when its bytes are
  corrupted or when they hold another archive type; an empty archive round-trips; and a `TBDS`
  version 2 header frames a satellite index that still validates (`tests/cases_1.rs`).
