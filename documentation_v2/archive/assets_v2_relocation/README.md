**Status:** live

# Asset relocation records

The records of the move of the map assets into `assets_v2/`: the census of every committed asset,
the storage and streaming plan, and the handoff that maps each old path to its new one. Status:
archived — frozen records.

## Contents

```text
documentation_v2/archive/assets_v2_relocation/
├── analysis_and_inventory.md  census of every committed asset: size, encoding, reader
├── architecture_plan.md       how terrain data is partitioned, served, cached and versioned
└── migration_handoff.md       where each asset path went, and what stayed byte-identical
```

## Code

- [Assets](/assets_v2/) — the tree the move created: terrains, glyphs, scratch and the storage
  specification.

## Boundaries

- Depends on: nothing live; the records quote the tree of their time.
- Used by: the documentation program's own records at the documentation root and nothing else.
- Rules: never reworded, only links change.

## Related documentation

- [Asset documentation](/documentation_v2/assets_v2/README.md) — the live description of the
  assets, the terrain export and the uploaded-terrain volume.
- [Assets README](/assets_v2/README.md) — the tree as it is.
