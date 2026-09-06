# T-935.14 — the prefab archive needs a residency reader

## Why this exists

T-935.11 shipped `objects/prefabs.rkyv` and a checked reader for it
(`PrefabCatalog::catalog_from_bytes`, everon-pinned, terrain- and census-verified). It did **not**
add a frontend fetch for it, and that omission is correct rather than lazy: the SPA has no code that
could consume the result.

Measured on main after T-935.11 landed:

```
$ grep -rn 'load_prefabs' apps/website/frontend/src/
apps/website/frontend/src/editor/world_assets/world_host.rs:174:  let _ = self.residency.load_prefabs_gz(&bytes);
apps/website/frontend/src/pages/debug/world_los.rs:358:          .load_prefabs_gz(&prefabs)
```

Both consumers go through `WorldResidency::load_prefabs_gz`, which takes gzipped JSON and derives
every downstream lookup from a `serde_json::Value`: `building_prefab_lookup`, `fence_prefab_lookup`
and `rebuild_glyph_lookup_from_prefabs`. The private maps those feed (`prefab_by_id`,
`building_by_u16`, `fence_by_u16`) have no other writer. A `world_host` fetch of the archive would
therefore hand its bytes to nothing — the dead mechanism the slice briefs forbid.

So the archive is shipped, verified and unreachable. This ticket makes it reachable.

## The work

`WorldResidency::load_prefabs(&[u8])` — one public entry that sniffs gzip versus rkyv the way
`store.rs`'s road loader already does, delegates the JSON arm to today's path unchanged, and
delegates the rkyv arm to `catalog_from_bytes` plus archive-shaped twins of the two lookup builders.
Then `world_host` picks `objects/prefabs.rkyv` when `manifest.objects.binary` names it, exactly as
it now does for roads and regions.

`residency.rs` is SIZE-3 allowlisted, so per the file-length rule the new logic goes in a new file
(or in `prefab.rs`, beside the reader it calls) and `residency.rs` takes only the call site — the
same split T-935.3 used for `chunk_bin.rs`.

## Acceptance

The archive lane and the JSON lane produce an identical `WorldResidency` for everon: same prefab
rows in the same order, same building and fence lookups, same glyph table. JSON stays the default
until T-935.13 flips it.
