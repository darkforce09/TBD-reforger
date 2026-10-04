# Frontend API DTOs

The `frontend_api_dtos` crate: the single-page app's wire types, one Rust shape per JSON body the
[API](/documentation/glossary/a_to_f.md#api) sends or accepts, grouped by domain, and the
serde-transparent typed identifiers every one of them holds. The API's models are the source of
truth; these mirror them, and every type is held to a captured answer of the API.

## Contents

```text
crates/frontend/foundation/frontend_api_dtos/
├── Cargo.toml  the package: `serde`, `newtype_ids`, the mission and ballistics crates, layout tier 7
└── src/        the DTO modules by domain, the typed identifiers and their golden round trips
```

## How it works

Every type is plain `serde` data and compiles on every target, so the golden round trips run
natively: each DTO re-serialises its capture from `contracts/fixtures/api_goldens/` byte for
byte, and the keys no named field reads are exactly the ones its test lists. Identifiers are
newtypes over the exact wire type (a `String` for text ids, never a parsed UUID), so a golden's
bytes round-trip unchanged. The mission and ballistics types a DTO carries (the faction library,
the compiled-row metadata, the catalog document, the fire-mission solution) are named from their
own crates, never re-exported here. The [source tree README](src/README.md) describes each module
and the value rules the DTOs keep.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_api_dtos   # every golden round trip, the shapes and the write bodies
```

To follow an API model change: change the DTO here, re-capture the golden if the answer changed,
and run the command above.

## Configuration

None: no feature, no environment variable. The tests read the fixture corpus through
`frontend_test_support`, a dev-dependency.

## Public surface

- The DTO modules, re-exported flat at the crate root except `administration`,
  `ballistics_catalogs`, `vehicles`, `wiki`, `equipment_data_viewer`, `identifiers` and `role`,
  which callers name by module.
- `identifiers`: one newtype per identifier kind (`MissionId`, `EventMissionId`, `ServerId`,
  `DiscordUserId`, …) and `identifier_is_empty`.
- `role::Role`: the five-tier account ladder.
- `decode_server_status_frame` and `MissionDetail::compiled_meta`.
- `prelude`: every typed identifier and `Role`.

## Boundaries

- Depends on: `serde`, `serde_json`, `newtype_ids`, `mission_operations`, `mission_compiler`,
  `ballistics_model`, `ballistics_solver`, `fire_mission_planning`; `web-sys` and `wasm-bindgen`
  in the wasm32 build only.
- Used by: the frontend transport, the session, the route table, the pages, the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) and the equipment data
  viewer.
- Rules: the API model changes first and the DTO follows; a golden round-trips and its unread keys
  match its list, so drift either way fails (`cargo test -p frontend_api_dtos`); the crate
  depends on no frontend crate but the dev-only test support (`cargo xtask ci
  verify-workspace-laws`).

## Related documentation

- [Documentation standards](/documentation/standards/documentation_standards.md#2-contracts-behind-the-tags)
  — how the API's models, the schemas and these DTOs stay one contract.
