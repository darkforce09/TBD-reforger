# T-180.9 verify log — Open Arsenal + orbat[] loadout truth

**Date:** 2026-07-19  
**Slice:** [`t180_9_arsenal_compile.md`](../../docs/specs/Mission_Creator_Architecture/t180_9_arsenal_compile.md)  
**Tag:** `T-180.9`

## Shipped

- [`crates/map-engine-core/src/mission/orbat.rs`](../../crates/map-engine-core/src/mission/orbat.rs) — `Sl.loadout: Option<Value>`; `loadout_summary_from_value` (summary → else primary+launcher `" + "` + basename strip `.et`); derive map uses helper (no `String::new()` hardcode)
- [`crates/map-engine-core/src/mission/compile.rs`](../../crates/map-engine-core/src/mission/compile.rs) — `compile_export_orbat_loadout` (I6); sort golden empty loadouts kept (slots lack loadout)
- [`apps/website/frontend/src/editor_ops.rs`](../../apps/website/frontend/src/editor_ops.rs) — `OpsCtx.attrs_tab`; `open_arsenal` sets tab **3** + `attrs_open`
- [`apps/website/frontend/src/attributes.rs`](../../apps/website/frontend/src/attributes.rs) / [`mission_editor.rs`](../../apps/website/frontend/src/mission_editor.rs) — lift tab signal
- [`apps/website/frontend/src/orbat_manager.rs`](../../apps/website/frontend/src/orbat_manager.rs) — OPEN ARSENAL → `open_arsenal`; I7 test

## Gates

### I1 / I2 — derive fill

```text
$ cargo test -p map-engine-core --features mission --lib derive_fills_loadout
test mission::orbat::tests::derive_fills_loadout_from_summary ... ok
test mission::orbat::tests::derive_fills_loadout_from_weapons ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 131 filtered out
```

### I3 / I4

```text
$ cargo test -p map-engine-core --features mission --lib derive_empty_loadout
test mission::orbat::tests::derive_empty_loadout_when_absent ... ok

$ cargo test -p map-engine-core --features mission --lib derives_from_editor_sorted
test mission::orbat::tests::derives_from_editor_sorted_by_index ... ok
```

### I5 / I9

```text
$ rg -n 'loadout: String::new\(\)' crates/map-engine-core/src/mission/orbat.rs
# (no matches) → I5 PASS

$ rg -n 'loadout\.is_empty' crates/map-engine-core/src/mission/orbat.rs
# (no matches — empty-all assert removed) → I9 PASS
```

### I6 — Export compile

```text
$ cargo test -p map-engine-core --features mission --lib compile_export_orbat_loadout
test mission::compile::tests::compile_export_orbat_loadout ... ok
```

### Lib suite

```text
$ cargo test -p map-engine-core --features mission --lib
test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### I7 — Open Arsenal tab 3

```text
$ cargo test -p website-frontend open_arsenal_selects_arsenal_tab
test orbat_manager::tests::open_arsenal_selects_arsenal_tab ... ok
```

### I8 — No Standardization

```text
$ rg -ni 'standardization|IFAK|Grenade Complement' \
    apps/website/frontend/src/orbat_manager.rs \
    apps/website/frontend/src/eden_chrome.rs
# (no matches) → I8 PASS
```

### `make test-it`

PASS (includes `editor_only_orbat_derivation`).

### `make ci-local-leptos`

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
cargo test -p website-frontend                 PASS (89)
trunk build --release                          PASS
```

## Manual (operator)

| ID | Check |
|----|-------|
| M-I1 | Edit Arsenal on a slot → Save Version → Event attach / GET orbat shows loadout text |
| M-I2 | ORBAT Manager → OPEN ARSENAL → same picks as Attributes Arsenal for that slot |
| M-I3 | Export JSON `orbat[].slots[].loadout` non-empty for geared slots |

## Ready for

**Cursor doc sync** — T-180 program complete (T-180.9 last code slice).
