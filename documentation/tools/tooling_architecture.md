**Status:** live

# Tooling architecture

How the developer tooling in `tools/` is put together: the crates and the npm package, the
direction their dependencies run, the invariants that keep them apart, and the verification
surface they build. Developers and AI agents read it before adding a command, a check or a
module to the tooling; each crate's README then says what its own folders hold.

## Where it lives

- Code: [`tools/`](/tools/README.md) with
  [`xtask/`](/tools/xtask/README.md), the [ticket crates](/tools/tickets/README.md),
  the [foundation crates](/tools/foundation/README.md)
  ([`verification_core/`](/tools/foundation/verification_core/README.md),
  [`process_runner/`](/tools/foundation/process_runner/README.md),
  [`repository_laws/`](/tools/foundation/repository_laws/README.md)),
  [`developer_tools/`](/tools/developer_tools/README.md) and
  [`enfusion_mcp_node_package/`](/tools/enfusion_mcp_node_package/README.md); the
  [ticketboard](/tools/tickets/ticketboard_desktop/README.md) in `tools/tickets/ticketboard_desktop/` links `ticket_model`.
- Entry: `cargo xtask`, the alias `run --package xtask --` in `.cargo/config.toml`; the eight
  `developer_tools` executables (`enf`, `gate`, `mcpd`, `world`, `map`, `capture`,
  `acknowledgement-dropping-relay`, `staging-load`).
- Related features: the [ticket crates documentation](/documentation/tools/tickets/README.md),
  the [developer tools documentation](/documentation/tools/developer_tools/README.md) and the
  [ticketboard documentation](/documentation/tools/tickets/ticketboard_desktop/README.md).

## Behaviour

### The shape

| Unit | Kind | Owns |
|---|---|---|
| `xtask` | binary | the command router: database, deploys, mod servers, map helpers, the platform factory, and every repository verification and CI task |
| `ticket_model` | library, `tools/tickets` tier 2 | the typed [ticket](/documentation/glossary/n_to_z.md#ticket), its canonical TOML encoding, the corpus store over `.ai/tickets/T-*.toml`, the scope vocabulary and the ticket-domain paths |
| `ticket_metrics` | library, `tools/tickets` tier 3 | slice-run receipts under the registry's metrics folder (`METRICS_DIR`) and token estimates under `.ai/tickets/estimates/` |
| `ticket_wave_lock` | library, `tools/tickets` tier 3 | the [wave](/documentation/glossary/n_to_z.md#wave) lock compiler, reader and checker, and its history |
| `ticket_registry` | library, `tools/tickets` tier 4 | the registry view, typed operations, validation, `queue.json` and the roadmap and gap-analysis markers, the corpus pins and the bodies of the `cargo xtask ticket` verbs |
| `verification_core` | library, `tools/foundation` tier 0 | the fail-closed primitives: verdicts, findings, pattern scans, the report and the shared verification lock |
| `process_runner` | library, `tools/foundation` tier 1 | child processes in their own process group with deadlines and honest statuses, the container-to-host bridge, the ssh transport |
| `repository_laws` | library, `tools/foundation` tier 1 | the structural engineering laws as pure checks: file length, test placement, exemptions, the workspace laws (the crate tiers among them, which hold every member's dependency direction) |
| `repository_layout` | library, `tools/foundation` tier 0 | the repository locations more than one tool names |
| `deploy_settings` | library, `tools/foundation` tier 2 | the `deploy/deploy.env` reader: the precedence of the file over the process environment, the deploy host, its remote folders and the ssh transport choice |
| `tool_test_support` | library, `tools/foundation` tier 1, dev-dependency only | the environment and working-directory locks and the test checkout root the tool tests share |
| `enfusion_pak` | library, `tools/enfusion` tier 0 | the [Enfusion](/documentation/glossary/a_to_f.md#enfusion) `.pak` archive reader: one parser and one decompressor under the blueprint and world policies, the loose and layered sources |
| `blueprint_compiler` | library, `tools/map_assets` tier 6 | the building-blueprint compiler: voxel dumps and game models to blueprints, occlusion sidecars, the prefab occluder library and the blueprint archive, behind the `cargo xtask map` blueprint, BVH and model commands |
| `map_asset_verification` | library, `tools/map_assets` tier 7 | the map asset gates: the terrain manifest, the prefab BLAS library, the labels, the elevation anchors and the map-object goldens behind `cargo xtask schema` and `verify blas-manifest`, and the world line-of-sight probe behind `cargo xtask map world-los` |
| `enfusion_script_index` | library, `tools/enfusion` tier 2 | the script oracle behind `enf`: symbol indexes, lookups, the citation and capability gates, vanilla extraction, and the vanilla page mirrors behind `cargo xtask fetch` |
| `chrome_devtools_protocol` | library, `tools/browser_testing` tier 1 | the Chrome DevTools Protocol client: Chromium discovery and headless launch in its own process group, pages over one WebSocket each, the gate font cache |
| `browser_gate_suites` | library, `tools/browser_testing` tier 5 | the headless browser gates of the single-page app: the static server, the DOM oracle, route drift, the Mission Creator smokes, the data viewer gate, the ballistics agreement and offline mortar gates, the capture rig, the doctor, and the `gate` and `capture` command lines |
| `map_raster_pipeline` | library, `tools/map_assets` tier 7 | the map raster pipeline behind the `map` binary: the orthophoto stitch, the satellite container and tile pyramids, the cartographic render, the label sets and archives, the water archives and the world-glyph atlas; never in xtask's closure |
| `developer_tools` | eight binaries, no library | one-line `main`s: `enf` and `mcpd` over the Enfusion crates, `gate` and `capture` over `browser_gate_suites`, `world` and `map` over `world_export_pipeline` and `map_raster_pipeline`, and `acknowledgement-dropping-relay` and `staging-load` over the staging crates |
| `enfusion_mcp_node_package` | npm data, not a crate | the pinned `enfusion-mcp` server that `mcpd` and `cargo xtask mcp` start |

One word names one thing. A gate is a repository verification that reaches a verdict; the
headless browser harness is `browser_gate_suites` over `chrome_devtools_protocol`; the assertion primitives are `verification_core`;
the ticket rules are `ticket_registry::validation`.

### Dependency direction

```text
verification_core ◀── process_runner, repository_laws        (tools/foundation, tiers 0 and 1)
        ▲                      ▲
        │                      │          ticket crates ◀──── ticketboard (tools/tickets/ticketboard_desktop)
        └──────── xtask ───────┴───────────────┘
                    │
                    ▼ runs as child processes
             developer_tools (binaries only)
                    │
                    ├── mcpd ──▶ enfusion_mcp_broker (tools/enfusion) ──starts──▶ enfusion_mcp_node_package (after npm ci)
                    ├── world, map ──▶ world_export_pipeline, map_raster_pipeline (tools/map_assets)
                    └── gate, capture ──▶ browser_gate_suites ──▶ chrome_devtools_protocol (tools/browser_testing)
```

1. A `tools/foundation` crate depends only on lower `tools/foundation` crates (`process_runner`
   and `repository_laws` on `verification_core`, which depends on none) and on the checkout-root
   finder `repository_root` (`foundation_crates_depend_only_on_lower_foundation_crates`), and a
   `tools/tickets` crate only on `tools/foundation` crates, on `time_source`, `content_digest`,
   `newtype_ids` and `repository_root`, and on
   ticket crates of a lower tier
   (`ticket_crates_depend_only_on_foundations_and_lower_ticket_crates`), so each is read, tested
   and reasoned about without the tools above it.
2. `xtask` and `developer_tools` are binary-only packages whose workspace dependencies are tool
   crates alone (the checkout-root finder comes through `repository_layout`'s prelude); neither depends on the other, and no member depends on either
   (`tooling_dependency_direction_is_enforced`).
3. No tokio, axum, reqwest, resvg or image enters xtask's dependency closure (rule 6 of `cargo xtask verify crate-tiers`):
   the async servers and the raster crates run behind the `developer_tools` binaries, so neither
   a server nor an image codec rebuilds the command surface.
4. Ticket logic has one owner: the xtask `ticket` group delegates to `ticket_registry` and the
   `wave` group to `ticket_wave_lock` (`ticket_implementations_have_one_owner`), and the
   ticketboard reads through the public model of `ticket_model`.

The five tests live in `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`, a
test file of the `repository_checks` crate; its structural rules (line limits, sibling test files)
cover every tool crate found by folder — each `tools/<name>` and `tools/<category>/<name>` holding
a `Cargo.toml` (`tooling_crate_folders_are_found_by_folder`). The eight binary names and the layout
modules are pinned by `the_tooling_tree_holds_its_executables_manifests_and_layout_modules` in the
same file.

### One owner per path

Only the `repository_layout` crate and the tools' own layout modules spell a repository path
literal. A tool's own layout module declares itself on its first line
(`//! The repository locations only …`), so the rule finds it without a list
(`layout_modules_are_the_shared_crate_and_the_self_declared_modules`):

| Module | Owns |
|---|---|
| `tools/foundation/repository_layout` | the locations more than one tool names: the ticket registry files, the artifact tree, the reference lanes and their folders, the contract and map-asset trees, the enfusion-mcp npm package, the browser gate pins, the documentation root, the roadmap and gap analysis, the deploy tree, the server profiles, the MCP fixtures, the runbooks and documentation areas the commands name, and the build output folder with its purpose subfolders |
| `tools/tickets/ticket_model/src/repository.rs` | the handoff document, the sparse-checkout sets, and in its `documentation` submodule the documents only the ticket domain names |
| `tools/map_assets/map_raster_pipeline/src/decision_record_locations.rs` | the decision records of the inland-water, aerial-orthophoto and cartographic lanes |
| `tools/enfusion/enfusion_script_index/src/script_index_layout.rs` | the Enfusion symbol index and the capability verdict table |
| `tools/browser_testing/browser_gate_suites/src/gate_layout.rs` | the map-asset mounts of the gates' server and the editor gate runbook |

Every tool resolves the checkout root through `repository_root::find_repository_root`
(`crates/foundation/repository_root`, the one root walk of the workspace, which the API's and the
frontend crates' tests use as well; xtask reaches it through `repository_layout::prelude`), which walks up to `.ai/tickets/ROOT`, so a command run in a
linked worktree reads that worktree's files; a working directory outside any checkout is an
error, never a guessed folder. `tools/foundation/repository_layout` spells the locations more
than one tool names: the ticket registry files, the artifact tree, the reference lanes, the documentation root
and the build output folder. `only_a_layout_module_spells_a_repository_path` in
`tools/checks/repository_checks/src/tests/tooling_prose_rules.rs` holds the rule for production source.

### One outcome vocabulary

Every verification returns a `verification_core::Verdict`: held, failed, or did not run. A check
whose input is missing, whose tool is absent, whose child was killed or whose corpus will not load
reports did-not-run, never a pass; an empty input is never a clean tree. `Verdict` has no
`From<bool>` and no `is_ok()`, so no expression folds the third outcome into the first. A `Report`
turns the verdicts into the exit code, 0 held, 1 failed, 2 did not run, with 2 outranking 1. This
is what makes a green run evidence; the [verification core README](/tools/foundation/verification_core/README.md#how-it-works)
gives the table and the lock rules.

Gates that must not overlap (the platform wave gate, the MCP broker start) serialise on one
`flock` at `target/.repository-verification.lock` in the primary checkout, found through
`git rev-parse --git-common-dir`, so linked worktrees share it. Running out of time is a refusal,
never an unserialised run.

### The verification surface

`cargo xtask verify <name>` runs one repository check, `cargo xtask schema <name>` one contract or
map-asset gate, and `cargo xtask ci <task>` one row of the CI task table; `cargo xtask ci
ci-local` replays the composite the CI jobs run. The three callers reach the same functions in
the check crates under `tools/checks/` and the command crates under `tools/commands/`, which the
[verify group README](/tools/xtask/src/commands/verify/README.md) maps verb by verb. Two checks guard the
surface itself: `verify ci-schema-parity` pins the CI schema job and the `ci-local` rows to the
full gate set, reading the task table in process, and the documentation gates
(`readme-coverage`, `markdown-placement`, `link-check`) hold the README and link structure the
documentation standard sets.

### Structural limits and prose

- Production files stay under 500 lines, test files under 1,000, `tools/xtask/src/main.rs`
  under 150, the executables' `src/bin/` files under 250 and the editor smoke scenarios under 450
  (`tooling_source_files_stay_below_their_structural_limits`). No exemption mechanism exists
  (`file_size_allowlist_is_permanently_retired`).
- Unit tests live in sibling files declared with `#[path = "tests/…"]`; an inline test module is
  refused by a syn walk that also enters macro bodies and function-local modules
  (`tooling_test_modules_live_in_separate_files`).
- Every tracked file under `tools/` describes the present: no ticket ids, no history words, no
  retired crate or file spellings, no shell, Python or Node script names, and every Rust file it
  names exists (`tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`). Commit history owns history.

### Data beside the crates

`enfusion_mcp_node_package/` sits outside every crate root on purpose: `verify file-length` walks
whole crate folders and the structural walk refuses symlinks under `src/`, so a vendored `.rs`
inside an installed `node_modules/` would be subject to both. The other data folders
(`xtask/deploy/`, `xtask/dedicated_server_profiles/`, `xtask/fixtures/`,
`developer_tools/fixtures/`, `map_assets/blueprint_compiler/test_fixtures/`, `tickets/ticket_metrics/tests/fixtures/`)
sit beside the code that reads them, and a layout module names each once.

### Known discrepancies

- The dependency rule says the ticketboard reads tickets through `ticket_model`'s public model
  (`tools/tickets/ticketboard_desktop/README.md`, How it works) — the ticketboard loads tickets without
  `Corpus::load`'s vocabulary and id-to-file checks and keeps its own copies of the wave-lock
  types, the scope vocabulary parsing and the estimate validation
  (`tools/tickets/ticketboard_model/src/execution_metrics/estimated/validation.rs`).

## Data

- `.cargo/config.toml`: the `xtask` alias.
- `tools/browser_testing/browser_gate_suites/gate-env.json`: the pinned Chromium, toolchain versions and limits
  `gate doctor` checks.
- `target/.repository-verification.lock` in the primary checkout: the shared verification lock
  (`GATE_LOCK_RELPATH` in `verification_core`).
- The CI workflows in `.github/workflows/` (`ci.yml`, `contracts.yml`, `editor-gates.yml`,
  `mod-gates.yml`, `schema.yml`), which call `cargo xtask` tasks.

## Design

The tooling is layered so the cheapest crates carry the rules everything else leans on: the two
foundational libraries build in seconds and know nothing of the repository's products, the router
depends on them and on one heavy crate, and only that heavy crate links the map engine. A check
that cannot run says so, so a green run is proof. Paths, limits and prose rules are held by tests
rather than by review.

## Open work

- [T-1137 — Gate ticket_engine, verification_core, ticketboard and fleet agent tests and clippy](/.ai/tickets/T-1137.toml)
  (idea, no plan): CI, `ci-local` and the wave gate run `cargo test` and clippy for the two
  foundational crates, the ticketboard and the game server host agent.
- [T-1142 — Decide whether the ticketboard imports the ticket_engine logic it copies](/.ai/tickets/T-1142.toml)
  (idea, no plan): the ticketboard stops copying ticket crate logic, or the copy is recorded as intended.
- [T-1133 — Consolidate developer_tools duplicate helpers, repo-root lookup and fixture paths](/.ai/tickets/T-1133.toml)
  (idea, no plan): one root lookup in `developer_tools`, fixture paths through the layout module,
  and no `cargo run -p xtask` child from inside the crate.
- [T-1130 — Tidy xtask verifications: redundant check, scattered paths, wave gate](/.ai/tickets/T-1130.toml)
  (idea, no plan): verification paths move into the layout module.
- [T-1115 — Consolidate xtask host-bridge copies and move slice worktree tests](/.ai/tickets/T-1115.toml)
  (idea, no plan): one owner for container detection and host-bridge wrapping.
- [T-1114 — Rename xtask files named after one helper they hold](/.ai/tickets/T-1114.toml) and
  [T-1132 — Rename developer_tools files named after one helper they hold](/.ai/tickets/T-1132.toml)
  (idea, no plan): file names that say what the file holds.
- [T-1118 — Rewrite xtask build and ci comments narrating the retired Makefile](/.ai/tickets/T-1118.toml),
  [T-1119 — Rewrite xtask help, error and doc texts contradicting the code](/.ai/tickets/T-1119.toml),
  [T-1134 — Rewrite developer_tools help texts and comments contradicting the code](/.ai/tickets/T-1134.toml)
  and [T-1139 — Rewrite stale ticket_engine and verification_core comments](/.ai/tickets/T-1139.toml)
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
- `xtask` depends on pure-CPU tool crates only and starts the async and raster work as
  `developer_tools` child processes: the command surface does not rebuild when a server or an
  image codec changes.
- Three outcomes, never two: a missing prerequisite must not pass, so "did not run" has its own
  exit code and outranks a failure.
- Rules live in tests, not in review: dependency direction, path ownership, limits and prose are
  asserted on every `cargo test -p xtask`.
