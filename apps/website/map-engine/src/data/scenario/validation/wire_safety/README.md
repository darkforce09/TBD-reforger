# Wire-safe names and cargo capacity

Two scans of a [mission](/documentation_v2/glossary/g_to_m.md#mission) editor payload that the
[API](/documentation_v2/glossary/a_to_f.md#api) runs before it saves or compiles one: authored names that
would carry a control character into the compiled document, which the schema's `wireSafeString`
forbids, and [slot](/documentation_v2/glossary/n_to_z.md#slot) cargo heavier or bulkier than the garment
that holds it. The module is exposed as `data::scenario::wire_safety`.

## Contents

```text
apps/website/map-engine/src/data/scenario/validation/wire_safety/
├── mod.rs   the module tree; re-exports the byte checks, both scans and the cargo catalog types
├── scan.rs  the control-character scan of authored names and the cargo capacity scan
└── tests/   unit tests for the byte rule, the reporting caps and the capacity walk
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

## Boundaries

- Depends on: `serde_json` only; the caller supplies the `CargoPhysCatalog`, so the module reads
  no item [registry](/documentation_v2/glossary/n_to_z.md#registry) of its own.
- Used by:
  - the API: `apps/website/api_v2/src/missions/contract/schema_validators.rs` runs both scans
    after the payload schema on every save;
    `apps/website/api_v2/src/missions/services/mission_compile.rs` refuses a compile on any
    capacity line; `apps/website/api_v2/src/missions/services/cargo_catalog.rs` and
    `apps/website/api_v2/src/missions/services/mission_artifacts/artifact_inputs.rs` build the
    catalog from the registry's rows;
  - `crate::data::scenario::flatten` (`is_wire_unsafe` for the identity keys it emits) and
    `crate::data::scenario::validate` (the `CARGO-OVER-CAPACITY` rule and its catalog fact).
- Rules: the byte rule catches exactly the schema's forbidden characters and nothing else
  (`byte_scan_equals_char_scan_over_the_schema_pattern` in `tests/cases_1.rs`); the slot id is
  scanned because the compile copies it into `uid`
  (`slot_id_is_scanned_because_flatten_copies_it_verbatim_into_uid`); reports stay bounded
  (`identical_bad_values_collapse_to_one_row_with_a_count`,
  `distinct_bad_values_are_capped_with_a_tail_line`); an empty catalog never invents a limit
  (`empty_catalog_never_invents_a_limit`).
