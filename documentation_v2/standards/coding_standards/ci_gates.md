**Status:** live

# CI gates

Rules CI-1 and CI-2, the rules about the CI configuration itself, and the `verify-coding-standards`
task that bundles four of the code gates. Code comments and help strings that cite "§0.3" or
"§11" point here. Where every gate runs (the local replay, each GitHub job, the
[wave](/documentation_v2/glossary/n_to_z.md#wave) gate) is the gate matrix of
[Testing and CI](/documentation_v2/runbooks/testing_and_ci.md#gate-matrix); this page does not
repeat it.

## Rules

- **CI-2 (Debuggability) — `ci.yml` gates every push and pull request to `main`.**
  `.github/workflows/ci.yml` runs on every push and pull request to `main` with no path filter,
  unlike `contracts.yml` and `schema.yml`, which run only when their paths change. Its jobs:

  | Job | Runs | Rules |
  |---|---|---|
  | `website-api` | `cargo xtask mk rust-fmt`, `mk rust-clippy`, `mk rust-build`, `ci developer-tools-test`, then `ci website-api-test` (the API's `cargo test`) against a Postgres 18 service | FMT-1, GO-2, GO-8, GO-9, TEST-1 |
  | `map-engine` | `cargo xtask mk wasm-ci`: format, clippy with `-D warnings` on the host and `wasm32`, tests | FMT-1 |
  | `website-frontend` | `cargo xtask mk ci-local-leptos`: format, clippy for `wasm32`, tests, release Trunk build | TEST-2, TS-6 |
  | `schema` | `cargo xtask ci ci-local-schema`: generated types current, schema validation, `@contract` citations | TEST-3, ENF-3, ENF-4 |
  | `editorconfig` | `cargo xtask ci verify-editorconfig` | FMT-2 |
  | `language-gates` | `verify no-python`, `no-node`, `file-length`, `no-shell`, `ci-shell`, `engine-layers`, `ticket check --strict` | LANG-1, LANG-2, LANG-3, SIZE-3 |
  | `mod-gates-hosted` | `mod world-boot --selftest`, `verify staging-compose-paths`, `mission-rest-size-limits`, `ci-schema-parity` | none |

  `cargo xtask ci ci-local` replays the same gates locally, with the integration tests run by
  `cargo xtask ci rust-test-it` against the local database. Gate: CI-BLOCK, the workflow itself.
- **CI-1 (Debuggability) — No lint job hides old issues.** It forbade `only-new-issues: true` on
  the Go lint job, so that every lint finding in the tree fails, not only the new ones. Status:
  retired with the Go backend; clippy has no such switch, and every job lints the whole crate.

## verify-coding-standards

`cargo xtask ci verify-coding-standards` runs four gates in order and stops at the first failure:

1. `cargo xtask ci verify-doc-layout`: no Markdown under a `docs` folder in `apps/`,
   `contracts_v2/` or `assets_v2/` (the documentation standards own the rule).
2. `cargo xtask verify file-length`: SIZE-3.
3. `cargo xtask verify no-select-star`: no `SELECT *` or `RETURNING *` in the API's SQL, outside
   the two tables with no nullable column; the
   [database verifications README](/tools_v2/xtask/src/verifications/database/README.md) has the
   rule. It has no rule code.
4. `cargo xtask verify route-tags`: GO-7.

`ci-local` runs the task as one step. On GitHub, only `file-length` runs (in `language-gates`); the
doc-layout, `SELECT *` and route-tag gates run in `ci-local` and, for `route-tags`, the wave and
slice gates, but in no workflow.

## Adding a rule

A new rule gets the next free number of its family, a pillar, one gate and a line in the rule
index of the [README](/documentation_v2/standards/coding_standards/README.md). The gate is wired
into `cargo xtask ci ci-local` and into a GitHub job: a gate that only a local replay runs blocks no
push. A rule no tool can check is stated as unenforced, and only an
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) runtime rule may be MANUAL.
