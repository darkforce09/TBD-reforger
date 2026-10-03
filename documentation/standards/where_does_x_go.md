**Status:** live

# Where does X go?

The home of each kind of file in the repository, so a new file lands where the next reader looks
for it. The layout rules a gate enforces name the gate; the others are conventions. The whole tree
is drawn in the directory atlas of `CLAUDE.md`, and every code folder's README says what it holds.

## Website

| X | Home |
|---|---|
| a page of the app | `apps/frontend/src/pages/<area>/<page>/`; its route in `apps/frontend/src/app_routes.rs` (the component) and `apps/frontend/src/foundation/route_table/mod.rs` (layout flags and access tier) |
| a standalone workspace, such as the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) | `apps/frontend/src/workspaces/<workspace>/` |
| what every page shares: the API client, auth, design-system primitives, utilities | `apps/frontend/src/foundation/` |
| an app-side mirror of an API model | `apps/frontend/src/foundation/transport/dto/`, with its R-api golden test in that folder's `tests/` |
| an [API](/documentation/glossary/a_to_f.md#api) endpoint | `apps/api/src/<domain>/handlers/<surface>.rs`, registered in that domain's `routes.rs`, with its `/// @route` tag |
| API logic that two surfaces share | that domain's `services/` |
| an API wire or database model | that domain's `models/`, the snake_case contract |
| API code that names no domain concept: pagination, SQLSTATE predicates, wire formats, text guards, token primitives | the API infrastructure crates under `crates/api/` (`api_foundation`, `api_http_layer`, `api_configuration`, `api_database`) |
| an API background ticker | `crates/api/api_background_workers/src/`; the work itself stays in the owning domain's `services/` |
| an API binary | `apps/api/src/bin/` |
| a database migration | `crates/api/api_database/migrations/NNNN_<subject>.sql` (sqlx, embedded, applied at boot) |
| a development seed | `crates/api/api_database/seeds/`; `cargo xtask db seed` applies the files its `SEEDS` list names, and `mock_data.sql` is applied by hand |
| the [mission](/documentation/glossary/g_to_m.md#mission) document model, compiler and validation | `crates/mission/` |
| terrain, world-object and world formats, spatial indexes, line of sight, camera math | `crates/terrain/`, `crates/world_objects/`, `crates/world_formats/`, `crates/geometry/`, `crates/line_of_sight/` |
| map streaming: chunk residency, draw buffers, browser loaders, the map host | `crates/streaming/` |
| map rendering: the render engine, its typed GPU layers and readback self-checks | `crates/map_rendering/` |
| GPU rendering primitives with no map concept | `crates/graphics/` |

The API has eight domains: `administration`, `command_center`, `community_content`,
`identity_and_access`, `match_telemetry`, `missions`, `operations` and `server_infrastructure`.
`api_v1_routes` in `apps/api/src/router.rs` merges their route tables and
nests them under `/api/v1`, so a public URL is the literal in the domain's `routes.rs` with
`/api/v1` in front. `core` imports no domain except its composition root, a domain's handlers
never import another domain's handlers, and `background_workers` is imported only by
`apps/api/src/bin/api.rs`; the tests in
`apps/api/src/tests/architecture_rules.rs` enforce all three. The walls between the map
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
| gameplay [EnfScript](/documentation/glossary/a_to_f.md#enfscript) | `apps/mod/tbd-framework/Scripts/Game/TBD/<area>/` (`API`, `Core`, `Gamemode`, `Session`, `Systems`, `UI`) |
| a UI layout | `apps/mod/tbd-framework/UI/layouts/` (`Common`, `Hud`, `Session`) |
| a [mission header](/documentation/glossary/g_to_m.md#mission-header) | `apps/mod/tbd-framework/Missions/` |
| a Workbench export plugin | `apps/mod/tbd-export/Scripts/WorkbenchGame/` |
| an Enfusion MCP handler | `apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/` |

## Tooling and tickets

| X | Home |
|---|---|
| a repository tool, gate or code generator | a `cargo xtask` subcommand in `tools/xtask/`; tooling is Rust, never a tracked shell or Python script (LANG-1, `cargo xtask verify no-shell`) |
| a heavy tool: the gate harness, asset pipelines, the Enfusion unpacker, the MCP broker | a binary of `tools/developer_tools/src/bin/` |
| a browser smoke of the Mission Creator | the `gate` binary of `tools/developer_tools`, wired into `cargo xtask mk leptos-gates` |
| a deploy template or systemd unit | `deploy/` |
| a [ticket](/documentation/glossary/n_to_z.md#ticket) | `.ai/tickets/T-<id>.toml`, written through `cargo xtask ticket` commands; `cargo xtask ticket sync` writes the derived files, never a hand edit |
| a ticket's spec and plan | `documentation/tickets/specs/t<id>_<subject>.md` and `documentation/tickets/plans/t-<id>_plan.md` (see [Ticket identifiers](/documentation/standards/ticket_identifiers.md)) |

## Documentation

| X | Home |
|---|---|
| what a code folder holds | its own `README.md`, per the [README standard](/documentation/standards/readme_standard.md) |
| any other Markdown about code: feature docs, specs, decisions, research | `documentation/`, in the folder that mirrors the code folder; never a `docs` folder under `apps/`, `contracts/` or `assets/` |
| a procedure | `documentation/runbooks/` |
| a design reference image or export | the `visual_references/` folder of the feature it depicts |
| a recorded defect | `documentation/known_bugs/` |
| a term | the file of its first letter in `documentation/glossary/`, with a line in its index |

`cargo xtask verify markdown-placement` holds the documentation layout: it refuses any Markdown
in a code tree other than a README (a `tests`, `generated`, `Generated` or dot-prefixed folder
excepted) and any live document under `documentation/` over 500 lines. `ci-local` (through
`cargo xtask ci verify-documentation`) and the `language-gates` job of `ci.yml` run it. The
[documentation standards](/documentation/standards/documentation_standards.md) hold the rest.
