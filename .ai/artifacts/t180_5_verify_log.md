# T-180.5 verify log — Eden side chips + Objects stub

**Date:** 2026-07-19  
**Slice:** [`t180_5_right_dock_side_chips.md`](../../docs/specs/Mission_Creator_Architecture/t180_5_right_dock_side_chips.md)  
**Tag:** `T-180.5`

## Shipped

- DockRight Eden chip row above search: **BLUFOR / OPFOR / INDFOR / Objects** (Aegis `primary` / `error-alert` / `success` / `tactical-yellow`)
- `apply_eden_chip` writes the same `active_side` signal OpsCtx / `place_at` read
- `objects_mode` on OpsCtx — Objects empty-state `"Objects coming soon…"`; `begin_place` / `place_at` no-op
- No F1–F6 mode row · no CIV chip · catalog **not** filtered by side (E-L2b)

## E-L2b note

Registry character rows lack Eden side tags usable for palette filter. Catalog stays unfiltered; **E4 place side** (`active_side` → `place_character_under_side` → `faction-OPFOR`) is the hard gate.

## Gates

### E1 / E2 / E3 / E5 — FE unit

```text
$ cargo test -p website-frontend
test eden_chrome::tests::eden_side_chips_labels_no_civ ... ok
test eden_chrome::tests::apply_eden_chip_opfor_sets_active_side ... ok
test eden_chrome::tests::objects_chip_empty_copy_and_mode ... ok
test result: ok. 77 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### E4 — core place OPFOR

```text
$ cargo test -p map-engine-core --features doc place_character_under_side_opfor
test doc::place_orbat::tests::place_character_under_side_opfor ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 133 filtered out
```

### E1 — no F-key palette row (UI)

```text
$ rg -n 'F1|F2|F3|F4|F5|F6' apps/website/frontend/src/eden_chrome.rs | head
```

Matches are **comments + unit-test ban asserts only** — no F1–F6 mode row / buttons in DockRight.

### `make ci-local-leptos`

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
  (pre-existing warnings only; no new errors)
cargo test -p website-frontend                 PASS (77)
trunk build --release                          PASS
  (make recipe / ambient NO_COLOR=1 → trunk `--no-color` quirk;
   `env -u NO_COLOR -u FORCE_COLOR trunk build --release` ✅)
```

## Locked pins (asserted)

| Pin | Value |
|-----|-------|
| `EDEN_SIDE_CHIPS` | `["BLUFOR","OPFOR","INDFOR","Objects"]` |
| `OBJECTS_COMING_SOON` | `"Objects coming soon…"` |
| Default `active_side` | `BLUFOR` |
| Objects place | `begin_place` / `place_at` no-op |

## Manual

| ID | Status |
|----|--------|
| M-E1 | Operator: eye-pass vs `eden_side_chips_ref.png` (chips + search; ignore F1–F6 / CIV) |
| M-E2 | Operator: OPFOR chip → place → red ring (.3) + squad links (.4) if multi |

## Ready for T-180.6

Yes — dock chips drive `active_side`; next is refile sync.
