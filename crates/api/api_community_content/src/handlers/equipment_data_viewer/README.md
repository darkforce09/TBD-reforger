# Equipment data viewer handlers

The development-only debug reads behind the equipment data viewer: anonymous, generation-pinned
views of the equipment datasets the Workbench export produces, served through the normal API
middleware.

## Contents

```text
crates/api/api_community_content/src/handlers/equipment_data_viewer/
├── dataset.rs            `status` and `overview`: dataset availability and the pinned generation's summary
├── downloads.rs          `download`: one complete original document of the pinned generation, streamed
├── mod.rs                the module tree, the failure and query-rejection envelopes, and the page-size check every JSON answer passes
├── resources.rs          `resources`, `relationships` and `fields`: the resource, relationship and native-field catalogs
└── source_inspection.rs  `resource-cards`, `selection`, `containers`, `properties`, `values` and `documents`: bounded source pages
```

## How it works

The domain's `routes.rs` registers the twelve routes under `/api/v1/debug/equipment-data/` only
when `dev` is true (`Config::is_development`), so a production router answers 404 for every one of
them. None takes an auth extractor.

| Route | Handler |
|---|---|
| `GET /debug/equipment-data/status` | `dataset::status` |
| `GET /debug/equipment-data/overview` | `dataset::overview` |
| `GET /debug/equipment-data/resources` | `resources::resources` |
| `GET /debug/equipment-data/relationships` | `resources::relationships` |
| `GET /debug/equipment-data/fields` | `resources::fields` |
| `GET /debug/equipment-data/resource-cards` | `source_inspection::resource_cards` |
| `GET /debug/equipment-data/selection` | `source_inspection::selection` |
| `GET /debug/equipment-data/containers` | `source_inspection::containers` |
| `GET /debug/equipment-data/properties` | `source_inspection::properties` |
| `GET /debug/equipment-data/values` | `source_inspection::values` |
| `GET /debug/equipment-data/documents` | `source_inspection::documents` |
| `GET /debug/equipment-data/download` | `downloads::download` |

Every route reads the shared `api_equipment_datasets::queries::ViewerQuery`: `dataset`
selects the dataset (`gameplay` when absent) and `generation` pins the generation the answer
comes from. Every answer but `download` is re-read as its generated contract type from
`contract_schema_types::community_content::equipment_data_viewer` and refused when it serialises to more than
`api_equipment_datasets::PAGE_BYTES` (256 KiB); a complete value is reached through
document expansion instead. Any failure a handler raises answers 400 in the `{error}` envelope,
and so does a query string that does not decode (a `field_id` that is not a number): `mod.rs`
turns the `Query` rejection into that envelope through `ApiError::from_query_rejection`, and the
error names the parameter that failed.
`download` streams the named document (`generation` when absent) as an `application/json`
attachment with `Cache-Control: private, no-store`.

## Boundaries

- Depends on: `api_equipment_datasets` (dataset selection, queries, the source manifest)
  through `AppState::equipment_data`, and
  `contract_schema_types::community_content::equipment_data_viewer`.
- Used by: the domain's `routes.rs`, which registers every handler here in development only;
  over HTTP, the equipment data viewer bench in `apps/frontend/src/workspaces/debug/data_viewer/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`), and
  `apps/api/tests/debug_routes_are_development_only.rs` proves the production 404 and
  the development registration for every tagged debug route.
