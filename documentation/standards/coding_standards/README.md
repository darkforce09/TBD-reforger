**Status:** live

# Coding standards

The rules for how code is written across the repository's Rust crates, its
[EnfScript](/documentation/glossary/a_to_f.md#enfscript) mod and its tooling, each with a stable rule
code and the one gate that enforces it, or a plain statement that nothing does. Every developer
and AI agent that changes code reads the topic page for the layer they touch; how code is
documented is in the sibling [documentation standards](/documentation/standards/documentation_standards.md).

## Contents

```text
documentation/standards/coding_standards/
├── api_code_structure.md          GO-1 to GO-9: the API crate's layout, database errors, lints and route tags
├── api_errors_and_logging.md      ERR-1, ERR-2, ERR-4, ERR-5 and LOG-3: the error envelope, statuses, request logs
├── ci_gates.md                    CI-1 and CI-2, the ci.yml jobs, and the verify-coding-standards task (§0.3, §11)
├── enfusion_code_policy.md        ENF-1 to ENF-4: logging, authority comments, tags and samples in mod scripts
├── file_size_and_complexity.md    SIZE-1 to SIZE-3 and COMP-1: the line limits, the walk, function complexity
├── formatting.md                  FMT-1 to FMT-3: rustfmt and the root editorconfig (§7)
├── frontend_code.md               TS-1 to TS-7 and LOG-2: types, layers, errors and logging in the app
├── testing_bar.md                 TEST-1 to TEST-3: the least testing each layer ships with, and test placement
└── tooling_languages.md           LANG-1 to LANG-3: tooling is Rust; the shell, Python and Node script bans
```

## How it works

Each topic page states its rules in one form: the code, its pillar, the rule, then either its gate
or its status. The pages are prescriptive: **MUST**-level rules are stated as plain present-tense
requirements, and a forbidden pattern is listed under "Forbidden". Where a rule cannot be stated as
an exact number, command or tool, it is not a rule yet.

**Authority.** Running code wins over every document, then `CLAUDE.md` (its laws), then these
standards. Comment and tag rules (`@route`, `@contract`, `@authority`, doc-comment presence) belong
to the [documentation standards](/documentation/standards/documentation_standards.md); the
layer walls between the map crates, the graphics crates and the apps belong to the
[crate boundary rules](/documentation/standards/crate_boundary_rules.md). A code rule that
depends on one of those links to it and does not restate it.

**Rule codes are stable.** Code comments, help strings and CI step names cite rules by code
(`FMT-2`, `LANG-1`, `SIZE-3`, `GO-7`, "GO-2..8 analog"), so a code is never renumbered or reused.
A retired rule keeps its line in the index below. Several families carry the prefix of a language
the repository no longer holds (GO for the Go backend, TS for the TypeScript app); their rules now
state the Rust form, and the prefixes stay because the code cites them.

**Gates.** Each rule names at most one gate kind:

| Gate | Meaning |
|---|---|
| CI-BLOCK | a required GitHub job, or a test inside one, fails on a violation |
| CI-SCRIPT | a `cargo xtask verify …` or `cargo xtask ci …` command exits non-zero on a violation, run by `cargo xtask ci ci-local` and by a job or the [wave](/documentation/glossary/n_to_z.md#wave) gate |
| MANUAL | only a run in [Workbench](/documentation/glossary/n_to_z.md#workbench) or on a server can show it; allowed for [Enfusion](/documentation/glossary/a_to_f.md#enfusion) runtime rules only (ENF-1, ENF-2) |
| none | "live, unenforced": the rule binds, and no tool checks it; or "retired": the rule no longer binds |

There is no allowlist gate: no file may exempt a path from a rule (CLAUDE.md law 7).

**Pillars.** Every rule serves one: Scalability (Sc), workable at ten times the size, data or
team; Readability (Re), understandable without the history; Usability (Us), a correct and
predictable contract for the consumer; Debuggability (De), a failure that says what and why.

### Rule index

| Code | Pillar | Rule | Status and gate | Page |
|---|---|---|---|---|
| CI-1 | De | no lint job reports only new issues | retired | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md) |
| CI-2 | De | `ci.yml` gates every push and pull request to `main` | CI-BLOCK, the workflow | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md) |
| COMP-1 | Re | at most 15 independent paths per function | live, unenforced | [file_size_and_complexity.md](/documentation/standards/coding_standards/file_size_and_complexity.md) |
| ENF-1 | De | explicit log levels; no hot-path logging; development switches off | MANUAL | [enfusion_code_policy.md](/documentation/standards/coding_standards/enfusion_code_policy.md) |
| ENF-2 | De | an authority gate carries its reason | MANUAL | [enfusion_code_policy.md](/documentation/standards/coding_standards/enfusion_code_policy.md) |
| ENF-3 | Re | `@contract` citations in `.c` files resolve | CI-SCRIPT, `cargo xtask ci verify-citations` | [enfusion_code_policy.md](/documentation/standards/coding_standards/enfusion_code_policy.md) |
| ENF-4 | Us | a parsed JSON document has a validating golden sample | CI-SCRIPT, `cargo xtask ci schema-validate` | [enfusion_code_policy.md](/documentation/standards/coding_standards/enfusion_code_policy.md) |
| ERR-1 | Us | the error body is `{"error"}` with an optional `details` | live, held by `ApiError`; no gate | [api_errors_and_logging.md](/documentation/standards/coding_standards/api_errors_and_logging.md) |
| ERR-2 | Us | status codes follow the table | live, unenforced | [api_errors_and_logging.md](/documentation/standards/coding_standards/api_errors_and_logging.md) |
| ERR-4 | Us | no error key besides `error` and `details` | live, unenforced | [api_errors_and_logging.md](/documentation/standards/coding_standards/api_errors_and_logging.md) |
| ERR-5 | Us | a named integration test per status class | live, unenforced | [api_errors_and_logging.md](/documentation/standards/coding_standards/api_errors_and_logging.md) |
| FMT-1 | Re | source is formatter clean | CI-BLOCK, `cargo xtask mk rust-fmt` | [formatting.md](/documentation/standards/coding_standards/formatting.md) |
| FMT-2 | Re | the root `.editorconfig` governs whitespace | CI-BLOCK, `cargo xtask ci verify-editorconfig` | [formatting.md](/documentation/standards/coding_standards/formatting.md) |
| FMT-3 | Re | one formatter of record per language | retired; FMT-1 covers it | [formatting.md](/documentation/standards/coding_standards/formatting.md) |
| GO-1 | Sc | logic in `services/`; handlers do HTTP only | live, unenforced | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-2 | De | a used database result has its error handled | CI-BLOCK, `cargo xtask mk rust-clippy` | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-3 | De | a best-effort write logs and gives its reason | live, unenforced | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-4 | De | a propagated error keeps its cause | retired; the type system carries it | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-5 | Us | a unique violation answers `409` by SQLSTATE | live, unenforced | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-6 | Re | every public item has a doc comment | retired; the documentation standards own it | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-7 | Re | every routed handler's `@route` tag matches its route | CI-SCRIPT, `cargo xtask verify route-tags` | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-8 | De | the static analyser runs with every check on | CI-BLOCK, `cargo xtask mk rust-clippy` | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| GO-9 | Sc | handlers reach other code through services and models | CI-BLOCK, the API's `architecture_rules.rs` tests | [api_code_structure.md](/documentation/standards/coding_standards/api_code_structure.md) |
| LANG-1 | Sc | new tooling is Rust; no tracked shell or Make | CI-SCRIPT, `cargo xtask verify no-shell` | [tooling_languages.md](/documentation/standards/coding_standards/tooling_languages.md) |
| LANG-2 | Sc | no tracked Python and no `python3` calls | CI-SCRIPT, `cargo xtask verify no-python` | [tooling_languages.md](/documentation/standards/coding_standards/tooling_languages.md) |
| LANG-3 | De | the language bans are hard zeros | CI-SCRIPT, both commands | [tooling_languages.md](/documentation/standards/coding_standards/tooling_languages.md) |
| LOG-2 | De | no debug console logging in the app | live, unenforced | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| LOG-3 | De | a failed request is logged with path, status and duration | live, held by the access-log middleware; no gate | [api_errors_and_logging.md](/documentation/standards/coding_standards/api_errors_and_logging.md) |
| SIZE-1 | Sc | a soft warning at 600 lines | retired; SIZE-3 replaces it | [file_size_and_complexity.md](/documentation/standards/coding_standards/file_size_and_complexity.md) |
| SIZE-2 | Sc | file-level exemptions | retired; none exist | [file_size_and_complexity.md](/documentation/standards/coding_standards/file_size_and_complexity.md) |
| SIZE-3 | Sc | production Rust and pinned mod EnfScript ≤ 500 lines, tests ≤ 1000, no exemption | CI-SCRIPT, `cargo xtask verify file-length` | [file_size_and_complexity.md](/documentation/standards/coding_standards/file_size_and_complexity.md) |
| TEST-1 | De | a handler change passes the API's tests against Postgres | CI-BLOCK, the `api` job | [testing_bar.md](/documentation/standards/coding_standards/testing_bar.md) |
| TEST-2 | De | non-trivial app logic has a unit test | CI-BLOCK, the `frontend` job | [testing_bar.md](/documentation/standards/coding_standards/testing_bar.md) |
| TEST-3 | Us | a schema change ships a fixture and a green schema gate | CI-BLOCK, the `schema` job | [testing_bar.md](/documentation/standards/coding_standards/testing_bar.md) |
| TS-1 | De | the compiler runs in its strictest mode | retired; the Rust compiler carries it | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-2 | Sc | layer boundaries hold | CI-SCRIPT, `cargo xtask verify frontend-layering` and the crate firewalls (`cargo xtask verify crate-tiers`) | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-3 | De | contract data is fully typed | retired; the Rust type system carries it | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-4 | Us | a failed request shows the user an error | live, unenforced | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-5 | Re | every exported contract item has a doc comment | retired; the documentation standards own it | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-6 | Re | a DTO mirrors its API model exactly | CI-BLOCK, the R-api golden tests | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| TS-7 | Us | no failure is swallowed | CI-SCRIPT, the app's clippy `-D warnings` in `cargo xtask mk ci-local-leptos` | [frontend_code.md](/documentation/standards/coding_standards/frontend_code.md) |
| WS-1 | Sc | every manifest is a member; judged crates declare their layout, tier and category edges; the firewalls hold | CI-SCRIPT, `cargo xtask verify crate-tiers` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#ws-1-crate-tiers) |
| WS-2 | Sc | a judged library crate keeps the crate anatomy | CI-SCRIPT, `cargo xtask verify crate-anatomy` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#ws-2-crate-anatomy) |
| WS-3 | Sc | every `.rs` file in a member's test folders is compiled by one of its targets | CI-SCRIPT, `cargo xtask verify test-file-reachability` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#ws-3-test-file-reachability) |
| WS-4 | Sc | no frontend crate depends on a higher layer or a peer page crate, and the foundation and Mission Creator crates keep their crate orders | CI-SCRIPT, `cargo xtask verify frontend-layering` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#ws-4-frontend-layering) |
| WS-5 | Sc | every leptos crate has exactly one `@source` line in the app stylesheet, and no line is stale | CI-SCRIPT, `cargo xtask verify tailwind-sources` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#ws-5-tailwind-sources) |

46 codes: 23 gated (10 CI-BLOCK, 13 CI-SCRIPT, TS-2 counted for its engine wall), 2 MANUAL, 12
live with no gate (ERR-1 and LOG-3 held by construction), 9 retired. The code ERR-3 is not used.

### Section numbers cited by code

Comments and help strings cite some rules by the section number of a single-file layout; each
now maps to a page:

| Cited as | Cited by | Page |
|---|---|---|
| §0.3 (CI-2) | `.github/workflows/ci.yml` header | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md) |
| §7 (FMT-2) | `.editorconfig` header; the `verify-editorconfig` help in `tools/commands/ci_task_catalog/src/task_definitions.rs` | [formatting.md](/documentation/standards/coding_standards/formatting.md) |
| §11 | the `verify-coding-standards` help in `tools/commands/ci_task_catalog/src/task_definitions.rs` | [ci_gates.md](/documentation/standards/coding_standards/ci_gates.md#verify-coding-standards) |

### Before a commit

- Rust in any crate: `cargo xtask mk rust-fmt` and `cargo xtask mk rust-clippy` for the API,
  `cargo xtask mk wasm-ci` for the wasm32 crates, `cargo xtask mk ci-local-leptos` for the app; files
  within SIZE-3; unit tests in sibling `tests/` files.
- The API: handlers thin, errors through `ApiError`, a `409` for a unique violation, a `@route`
  tag on every routed handler; `cargo xtask db test-it` green.
- The app: DTOs mirror the API models and their goldens pass; failures reach the user.
- Mod scripts: log levels explicit, development switches off, authority gates commented,
  `cargo xtask mod compile` clean.
- Contracts: fixture added, `cargo xtask ci ci-local-schema` green.
- Always: `cargo xtask ci ci-local` green (after `cargo xtask db up`), and the documentation of the
  change in the same commit ([commit checklist](/documentation/standards/commit_checklist.md)).

## Code

- [Language ban gates](/tools/checks/repository_checks/src/language_bans/) — LANG-1 to LANG-3 and
  SIZE-3.
- [Architecture verifications](/tools/checks/repository_checks/src/architecture/) — GO-7, and the
  engine wall of TS-2.
- [Schema gates](/tools/commands/schema_tooling/src/schema_checks/) — ENF-3, ENF-4, TEST-3.
- [CI task commands](/tools/commands/ci_task_catalog/src/) — `ci-local`, `verify-coding-standards`,
  `verify-editorconfig` (FMT-2).
- [API layout tests](/apps/api/src/tests/) — GO-9 and the API's test placement.
- [Handler errors](/crates/api/api_foundation/src/error_handling/) — ERR-1 and ERR-4.
- [Middleware](/crates/api/api_http_layer/src/middleware/) — LOG-3.
- [Workflows](/.github/workflows/) — CI-2 and every CI-BLOCK gate.

## Boundaries

- Depends on: `CLAUDE.md` laws 3, 7 and 9; the gates named in each rule, as the code under Code
  implements them; the [documentation standards](/documentation/standards/documentation_standards.md)
  and [crate boundary rules](/documentation/standards/crate_boundary_rules.md) for the rules
  they own.
- Used by: rule codes and this README's path in `.github/workflows/ci.yml`, `.editorconfig`,
  `tools/commands/ci_task_catalog/src/task_definitions.rs`,
  `tools/checks/repository_checks/src/architecture/route_tags.rs`,
  `tools/checks/repository_checks/src/language_bans/shell_scripts.rs` and
  `tools/map_assets/map_raster_pipeline/src/satellite_archive_container.rs`; the READMEs of
  the language ban, file length, architecture and verify folders; the
  [Testing and CI](/documentation/runbooks/testing_and_ci.md) runbook, whose gate matrix uses
  the codes; the [commit checklist](/documentation/standards/commit_checklist.md).
- Rules: a rule code is never renumbered or reused; a rule names one gate or says it is
  unenforced; MANUAL is for Enfusion runtime rules only; a page restates no gate detail that a code
  README already holds, and links it instead.

## Related documentation

- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — how to run each gate, and where
  each runs.
- [Documentation standards](/documentation/standards/documentation_standards.md) — comment and
  tag rules, and where Markdown lives.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate walls
  `cargo xtask verify crate-tiers` holds.
- [Where does X go?](/documentation/standards/where_does_x_go.md) — the home of each kind of
  file.
- [Commit checklist](/documentation/standards/commit_checklist.md) — what a commit carries.
