**Status:** live

# Tooling architecture

How the developer tooling in `tools_v2/` is put together: the four crates and the npm package, the
direction their dependencies run, the invariants that keep them apart, and the verification
surface they build. Developers and AI agents read it before adding a command, a check or a
module to the tooling; each crate's README then says what its own folders hold.

## Where it lives

- Code: [`tools_v2/`](/tools_v2/README.md) with
  [`xtask/`](/tools_v2/xtask/README.md), [`ticket-engine/`](/tools_v2/ticket-engine/README.md),
  [`verification-core/`](/tools_v2/verification-core/README.md),
  [`developer-tools/`](/tools_v2/developer-tools/README.md) and
  [`enfusion_mcp_node_package/`](/tools_v2/enfusion_mcp_node_package/README.md); the
  [ticketboard](/apps/ticketboard/README.md) in `apps/ticketboard/` links `ticket-engine`.
- Entry: `cargo xtask`, the alias `run --package xtask --` in `.cargo/config.toml`; the six
  `developer-tools` executables (`enf`, `gate`, `mcpd`, `world`, `map`, `capture`).
- Related features: the [ticket engine documentation](/documentation_v2/tools_v2/ticket-engine/README.md),
  the [developer tools documentation](/documentation_v2/tools_v2/developer-tools/README.md) and the
  [ticketboard documentation](/documentation_v2/ticketboard/README.md).

## Behaviour

### The shape

| Unit | Kind | Owns |
|---|---|---|
| `xtask` | binary | the command router: database, deploys, mod servers, map helpers, the platform factory, and every repository verification and CI task |
| `ticket-engine` | library | the [ticket](/documentation_v2/glossary/n_to_z.md#ticket) registry: typed storage and operations, validation, `queue.json` and the roadmap and gap-analysis markers, the [wave](/documentation_v2/glossary/n_to_z.md#wave) lock, run receipts and estimates |
| `verification-core` | library | the fail-closed primitives: verdicts, findings, pattern scans, child processes with deadlines, the report and the shared verification lock |
| `developer-tools` | library and six binaries | the heavy offline work: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) archives and the script oracle, the MCP broker, the headless browser gates, the blueprint compiler, the world export and map raster pipelines, map verification |
| `enfusion_mcp_node_package` | npm data, not a crate | the pinned `enfusion-mcp` server that `mcpd` and `cargo xtask mcp` start |

One word names one thing. A gate is a repository verification that reaches a verdict; the
headless browser harness is `browser_testing`; the assertion primitives are `verification-core`;
the ticket rules are `ticket_engine::validation`.

### Dependency direction

```text
verification-core        ticket-engine ◀──────── ticketboard (apps/ticketboard)
        ▲                      ▲                 (foundational: no workspace dependency)
        │                      │
        └──────── xtask ───────┘
                    │
                    ▼
             developer-tools ──▶ website-map-engine (apps/website/map-engine)
                    │
                    └── mcpd starts ──▶ enfusion_mcp_node_package (after npm ci)
```

1. `verification-core` and `ticket-engine` depend on no workspace crate, so either is read, tested
   and reasoned about alone (`foundational_engines_have_no_workspace_dependencies`).
2. `developer-tools` never depends on `xtask`: the router calls the services, never the reverse
   (`tooling_dependency_direction_is_enforced`).
3. `xtask` never depends on `website-map-engine` or `website-graphics-engine` directly; the map
   engine reaches it only through `developer-tools`, so a graphics change does not rebuild the
   command surface (same test).
4. Ticket logic has one owner: the xtask `ticket` and `wave` groups delegate to `ticket_engine`
   (`ticket_implementations_have_one_owner`), and the ticketboard reads through its public model.

The four tests live in `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`; the six
binary names and the three layout modules are pinned by
`the_tooling_tree_holds_its_executables_manifests_and_layout_modules` in the same file.

### One owner per path

Only three layout modules spell a repository path literal:

| Module | Owns |
|---|---|
| `tools_v2/ticket-engine/src/repository.rs` | the ticket registry, its schemas, receipts and estimates, and in its `documentation` submodule every documentation path the registry writes or reads |
| `tools_v2/xtask/src/core/repository_layout.rs` | the deploy tree, the server profiles, the MCP fixtures, the worktree base, the verdict receipts and the documentation pointers the router prints |
| `tools_v2/developer-tools/src/repository_layout.rs` | the contract and asset trees, the Enfusion index, the operations logs and the npm package folder |

`xtask` resolves the checkout root through `ticket_engine::repository::find_repo_root`, which walks
up to `.ai/tickets/ROOT`, so a command run in a linked worktree reads that worktree's files.
`developer-tools` cannot depend on `ticket-engine` and keeps its own lookup in
`repository_paths.rs`. `only_a_layout_module_spells_a_repository_path` in
`tools_v2/xtask/src/tests/tooling_prose_rules.rs` holds the rule for production source.

### One outcome vocabulary

Every verification returns a `verification_core::Verdict`: held, failed, or did not run. A check
whose input is missing, whose tool is absent, whose child was killed or whose corpus will not load
reports did-not-run, never a pass; an empty input is never a clean tree. `Verdict` has no
`From<bool>` and no `is_ok()`, so no expression folds the third outcome into the first. A `Report`
turns the verdicts into the exit code, 0 held, 1 failed, 2 did not run, with 2 outranking 1. This
is what makes a green run evidence; the [verification core README](/tools_v2/verification-core/README.md#how-it-works)
gives the table and the lock rules.

Gates that must not overlap (the platform wave gate, the MCP broker start) serialise on one
`flock` at `target/.repository-verification.lock` in the primary checkout, found through
`git rev-parse --git-common-dir`, so linked worktrees share it. Running out of time is a refusal,
never an unserialised run.

### The verification surface

`cargo xtask verify <name>` runs one repository check, `cargo xtask schema <name>` one contract or
map-asset gate, and `cargo xtask ci <task>` one row of the CI task table; `cargo xtask ci
ci-local` replays the composite the CI jobs run. The three callers reach the same functions under
`tools_v2/xtask/src/verifications/`, one folder per invariant, which the
[verifications README](/tools_v2/xtask/src/verifications/README.md) maps. Two checks guard the
surface itself: `verify ci-schema-parity` pins the CI schema job and the `ci-local` rows to the
full gate set, reading the task table in process, and the documentation gates
(`readme-coverage`, `markdown-placement`, `link-check`) hold the README and link structure the
documentation standard sets.

### Structural limits and prose

- Production files stay under 500 lines, test files under 1,000, `tools_v2/xtask/src/main.rs`
  under 150, the executables' `src/bin/` files under 250 and the editor smoke scenarios under 450
  (`tooling_source_files_stay_below_their_structural_limits`). No exemption mechanism exists
  (`file_size_allowlist_is_permanently_retired`).
- Unit tests live in sibling files declared with `#[path = "tests/…"]`; an inline test module is
  refused by a syn walk that also enters macro bodies and function-local modules
  (`tooling_test_modules_live_in_separate_files`).
- Every tracked file under `tools_v2/` describes the present: no ticket ids, no history words, no
  retired crate or file spellings, no shell, Python or Node script names, and every Rust file it
  names exists (`tools_v2/xtask/src/tests/tooling_prose_rules.rs`). Commit history owns history.

### Data beside the crates

`enfusion_mcp_node_package/` sits outside every crate root on purpose: `verify file-length` walks
whole crate folders and the structural walk refuses symlinks under `src/`, so a vendored `.rs`
inside an installed `node_modules/` would be subject to both. The other data folders
(`xtask/deploy/`, `xtask/dedicated_server_profiles/`, `xtask/fixtures/`,
`developer-tools/fixtures/`, `developer-tools/test_fixtures/`, `ticket-engine/tests/fixtures/`)
sit beside the code that reads them, and a layout module names each once.

### Known discrepancies

- The dependency rule says the ticketboard reads tickets through `ticket-engine`'s public model
  (`apps/ticketboard/README.md`, How it works) — the ticketboard loads tickets without
  `Corpus::load`'s vocabulary and id-to-file checks and keeps its own copies of the wave-lock
  types, the scope vocabulary parsing and the estimate validation
  (`apps/ticketboard/src/execution_metrics/estimated/validation.rs`).
- `developer-tools` has two root lookups, `repository_paths::find_repo_root` and
  `browser_testing::server::repo_root` (`tools_v2/developer-tools/src/browser_testing/server.rs:434`),
  and the world export and raster code import the second.

## Data

- `.cargo/config.toml`: the `xtask` alias.
- `tools_v2/developer-tools/gate-env.json`: the pinned Chromium, toolchain versions and limits
  `gate doctor` checks.
- `target/.repository-verification.lock` in the primary checkout: the shared verification lock
  (`GATE_LOCK_RELPATH` in `verification-core`).
- The CI workflows in `.github/workflows/` (`ci.yml`, `contracts.yml`, `editor-gates.yml`,
  `mod-gates.yml`, `schema.yml`), which call `cargo xtask` tasks.

## Design

The tooling is layered so the cheapest crates carry the rules everything else leans on: the two
foundational libraries build in seconds and know nothing of the repository's products, the router
depends on them and on one heavy crate, and only that heavy crate links the map engine. A check
that cannot run says so, so a green run is proof. Paths, limits and prose rules are held by tests
rather than by review.

## Open work

- [T-1137 — Gate ticket-engine, verification-core, ticketboard and fleet agent tests and clippy](/.ai/tickets/T-1137.toml)
  (idea, no plan): CI, `ci-local` and the wave gate run `cargo test` and clippy for the two
  foundational crates, the ticketboard and the fleet host agent.
- [T-1142 — Decide whether the ticketboard imports the ticket-engine logic it copies](/.ai/tickets/T-1142.toml)
  (idea, no plan): the ticketboard stops copying engine logic, or the copy is recorded as intended.
- [T-1133 — Consolidate developer-tools duplicate helpers, repo-root lookup and fixture paths](/.ai/tickets/T-1133.toml)
  (idea, no plan): one root lookup in `developer-tools`, fixture paths through the layout module,
  and no `cargo run -p xtask` child from inside the crate.
- [T-1130 — Tidy xtask verifications: redundant check, scattered paths, wave gate](/.ai/tickets/T-1130.toml)
  (idea, no plan): verification paths move into the layout module.
- [T-1115 — Consolidate xtask host-bridge copies and move slice worktree tests](/.ai/tickets/T-1115.toml)
  (idea, no plan): one owner for container detection and host-bridge wrapping.
- [T-1114 — Rename xtask files named after one helper they hold](/.ai/tickets/T-1114.toml) and
  [T-1132 — Rename developer-tools files named after one helper they hold](/.ai/tickets/T-1132.toml)
  (idea, no plan): file names that say what the file holds.
- [T-1118 — Rewrite xtask build and ci comments narrating the retired Makefile](/.ai/tickets/T-1118.toml),
  [T-1119 — Rewrite xtask help, error and doc texts contradicting the code](/.ai/tickets/T-1119.toml),
  [T-1134 — Rewrite developer-tools help texts and comments contradicting the code](/.ai/tickets/T-1134.toml)
  and [T-1139 — Rewrite stale ticket-engine and verification-core comments](/.ai/tickets/T-1139.toml)
  (idea, no plan): help texts and comments that match the code.
- [T-1004 — Comment hygiene: design citations, ticket ids and history words](/.ai/tickets/T-1004.toml)
  (idea, no plan): the prose rules also catch design-document citations and reach ticket ids in
  Rust and EnfScript comments.
- [T-1120 — Remove dead xtask code and move the font table generator](/.ai/tickets/T-1120.toml)
  (idea, no plan): `gen font-table` leaves the file-length verification.
- [T-1147 — Enforce or remove the unread enfusion_mcp_node_package .nvmrc](/.ai/tickets/T-1147.toml)
  (idea, no plan): the pinned Node version is checked, or the file goes.

## Decisions

- The foundational crates take no workspace dependency: they stay fast to build and test, and
  every other crate can use them without a cycle.
- `xtask` reaches the map engine only through `developer-tools`: the command surface does not
  rebuild when rendering code changes.
- Three outcomes, never two: a missing prerequisite must not pass, so "did not run" has its own
  exit code and outranks a failure.
- Rules live in tests, not in review: dependency direction, path ownership, limits and prose are
  asserted on every `cargo test -p xtask`.
