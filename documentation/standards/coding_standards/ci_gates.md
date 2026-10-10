**Status:** live

# CI gates

Rules CI-1 and CI-2, the rules about the CI configuration itself, and the workspace laws
(WS-1 to WS-5). Code comments and help strings that cite "§0.3" or "§11" point here. Where every
gate runs is the gate matrix of
[Testing and CI](/documentation/runbooks/testing_and_ci.md#gate-matrix); this page does not
repeat it.

The repository is pre-alpha: CI runs formatting, clippy, the tests, the schema checks and a few
cheap structural laws (crate anatomy, the crate firewalls, Tailwind sources, the language bans).
Slow suites (headless browser gates, mod world boot) run nightly or on demand. What to test
follows the pre-alpha test policy, `CLAUDE.md` law 11.

## Rules

- **CI-2 (Debuggability) — `ci.yml` gates every push and pull request to `main`.**
  `.github/workflows/ci.yml` runs on every push and pull request to `main` with no path filter.
  Its jobs:

  | Job | Runs | Rules |
  |---|---|---|
  | `api` | `cargo xtask mk rust-fmt`, `mk rust-clippy`, then `ci api-test` (the API's `cargo test`) against a Postgres 18 service | FMT-1, GO-2, GO-8, TEST-1 |
  | `wasm-ci` | `cargo xtask mk wasm-ci`: clippy with `-D warnings` for `wasm32` over every crate the workspace marks `targets = "wasm32"` outside the frontend family | FMT-1 |
  | `frontend` | `cargo xtask mk ci-local-leptos`: format, clippy with `-D warnings` for `wasm32` and natively, tests, release Trunk build, over the frontend family | TEST-2, TS-6 |
  | `schema` | `cargo xtask ci ci-local-schema`: generated types current, schema validation | TEST-3, ENF-4 |
  | `editorconfig` | `cargo xtask ci verify-editorconfig` | FMT-2 |
  | `workspace-members` | one `cargo test --workspace` run over the members no other job covers | TEST-2 |
  | `language-gates` | `verify tailwind-sources`, `crate-anatomy`, `crate-tiers`, one language-ban step (`no-node`, `no-python`, `no-shell`), and `file-length` as a warning | LANG-1, LANG-2, LANG-3, WS-1, WS-2, WS-5 |

  `cargo xtask ci ci-local` replays the same gates locally, with the integration tests run by
  `cargo xtask ci rust-test-it` against the local database. Gate: CI-BLOCK, the workflow itself.
- **CI-1 (Debuggability) — No lint job hides old issues.** It forbade `only-new-issues: true` on
  the Go lint job, so that every lint finding in the tree fails, not only the new ones. Status:
  retired with the Go backend; clippy has no such switch, and every job lints the whole crate.

## On-demand checks

These commands stay available but block no push: `cargo xtask verify file-length` (SIZE-3, warns),
`no-select-star`, `test-file-reachability` (WS-3), `frontend-layering` (WS-4), `link-check`, and
the browser and mod world-boot gates, which run nightly or on demand.

## verify-workspace-laws

The workspace laws of the
[crate boundary rules](/documentation/standards/crate_boundary_rules.md#5-the-workspace-laws)
are WS-1 to WS-5. WS-1, WS-2 and WS-5 run in `ci-local` and the `language-gates` job of `ci.yml`;
WS-3 and WS-4 run on demand. The laws live in
`tools/foundation/repository_laws/src/workspace_laws/`
([README](/tools/foundation/repository_laws/src/workspace_laws/README.md)); xtask passes
in every path that moves with the tree. Each law prints `<LAW>: PASS`, or `FAIL` with exit 1 on a
finding and exit 2 when an input could not be read. The judged set is every workspace member that
declares `[package.metadata.layout]` plus every member under `crates/<category…>/<name>` or
`tools/<category>/<name>`; the members outside the set (the two tool binaries) are listed in a
note.

### WS-1 crate tiers

`cargo xtask verify crate-tiers` enforces two things: no member depends on an application package
(`api_server`, `frontend_application`, `offline_service_worker`, `game_server_host_agent`), in
any table; and the external-crate firewalls hold (`wgpu`, `sqlx`,
`axum`, `leptos` and the browser crates stay in the crates the
[crate boundary rules](/documentation/standards/crate_boundary_rules.md#54-the-firewalls) name,
and the dependency closure of xtask holds no tokio, axum, reqwest, resvg or image). The tier
numbers and the category matrix are layering guidance; the gate does not judge every edge.

### WS-2 crate anatomy

`cargo xtask verify crate-anatomy` holds every judged library crate to its anatomy: `lib.rs` at
most 80 lines of doc comments, attributes, `mod` and `pub use` lines; `pub mod prelude`; an
`error.rs` with a `thiserror` `pub enum Error` and a `pub type Result` when a `pub fn` returns
`Result`; no `anyhow` dependency; a README.md with a Contents block; `edition`, `rust-version`,
`[lints]` and every dependency from the workspace; features only `test_fixtures` and `failpoints`,
each enabled only by a dev-dependency; no primitive-typed public `id` or `*_id` field or parameter
outside `generated/` folders and `#[wasm_bindgen]` items; and no `pub use` of another workspace
crate outside `prelude.rs`. Binary crates are exempt.

### WS-3 test-file reachability

`cargo xtask verify test-file-reachability` fails on every `.rs` file of a workspace member that
sits in a `tests` folder (under `src/` or the member's own `tests/` folder) and that no target of
the member loads: the walk starts at the target roots (`[lib]` and `[[bin]]` paths, `src/lib.rs`,
`src/main.rs`, `src/bin/`, `tests/*.rs`, `tests/*/main.rs`, `benches/`, `examples/`) and follows
every `mod` declaration with its `#[path]` and every trybuild case a loaded file names. Such a file
never compiles, so its tests never run while the tree looks covered.

### WS-4 frontend layering

`cargo xtask verify frontend-layering` judges the frontend in two modes, configured in
`tools/checks/repository_checks/src/architecture/workspace_law_locations.rs`.

- **Crate-edge mode** (`FRONTEND_CRATE_EDGES`): every normal, dev and build dependency edge
  between frontend crates. A crate's layer is its folder `crates/frontend/<layer>/` (foundation <
  features < pages, workspaces) and the app `crates/frontend/shell/frontend_application` is the shell. An edge fails when a
  lower layer depends on a higher one, when pages and workspaces depend on each other, or when one
  page crate depends on another. The crate orders hold inside a layer folder:
  `FOUNDATION_CRATE_ORDER` (`frontend_ui` < `frontend_api_dtos` < {`frontend_transport`,
  `frontend_route_table`} < `frontend_session` < {`frontend_offline`, `frontend_map_view`}, the
  braced crates peers that never depend on each other, `frontend_test_support` reached only
  through dev-dependencies), `MISSION_CREATOR_CRATE_ORDER` (`mission_creator_state` <
  `mission_creator_engine_bridge` < `mission_creator_session` < `mission_creator_arsenal` <
  `mission_creator_workspace`) and `DEBUG_BENCHES_CRATE_ORDER`, an order of its own whose crate
  and the Mission Creator's never depend on each other. A frontend crate in no layer folder, a
  crate in a layer folder with orders but in none of them, a crate an order names that sits in
  another layer folder, and a crate an order names that no member carries are findings.
- **In-crate mode** (`APP_LAYERS`): the module-level rules inside one crate, through a layer
  table; the app's table maps `main.rs`, `app_routes.rs`, `shell/` and `tests/` onto the shell,
  and every app source must sit under a row.

The law runs on demand and is not a CI gate.

### WS-5 Tailwind sources

`cargo xtask verify tailwind-sources` holds the `@source` lines of `crates/frontend/shell/frontend_application/style/aegis.css`
exact: every workspace member that depends on `leptos` (outside dev-dependencies), the app
included, is named by exactly one line `@source "<path>/src/**/*.rs";` whose path, resolved from
the stylesheet's folder, is that member's folder. A member no line names, a member several lines
name, and a stale line that names no leptos member (an ancestor or wildcard glob included) are
findings. Trunk's `[watch]` list in `crates/frontend/shell/frontend_application/Trunk.toml` covers `crates/frontend`, so a
change in any frontend crate rebuilds the bundle.

## Adding a rule

A new rule gets the next free number of its family, a pillar and a line in the rule index of the
[README](/documentation/standards/coding_standards/README.md). Most rules are conventions
reviewers hold; add a gate only when a silent break would reach production, and prefer an
on-demand check over a CI gate while the project is pre-alpha. A rule no tool checks is stated as
unenforced, and only an
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) runtime rule may be MANUAL.
