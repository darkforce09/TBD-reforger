# T-180.3 verify log — Map side tint

**Date:** 2026-07-19  
**Slice:** [`t180_3_map_side_tint.md`](../../docs/specs/Mission_Creator_Architecture/t180_3_map_side_tint.md)  
**Tag:** `T-180.3`

## Shipped

- `SIDE_BLUFOR_RGBA` / `SIDE_OPFOR_RGBA` / `SIDE_INDFOR_RGBA` + `side_rgba` / `pack_rings` / `pack_slot_instances(…, side_tints)` / `unselected_row_patch_for`
- `SlotSoa.side_keys` via materialize: slot → squad.`factionId` → faction.`key` (missing → `BLUFOR`)
- Engine `slots_bind_soa(ids, xy, side_tints_rgba: &[u8])` caches `last_side_tints`; O(delta) deselect restores side tint
- Leptos / wasm pass `side_tints_rgba_bytes(&soa.side_keys)` into bind

## Gates

### C1 — `side_tint_three_distinct`

```text
$ cargo test -p map-engine-core side_tint_three_distinct
test slots_gpu::tests::side_tint_three_distinct ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out
```

### C2 — `selected_overrides_side_tint`

```text
$ cargo test -p map-engine-core selected_overrides_side_tint
test slots_gpu::tests::selected_overrides_side_tint ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out
```

### C3 — `pack_rings_side_tints`

```text
$ cargo test -p map-engine-core pack_rings_side_tints
test slots_gpu::tests::pack_rings_side_tints ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out
```

### C4 — `missing_side_defaults_blufor`

```text
$ cargo test -p map-engine-core missing_side_defaults_blufor
test slots_gpu::tests::missing_side_defaults_blufor ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out
```

### Frontend unit tests

```text
$ cargo test -p website-frontend
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Note: package is a bin crate — `--lib` is N/A; `make ci-local-leptos` uses `cargo test -p website-frontend`.

### `make ci-local-leptos` (clippy + test + trunk)

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
  (pre-existing warnings only; no new errors)
cargo test -p website-frontend                 PASS (74)
trunk build --release                          PASS
  (make recipe hit env `--no-color=1` quirk; direct `trunk build --release` ✅)
```

## Locked RGBA (asserted)

| Side | RGBA |
|------|------|
| BLUFOR | `[173, 198, 255, 255]` |
| OPFOR | `[248, 113, 113, 255]` |
| INDFOR | `[34, 197, 94, 255]` |
| Selected | `[250, 204, 21, 255]` |

## Manual

| ID | Status |
|----|--------|
| M-C1 | Operator: place one unit per side → three ring colors; select → yellow; deselect → side color |

## Ready for T-180.4

Yes — squad leader→member hairlines next.
