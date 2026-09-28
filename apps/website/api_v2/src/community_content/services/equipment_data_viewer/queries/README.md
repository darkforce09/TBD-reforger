# Equipment dataset queries

The read queries behind the equipment data viewer routes. Each answers one page from one pinned
generation: the navigation comes from the generation's index, and every native value comes from
its source documents with its original JSON token.

## Contents

```text
apps/website/api_v2/src/community_content/services/equipment_data_viewer/queries/
├── fields.rs             the field inventory, and one field's occurrences across resources
├── mod.rs                `ViewerQuery`, the shared query string, and `page`, the byte-bounded page
├── native_matches.rs     accepting and providing classes paired by `native-matching.json`
├── relationships.rs      a resource's outgoing or incoming references, or its native-type matches
├── resource_cards.rs     source cards: a resource's containers in native order, with facts
├── resources.rs          the resource catalog, searched by identity, domain, class or field
├── selection.rs          a gameplay generation's field selection decisions and its receipt
├── source_inspection.rs  instance navigation, properties, values and document expansion
└── values.rs             exact JSON value entries and paged expansion below a JSON pointer
```

## How it works

Every query takes the `ViewerQuery` the handler decoded: `generation` (`latest` when absent) pins
the generation, `cursor` is the row offset of the next page (at most 100,000,000), `view` is
`effective` unless the caller asks for `ancestor` or `all`, and the rest (`q`, `domain`,
`capability`, `resource_id`, `node_id`, `property`, `field_id`, `direction`, `kind`, `document`,
`pointer`, `class_name`) narrow the query that reads them. `page` returns `generation_id`,
`total`, `next_cursor` and the items that fit 2 KiB below `PAGE_BYTES`; it drops rows from the
end, never the continuation, and refuses a single row that does not fit, which the caller then
reads through document expansion.

`relationships` answers the native-type matches instead of references when `kind` is
`native_type_match`, through `native_matches.rs`, which embeds
`contracts_v2/rules/equipment-gameplay/native-matching.json`. `selection` applies only to a
gameplay generation, whose selection report and receipt it reads. The queries that read source
documents (`selection`, `source_inspection`, `resource_cards`) hold one of the service's two
reader permits while they read, and parse on a blocking thread. `values.rs` keeps a number's
original text and every array entry, so an expanded value is byte-faithful to the export.

## Boundaries

- Depends on: the `Dataset` and `EquipmentDataService` of `service_state.rs` (index pool, manifest,
  definitions, document cache, reader permits), the source readers in `source/`, and the rule file
  `contracts_v2/rules/equipment-gameplay/native-matching.json`, embedded at compile time.
- Used by: the handlers in
  `apps/website/api_v2/src/community_content/handlers/equipment_data_viewer/`, which re-read every
  answer as its generated contract type.
- Rules: a query reads one generation and nothing else; every answer passes through `page` or
  keeps under `PAGE_BYTES`; a value is never coerced or truncated (`tests/value_expansion.rs` and
  `tests/resource_cards.rs` of the parent folder).
