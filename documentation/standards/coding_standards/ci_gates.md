**Status:** live

# CI gates

Rules CI-1 and CI-2, the rules about the CI configuration itself, the `verify-coding-standards`
task that bundles four of the code gates, the `verify-workspace-laws` task that bundles the five
workspace laws (WS-1 to WS-5), and the `verify-documentation` task that bundles the three
documentation gates. Code comments and help strings that cite "§0.3" or
"§11" point here. Where every gate runs (the local replay, each GitHub job, the
[wave](/documentation/glossary/n_to_z.md#wave) gate) is the gate matrix of
[Testing and CI](/documentation/runbooks/testing_and_ci.md#gate-matrix); this page does not
repeat it.

## Rules

- **CI-2 (Debuggability) — `ci.yml` gates every push and pull request to `main`.**
  `.github/workflows/ci.yml` runs on every push and pull request to `main` with no path filter,
  unlike `contracts.yml` and `schema.yml`, which run only when their paths change. Its jobs:

  | Job | Runs | Rules |
  |---|---|---|
  | `api` | `cargo xtask mk rust-fmt`, `mk rust-clippy`, `mk rust-build`, then `ci api-test` (the API's `cargo test`) against a Postgres 18 service | FMT-1, GO-2, GO-8, GO-9, TEST-1 |
  | `wasm-ci` | `cargo xtask mk wasm-ci`: clippy with `-D warnings` for `wasm32` over every crate the workspace marks `targets = "wasm32"` outside the frontend family | FMT-1 |
  | `frontend` | `cargo xtask mk ci-local-leptos`: format, clippy with `-D warnings` for `wasm32` and natively, tests, release Trunk build, over the frontend family (every `crates/frontend` crate, the single-page app and the offline service worker among them) | TEST-2, TS-6 |
  | `schema` | `cargo xtask ci ci-local-schema`: generated types current, schema validation, `@contract` citations | TEST-3, ENF-3, ENF-4 |
  | `editorconfig` | `cargo xtask ci verify-editorconfig` | FMT-2 |
  | `language-gates` | `verify no-python`, `no-node`, `file-length`, `enfusion-comments`, `no-shell`, `ci-shell`, `crate-tiers`, `crate-anatomy`, `test-file-reachability`, `frontend-layering`, `tailwind-sources`, `ticket check --strict`, then `verify readme-coverage`, `link-check`, `markdown-placement` | LANG-1, LANG-2, LANG-3, SIZE-3, WS-1 to WS-5 |
  | `mod-gates-hosted` | `mod world-boot --selftest`, `verify staging-compose-paths`, `mission-rest-size-limits`, `ci-schema-parity` | none |

  `cargo xtask ci ci-local` replays the same gates locally, with the integration tests run by
  `cargo xtask ci rust-test-it` against the local database. Gate: CI-BLOCK, the workflow itself.
- **CI-1 (Debuggability) — No lint job hides old issues.** It forbade `only-new-issues: true` on
  the Go lint job, so that every lint finding in the tree fails, not only the new ones. Status:
  retired with the Go backend; clippy has no such switch, and every job lints the whole crate.

## verify-coding-standards

`cargo xtask ci verify-coding-standards` runs four gates in order and stops at the first failure:

1. `cargo xtask verify file-length`: SIZE-3.
2. `cargo xtask verify enfusion-comments`: the Enfusion comment card (rules ECM-1 to ECM-9 of the
   [comment gate README](/tools/checks/mod_script_checks/src/enfusion_comments/README.md),
   sections 6 and 7 of the
   [documentation standards](/documentation/standards/documentation_standards.md#6-enfusion-comments))
   over the pinned mod Scripts roots, today `apps/mod/tbd-framework/Scripts` and
   `apps/mod/tbd-emcp/Scripts`.
3. `cargo xtask verify no-select-star`: no `SELECT *` or `RETURNING *` in the API's SQL, outside
   the two tables with no nullable column; the
   [database verifications README](/tools/commands/database_operations/src/database_checks/README.md) has the
   rule. It has no rule code.
4. `cargo xtask verify route-tags`: GO-7.

`ci-local` runs the task as one step. On GitHub, only `file-length` and `enfusion-comments` run
(in `language-gates`); the `SELECT *` and route-tag gates run in `ci-local` and, for `route-tags`,
the wave and slice gates, but in no workflow.

## verify-workspace-laws

`cargo xtask ci verify-workspace-laws` runs the five workspace laws of the
[crate boundary rules](/documentation/standards/crate_boundary_rules.md#5-the-workspace-laws) in
order and stops at the first failure; `ci-local` runs it right after `verify-ci-shell`, and the
`language-gates` job of `ci.yml` runs the five commands as separate steps. The laws live in
`tools/foundation/repository_laws/src/workspace_laws/`
([README](/tools/foundation/repository_laws/src/workspace_laws/README.md)); xtask passes
in every path that moves with the tree. Each law prints `<LAW>: PASS`, or `FAIL` with exit 1 on a
finding and exit 2 when an input could not be read. The judged set is every workspace member that
declares `[package.metadata.layout]` plus every member under `crates/<category…>/<name>` or
`tools/<category>/<name>`; the members outside the set (the apps and the two tool binaries) are
listed in a note.

### WS-1 crate tiers

`cargo xtask verify crate-tiers` judges rules 1 to 7 of the crate-tier law: every `Cargo.toml`
under `apps`, `crates` and `tools` (outside test trees, fixtures and build output) is a workspace
member; each judged member declares `category`, `tier` and `targets`, and every member outside the
judged set is an app or one of the two tool binaries; a judged member sits at its category plus
its name and declares the tier its dependencies give it (0 with no judged dependency, otherwise 1
plus the highest); edges point strictly down and follow the category matrix; a wasm-only crate is
reached from a crate for every platform only through a `cfg(target_arch = "wasm32")` table; the
external-crate firewalls hold (wgpu, the browser crates, sqlx and axum, leptos, the dependency
closure of xtask, map nouns in graphics, browser words in mission editing, `#[wasm_bindgen]`
exports); and dev-dependencies never point at apps. The
[crate boundary rules](/documentation/standards/crate_boundary_rules.md#52-crate-tiers-cargo-xtask-verify-crate-tiers)
state each rule, the matrix and the firewalls in full.

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

The law is hard at zero: every edge, production or test, normal or dev, fails it.

### WS-5 Tailwind sources

`cargo xtask verify tailwind-sources` holds the `@source` lines of `crates/frontend/shell/frontend_application/style/aegis.css`
exact: every workspace member that depends on `leptos` (outside dev-dependencies), the app
included, is named by exactly one line `@source "<path>/src/**/*.rs";` whose path, resolved from
the stylesheet's folder, is that member's folder. A member no line names, a member several lines
name, and a stale line that names no leptos member (an ancestor or wildcard glob included) are
findings. Trunk's `[watch]` list in `crates/frontend/shell/frontend_application/Trunk.toml` covers `crates/frontend`, so a
change in any frontend crate rebuilds the bundle.

## verify-documentation

`cargo xtask ci verify-documentation` runs the three documentation gates over the committed files,
in order, and stops at the first failure: `cargo xtask verify readme-coverage` (every folder's
README.md and its Contents block), `cargo xtask verify link-check` (links, backticked paths and
cited commands) and `cargo xtask verify markdown-placement` (no Markdown but README.md in a code
tree, live documents at or under 500 lines). `ci-local` runs the task as one step, and the
`language-gates` job of `ci.yml` runs the three commands as separate steps. The
[documentation gates README](/tools/checks/documentation_checks/src/README.md) holds the
rules.

## Adding a rule

A new rule gets the next free number of its family, a pillar, one gate and a line in the rule
index of the [README](/documentation/standards/coding_standards/README.md). The gate is wired
into `cargo xtask ci ci-local` and into a GitHub job: a gate that only a local replay runs blocks no
push. A rule no tool can check is stated as unenforced, and only an
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) runtime rule may be MANUAL.
