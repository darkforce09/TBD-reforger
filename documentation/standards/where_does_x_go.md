**Status:** live

# Where does X go?

The home of each kind of file in the repository, so a new file lands where the next reader looks
for it. The layout rules a gate enforces name the gate; the others are conventions. The whole tree
is drawn in the directory atlas of `CLAUDE.md`, and every code folder's README says what it holds.

## Website

| X | Home |
|---|---|
| a page of the app | `crates/frontend/pages/<crate>/src/<page>/`, in the page crate of its navigation area; its route in `crates/frontend/shell/frontend_application/src/app_routes.rs` (the component) and `crates/frontend/foundation/frontend_route_table/src/routes.rs` (layout flags and access tier) |
| a standalone workspace, such as the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) | its own crate under `crates/frontend/workspaces/<crate>/` (the Mission Creator is five crates in a fixed order) |
| what every page shares: the API client, auth, design-system primitives, utilities | a foundation crate under `crates/frontend/foundation/` (`frontend_transport`, `frontend_session`, `frontend_ui`, …); a capability several pages show, a feature crate under `crates/frontend/features/` |
| an app-side mirror of an API model | `crates/frontend/foundation/frontend_api_dtos/src/`, with its R-api golden test in that folder's `tests/` |
| an [API](/documentation/glossary/a_to_f.md#api) endpoint | `crates/api/api_<domain>/src/handlers/<surface>.rs` in the crate of its domain, registered in that crate's `src/routes.rs`, with its `/// @route` tag |
| API logic that two surfaces share | that domain crate's `src/services/` |
| an API wire or database model | that domain crate's `src/models/`, the snake_case contract |
| API code that names no domain concept: pagination, SQLSTATE predicates, wire formats, text guards, token primitives | the API infrastructure crates under `crates/api/` (`api_foundation`, `api_http_layer`, `api_configuration`, `api_database`) |
| an API background ticker | `crates/api/api_background_workers/src/`; the work itself stays in the owning domain crate's `src/services/` |
| an API binary | `crates/api/api_server/src/bin/` (the `api-server` server and the `import-item-registry` tool), declared as a `[[bin]]` of `api_server` |
| a database migration | `crates/api/api_database/migrations/NNNN_<subject>.sql` (sqlx, embedded, applied at boot) |
| a development seed | `crates/api/api_database/seeds/`; `cargo xtask db seed` applies the files its `SEEDS` list names, and `mock_data.sql` is applied by hand |
| the [mission](/documentation/glossary/g_to_m.md#mission) document model, compiler and validation | `crates/mission/` |
| terrain, world-object and world formats, spatial indexes, line of sight, camera math | `crates/terrain/`, `crates/world_objects/`, `crates/world_formats/`, `crates/geometry/`, `crates/line_of_sight/` |
| map streaming: chunk residency, draw buffers, browser loaders, the map host | `crates/streaming/` |
| map rendering: the render engine, its typed GPU layers and readback self-checks | `crates/map_rendering/` |
| GPU rendering primitives with no map concept | `crates/graphics/` |

The API has eight domain crates under `crates/api/`: `api_administration`, `api_command_center`,
`api_community_content`, `api_identity_and_access`, `api_match_telemetry`, `api_missions`,
`api_operations` and `api_server_infrastructure`. `api_v1_routes` in
`crates/api/api_server/src/router.rs` merges the route table each of them exports from its
`src/routes.rs` and nests them under `/api/v1`, so a public URL is the literal in the domain
crate's `routes.rs` with `/api/v1` in front. A kernel crate depends on no domain crate, a domain
crate depends on another only along the one-way domain graph and never imports its handlers, and
`api_background_workers` is named only by the server binary
`crates/api/api_server/src/bin/api_server.rs`. The walls between the map
crates and the app are in the [crate boundary rules](/documentation/standards/crate_boundary_rules.md).

## Contracts, data and assets

| X | Home |
|---|---|
| a JSON Schema for a shape that crosses a network, process or language boundary | `contracts/definitions/`; `cargo xtask ci schema-codegen` generates the Rust types into `crates/contracts/contract_schema_types/src/generated/` |
| a golden fixture shared across crates | `contracts/fixtures/<family>/` (`missions`, `map`, `registry`, `enfusion_samples`, `bridge_samples`) |
| a test fixture one crate reads | that crate's `tests/fixtures/`, beside the test; never `.ai/artifacts/` |
| a catalog exported from Workbench for the platform to ingest | `contracts/catalogs/` |
| a terrain dataset | `assets/terrains/<terrain>/`; images and binary payloads go through Git LFS (`.gitattributes`); the tile pyramid under `tiles/` is local build output and gitignored |
| terrain export scratch | `assets/scratch/<terrain>/`, gitignored and never served |

## Mod

| X | Home |
|---|---|
| gameplay [EnfScript](/documentation/glossary/a_to_f.md#enfscript) | `mod/tbd-framework/Scripts/Game/TBD/<area>/` (`API`, `Core`, `Gamemode`, `Session`, `Systems`, `UI`) |
| a UI layout | `mod/tbd-framework/UI/layouts/` (`Common`, `Hud`, `Session`) |
| a [mission header](/documentation/glossary/g_to_m.md#mission-header) | `mod/tbd-framework/Missions/` |
| a Workbench export plugin | `mod/tbd-export/Scripts/WorkbenchGame/` |
| an Enfusion MCP handler | `mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/` |

## Tooling and tickets

| X | Home |
|---|---|
| a repository tool, gate or code generator | a `cargo xtask` subcommand in `tools/xtask/`; tooling is Rust, never a tracked shell or Python script (LANG-1, `cargo xtask verify no-shell`) |
| a heavy tool: the gate harness, asset pipelines, the Enfusion unpacker, the MCP broker | a binary of `tools/developer_tools/src/bin/` |
| a browser smoke of the Mission Creator | the `gate` binary of `tools/developer_tools`, wired into `cargo xtask mk leptos-gates` |
| a deploy template or systemd unit | `deploy/` |
| a [ticket](/documentation/glossary/n_to_z.md#ticket), its run receipts and its place in a wave | the central ticket manager, written through `ttm --project reforger` commands, never a file in the repository |
| a ticket's spec and plan | the ticket manager (`ttm --project reforger brief <ticket>`); the records already written stay in `documentation/tickets/specs/` and `documentation/tickets/plans/` (see [Ticket identifiers](/documentation/standards/ticket_identifiers.md)) |
| code that talks to the ticket manager | through `tools/foundation/ticket_manager_client/`, never a `ttm` call of its own |

## Documentation

| X | Home |
|---|---|
| what a code folder holds | its own `README.md`, per the [README standard](/documentation/standards/readme_standard.md) |
| any other Markdown about code: feature docs, specs, decisions, research | `documentation/`, in the folder that mirrors the code folder; never a `docs` folder under `crates/`, `mod/`, `tools/`, `contracts/` or `assets/` |
| a procedure | `documentation/runbooks/` |
| a design reference image or export | the `visual_references/` folder of the feature it depicts |
| a recorded defect | `documentation/known_bugs/` |
| a term | the file of its first letter in `documentation/glossary/`, with a line in its index |

By convention a code tree holds no Markdown other than a README (a `tests`, `generated`,
`Generated` or dot-prefixed folder excepted), and a live document under `documentation/` stays
around 500 lines. The
[documentation standards](/documentation/standards/documentation_standards.md) hold the rest.
