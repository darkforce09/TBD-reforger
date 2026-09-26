**Status:** live

# Where does X go?

The home of each kind of file in the repository, so a new file lands where the next reader looks
for it. The layout rules a gate enforces name the gate; the others are conventions. The whole tree
is drawn in the directory atlas of `CLAUDE.md`, and every code folder's README says what it holds.

## Website

| X | Home |
|---|---|
| a page of the app | `apps/website/frontend/src/v2/pages/<area>/<page>/`; its route in `apps/website/frontend/src/app_routes.rs` (the component) and `apps/website/frontend/src/router.rs` (layout flags and access tier) |
| a standalone workspace, such as the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) | `apps/website/frontend/src/v2/apps/<workspace>/` |
| what every page shares: the API client, auth, design-system primitives, utilities | `apps/website/frontend/src/v2/core/` |
| an app-side mirror of an API model | `apps/website/frontend/src/v2/core/api/dto/`, with its R-api golden test in that folder's `tests/` |
| an [API](/documentation_v2/glossary/a_to_f.md#api) endpoint | `apps/website/api_v2/src/<domain>/handlers/<surface>.rs`, registered in that domain's `routes.rs`, with its `/// @route` tag |
| API logic that two surfaces share | that domain's `services/` |
| an API wire or database model | that domain's `models/`, the snake_case contract |
| API code that names no domain concept: pagination, SQLSTATE predicates, wire formats, text guards, token primitives | `apps/website/api_v2/src/core/` |
| an API background ticker | `apps/website/api_v2/src/background_workers/`; the work itself stays in the owning domain's `services/` |
| an API binary | `apps/website/api_v2/src/bin/` |
| a database migration | `apps/website/api_v2/migrations/NNNN_<subject>.sql` (sqlx, embedded, applied at boot) |
| a development seed | `apps/website/api_v2/seeds/`; `cargo xtask db seed` applies the files its `SEEDS` list names, and `mock_data.sql` is applied by hand |
| map graphics, spatial computation, terrain formats, streaming, camera math, the [mission](/documentation_v2/glossary/g_to_m.md#mission) document model | `apps/website/map-engine/src/` |
| GPU rendering primitives with no map concept | `apps/website/graphics-engine/src/` |

The API has eight domains: `administration`, `command_center`, `community_content`,
`identity_and_access`, `match_telemetry`, `missions`, `operations` and `server_infrastructure`.
`api_v1_routes` in `apps/website/api_v2/src/core/http_router.rs` merges their route tables and
nests them under `/api/v1`, so a public URL is the literal in the domain's `routes.rs` with
`/api/v1` in front. `core` imports no domain except its composition root, a domain's handlers
never import another domain's handlers, and `background_workers` is imported only by
`apps/website/api_v2/src/bin/api.rs`; the tests in
`apps/website/api_v2/src/tests/architecture_rules.rs` enforce all three. The walls between the two
engines and the app are in the [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md).

## Contracts, data and assets

| X | Home |
|---|---|
| a JSON Schema for a shape that crosses a network, process or language boundary | `contracts_v2/definitions/`; `cargo xtask ci schema-codegen` generates the API's Rust types into the `generated/` folders under `apps/website/api_v2/src/` |
| a golden fixture shared across crates | `contracts_v2/fixtures/<family>/` (`missions`, `map`, `registry`, `enfusion_samples`, `bridge_samples`) |
| a test fixture one crate reads | that crate's `tests/fixtures/`, beside the test; never `.ai/artifacts/` |
| a catalog exported from Workbench for the platform to ingest | `contracts_v2/catalogs/` |
| a terrain dataset | `assets_v2/terrains/<terrain>/`; images and binary payloads go through Git LFS (`.gitattributes`); the tile pyramid under `tiles/` is local build output and gitignored |
| terrain export scratch | `assets_v2/scratch/<terrain>/`, gitignored and never served |

## Mod

| X | Home |
|---|---|
| gameplay [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) | `apps/mod/tbd-framework/Scripts/Game/TBD/<area>/` (`API`, `Core`, `Gamemode`, `Session`, `Systems`, `UI`) |
| a UI layout | `apps/mod/tbd-framework/UI/layouts/` (`Common`, `Hud`, `Session`) |
| a [mission header](/documentation_v2/glossary/g_to_m.md#mission-header) | `apps/mod/tbd-framework/Missions/` |
| a Workbench export plugin | `apps/mod/tbd-export/Scripts/WorkbenchGame/` |
| an Enfusion MCP handler | `apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/` |

## Tooling and tickets

| X | Home |
|---|---|
| a repository tool, gate or code generator | a `cargo xtask` subcommand in `tools_v2/xtask/`; tooling is Rust, never a tracked shell or Python script (LANG-1, `cargo xtask verify no-shell`) |
| a heavy tool: the gate harness, asset pipelines, the Enfusion unpacker, the MCP broker | a binary of `tools_v2/developer-tools/src/bin/` |
| a browser smoke of the Mission Creator | the `gate` binary of `tools_v2/developer-tools`, wired into `cargo xtask mk leptos-gates` |
| a deploy template or systemd unit | `tools_v2/xtask/deploy/` |
| a [ticket](/documentation_v2/glossary/n_to_z.md#ticket) | `.ai/tickets/T-<id>.toml`, written through `cargo xtask ticket` commands; `cargo xtask ticket sync` writes the derived files, never a hand edit |
| a ticket's spec and plan | `documentation_v2/tickets/specs/t<id>_<subject>.md` and `documentation_v2/tickets/plans/t-<id>_plan.md` (see [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md)) |

## Documentation

| X | Home |
|---|---|
| what a code folder holds | its own `README.md`, per the [README standard](/documentation_v2/standards/readme_standard.md) |
| any other Markdown about code: feature docs, specs, decisions, research | `documentation_v2/`, in the folder that mirrors the code folder; never a `docs` folder under `apps/`, `contracts_v2/` or `assets_v2/` |
| a procedure | `documentation_v2/runbooks/` |
| a design reference image or export | the `visual_references/` folder of the feature it depicts |
| a recorded defect | `documentation_v2/known_bugs/` |
| a term | the file of its first letter in `documentation_v2/glossary/`, with a line in its index |

`cargo xtask verify markdown-placement` holds the documentation layout: it refuses any Markdown
in a code tree other than a README (a `tests`, `generated` or dot-prefixed folder excepted) and any
live document under `documentation_v2/` over 500 lines. `ci-local` (through
`cargo xtask ci verify-documentation`) and the `language-gates` job of `ci.yml` run it. The
[documentation standards](/documentation_v2/standards/documentation_standards.md) hold the rest.
