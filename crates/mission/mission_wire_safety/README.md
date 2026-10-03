# Mission wire safety

The `mission_wire_safety` crate: two scans of a [mission](/documentation/glossary/g_to_m.md#mission)
editor payload that the [API](/documentation/glossary/a_to_f.md#api) runs before it saves or compiles
one. The first finds authored names that would carry a control character into the compiled
document, which the schema's `wireSafeString` forbids; the second finds
[slot](/documentation/glossary/n_to_z.md#slot) cargo heavier or bulkier than the garment that holds it.

## Contents

```text
crates/mission/mission_wire_safety/
├── Cargo.toml  the package: `serde_json`, layout tier 0
└── src/        the byte rule, both scans, the prelude and the unit tests
```

## How it works

`is_wire_unsafe` is the `wireSafeString` rule, byte by byte: 0x00 to 0x1F and 0x7F.
`scan_editor_payload` checks each name that lands in the compiled document: a faction's `name`
(`factions[].displayName`), a squad's callsign, else its name, else its id (`groupCallsign` and the
slot id), a slot's `role` and a slot's `id` (`slots[].uid`). Each distinct bad value is one line
with its payload path, the value with its control characters escaped, the byte by name
(`TAB (U+0009)`) and where it lands; repeats of a value collapse into a count, and past
`MAX_REPORTED` (20) distinct values one tail line counts the rest.

`scan_cargo_capacity(payload, catalog)` adds up, per slot and per cargo container (`vest`, `pants`,
`jacket`, `backpack`), the catalogued weight and volume of the items in it and compares them with
the maximums of the garment worn there, `armoredVest` standing in for `vest`. A line names the
garment's `wear` row and ends with `CARGO_CAPACITY_CAVEAT`; items or garments the catalog lacks
count as nothing, and an empty catalog reports nothing.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_wire_safety   # the byte rule, the name scan and the capacity walk
```

## Configuration

None: no features and no environment variables. The caller supplies the `CargoPhysCatalog`, so the
crate reads no item [registry](/documentation/glossary/n_to_z.md#registry) of its own.

## Public surface

All also in `prelude`:

- `is_wire_unsafe(b: u8) -> bool`, `first_unsafe_byte(s: &str) -> Option<u8>`,
  `describe(b: u8) -> String` and `quote_value(s: &str) -> String`: the byte rule and the
  rendering of an offending value.
- `scan_editor_payload(payload: &Value) -> Vec<String>` and `MAX_REPORTED`.
- `scan_cargo_capacity(payload: &Value, catalog: &CargoPhysCatalog) -> Vec<String>`, `CargoPhys`
  (display name, weight, volume and the two maximums), `CargoPhysCatalog` (a `HashMap` from
  resource name to `CargoPhys`) and `CARGO_CAPACITY_CAVEAT`.

## Boundaries

- Depends on: `serde_json` only.
- Used by:
  - the API: `crates/api/api_missions/src/contract/schema_validators.rs` runs both scans after the
    payload schema on every save; `crates/api/api_missions/src/services/mission_compile.rs` refuses a
    compile on any capacity line; `crates/api/api_missions/src/services/cargo_catalog.rs` and
    `crates/api/api_missions/src/services/mission_artifacts/artifact_inputs.rs` build the catalog from
    the registry's rows; `crates/api/api_missions/src/contract/zone_quantisation.rs` caps its report
    at `MAX_REPORTED`;
  - `mission_compiler` (`crates/mission/mission_compiler/src/game_document/`: `is_wire_unsafe`
    for the identity keys it emits, `MAX_REPORTED` for the type-safety report) and
    `mission_validation` (`crates/mission/mission_validation/`: the `CARGO-OVER-CAPACITY` rule
    and its catalog fact).
- Rules: the byte rule catches exactly the schema's forbidden characters and nothing else
  (`byte_scan_equals_char_scan_over_the_schema_pattern` in `src/tests/cases_1.rs`); the slot id is
  scanned because the compile copies it into `uid`
  (`slot_id_is_scanned_because_flatten_copies_it_verbatim_into_uid`); reports stay bounded
  (`identical_bad_values_collapse_to_one_row_with_a_count`,
  `distinct_bad_values_are_capped_with_a_tail_line`); an empty catalog never invents a limit
  (`empty_catalog_never_invents_a_limit`); mission tier 0, so the crate depends on no workspace
  crate (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission crates](/crates/mission/README.md) — the category and its dependency rule.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
