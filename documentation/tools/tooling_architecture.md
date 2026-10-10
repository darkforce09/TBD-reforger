**Status:** live

# Tooling architecture

How the developer tooling in `tools/` is put together: the crates and the npm package, the
direction their dependencies run, the invariants that keep them apart, and the verification
surface they build. Developers and AI agents read it before adding a command, a check or a
module to the tooling; each crate's README then says what its own folders hold.

## Where it lives

- Code: [`tools/`](/tools/README.md) with
  [`xtask/`](/tools/xtask/README.md), the [foundation crates](/tools/foundation/README.md)
  ([`verification_core/`](/tools/foundation/verification_core/README.md),
  [`process_runner/`](/tools/foundation/process_runner/README.md),
  [`repository_laws/`](/tools/foundation/repository_laws/README.md),
  [`ticket_manager_client/`](/tools/foundation/ticket_manager_client/README.md)),
  [`developer_tools/`](/tools/developer_tools/README.md) and
  [`enfusion_mcp_node_package/`](/tools/enfusion_mcp_node_package/README.md).
- Entry: `cargo xtask`, the alias `run --package xtask --` in `.cargo/config.toml`; the eight
  `developer_tools` executables (`enf`, `gate`, `mcpd`, `world`, `map`, `capture`,
  `acknowledgement-dropping-relay`, `staging-load`).
- Related features: the [developer tools documentation](/documentation/tools/developer_tools/README.md);
  the wave and slice runner lives in the central ticket manager (`ttm`), outside the repository.

## Behaviour

### The shape

| Unit | Kind | Owns |
|---|---|---|
| `xtask` | binary | the command router: database, deploys, mod servers, map helpers, the platform factory, and every repository verification and CI task |
| `verification_core` | library, `tools/foundation` tier 0 | the fail-closed primitives: verdicts, findings, pattern scans, the report and the shared verification lock |
| `process_runner` | library, `tools/foundation` tier 1 | child processes in their own process group with deadlines and honest statuses, the container-to-host bridge, the ssh transport |
| `repository_laws` | library, `tools/foundation` tier 1 | the structural engineering laws as pure checks: file length and the workspace laws (crate tiers, crate anatomy, test-file reachability, frontend layering, Tailwind sources) |
| `repository_layout` | library, `tools/foundation` tier 0 | the repository locations more than one tool names |
| `deploy_settings` | library, `tools/foundation` tier 2 | the `deploy/deploy.env` reader: the precedence of the file over the process environment, the deploy host, its remote folders and the ssh transport choice |
| `tool_test_support` | library, `tools/foundation` tier 1, dev-dependency only | the environment and working-directory locks and the test checkout root the tool tests share |
| `ticket_manager_client` | library, `tools/foundation` tier 2 | the typed client of the central ticket manager: it runs `ttm --json --project <project>` and parses the [ticket](/documentation/glossary/n_to_z.md#ticket), receipt and [wave](/documentation/glossary/n_to_z.md#wave) documents it prints; `platform_execution` and `mod_operations` reach tickets and waves through it alone |
| `enfusion_pak` | library, `tools/enfusion` tier 0 | the [Enfusion](/documentation/glossary/a_to_f.md#enfusion) `.pak` archive reader: one parser and one decompressor under the blueprint and world policies, the loose and layered sources |
| `blueprint_compiler` | library, `tools/map_assets` tier 6 | the building-blueprint compiler: voxel dumps and game models to blueprints, occlusion sidecars, the prefab occluder library and the blueprint archive, behind the `cargo xtask map` blueprint, BVH and model commands |
| `map_asset_verification` | library, `tools/map_assets` tier 7 | the map asset gates: the terrain manifest, the prefab BLAS library, the labels, the elevation anchors and the map-object goldens behind `cargo xtask schema` and `verify blas-manifest`, and the world line-of-sight probe behind `cargo xtask map world-los` |
| `enfusion_script_index` | library, `tools/enfusion` tier 2 | the script oracle behind `enf`: symbol indexes, lookups, the citation and capability gates, vanilla extraction, and the vanilla page mirrors behind `cargo xtask fetch` |
| `chrome_devtools_protocol` | library, `tools/browser_testing` tier 1 | the Chrome DevTools Protocol client: Chromium discovery and headless launch in its own process group, pages over one WebSocket each, the gate font cache |
| `browser_gate_suites` | library, `tools/browser_testing` tier 5 | the headless browser gates of the single-page app: the static server, the Mission Creator smokes, the data viewer gate, the ballistics agreement and offline mortar gates, the capture rig, the doctor, and the `gate` and `capture` command lines |
| `map_raster_pipeline` | library, `tools/map_assets` tier 7 | the map raster pipeline behind the `map` binary: the orthophoto stitch, the satellite container and tile pyramids, the cartographic render, the label sets and archives, the water archives and the world-glyph atlas; never in xtask's closure |
| `developer_tools` | eight binaries, no library | one-line `main`s: `enf` and `mcpd` over the Enfusion crates, `gate` and `capture` over `browser_gate_suites`, `world` and `map` over `world_export_pipeline` and `map_raster_pipeline`, and `acknowledgement-dropping-relay` and `staging-load` over the staging crates |
| `enfusion_mcp_node_package` | npm data, not a crate | the pinned `enfusion-mcp` server that `mcpd` and `cargo xtask mcp` start |

One word names one thing. A gate is a repository verification that reaches a verdict; the
headless browser harness is `browser_gate_suites` over `chrome_devtools_protocol`; the assertion primitives are `verification_core`;
the ticket rules are the central ticket manager's (`ttm --project reforger check`).

### Dependency direction

```text
verification_core ◀── process_runner, repository_laws        (tools/foundation, tiers 0 and 1)
        ▲                      ▲
        │                      │
        └──────── xtask ───────┴──▶ platform_execution, mod_operations (tools/commands)
                    │                          │
                    │                          ▼
                    │               ticket_manager_client (tools/foundation) ──runs──▶ ttm
                    ▼ runs as child processes
             developer_tools (binaries only)
                    │
                    ├── mcpd ──▶ enfusion_mcp_broker (tools/enfusion) ──starts──▶ enfusion_mcp_node_package (after npm ci)
                    ├── world, map ──▶ world_export_pipeline, map_raster_pipeline (tools/map_assets)
                    └── gate, capture ──▶ browser_gate_suites ──▶ chrome_devtools_protocol (tools/browser_testing)
```

1. A `tools/foundation` crate depends only on lower `tools/foundation` crates (`process_runner`
   and `repository_laws` on `verification_core`, which depends on none) and on the checkout-root
   finder `repository_root` and the id macros `newtype_ids`, so each is read, tested and reasoned
   about without the tools above it.
2. `xtask` and `developer_tools` are binary-only packages whose workspace dependencies are tool
   crates alone (the checkout-root finder comes through `repository_layout`'s prelude); neither
   depends on the other, and no member depends on either.
3. No tokio, axum, reqwest, resvg or image enters xtask's dependency closure (rule 6 of `cargo xtask verify crate-tiers`):
   the async servers and the raster crates run behind the `developer_tools` binaries, so neither
   a server nor an image codec rebuilds the command surface.
4. Ticket logic has one owner, the central ticket manager: the workspace reads and records
   tickets, run receipts and the wave plan only through `ticket_manager_client`, which runs
   `ttm`, and no crate reads or writes ticket files.

### One owner per path

Only the `repository_layout` crate and the tools' own layout modules spell a repository path
literal. A tool's own layout module declares itself on its first line
(`//! The repository locations only …`), so the rule finds it without a list:

| Module | Owns |
|---|---|
| `tools/foundation/repository_layout` | the locations more than one tool names: the machine-local `.workstation/` folders and `.worktrees/`, the reference lanes and their folders, the contract and map-asset trees, the enfusion-mcp npm package, the browser gate pins, the documentation root, the roadmap and gap analysis, the deploy tree, the server profiles, the MCP fixtures, the runbooks and documentation areas the commands name, and the build output folder with its purpose subfolders |
| `tools/map_assets/map_raster_pipeline/src/decision_record_locations.rs` | the decision records of the inland-water, aerial-orthophoto and cartographic lanes |
| `tools/enfusion/enfusion_script_index/src/script_index_layout.rs` | the Enfusion symbol index and the capability verdict table |
| `tools/browser_testing/browser_gate_suites/src/gate_layout.rs` | the map-asset mounts of the gates' server and the editor gate runbook |

Every tool resolves the checkout root through `repository_root::find_repository_root`
(`crates/foundation/repository_root`, the one root walk of the workspace, which the API's and the
frontend crates' tests use as well; xtask reaches it through `repository_layout::prelude`), which walks up to `.repository_root`, so a command run in a
linked worktree reads that worktree's files; a working directory outside any checkout is an
error, never a guessed folder. `tools/foundation/repository_layout` spells the locations more
than one tool names: the machine-local `.workstation/` folders, the reference lanes, the
documentation root and the build output folder.

### One outcome vocabulary

Every verification returns a `verification_core::Verdict`: held, failed, or did not run. A check
whose input is missing, whose tool is absent, whose child was killed or whose corpus will not load
reports did-not-run, never a pass; an empty input is never a clean tree. `Verdict` has no
`From<bool>` and no `is_ok()`, so no expression folds the third outcome into the first. A `Report`
turns the verdicts into the exit code, 0 held, 1 failed, 2 did not run, with 2 outranking 1. This
is what makes a green run evidence; the [verification core README](/tools/foundation/verification_core/README.md#how-it-works)
gives the table and the lock rules.

Gates that must not overlap (the MCP broker start among them) serialise on one
`flock` at `target/.repository-verification.lock` in the primary checkout, found through
`git rev-parse --git-common-dir`, so linked worktrees share it. Running out of time is a refusal,
never an unserialised run.

### The verification surface

`cargo xtask verify <name>` runs one repository check, `cargo xtask schema <name>` one contract or
map-asset gate, and `cargo xtask ci <task>` one row of the CI task table; `cargo xtask ci
ci-local` replays the composite the CI jobs run. The three callers reach the same functions in
the check crates under `tools/checks/` and the command crates under `tools/commands/`, which the
[verify group README](/tools/xtask/src/commands/verify/README.md) maps verb by verb. The
documentation gate `link-check` holds the link structure the documentation standard sets.

### Structural limits and prose

- Production files stay under 500 lines, test files under 1,000, `tools/xtask/src/main.rs`
  under 150, the executables' `src/bin/` files under 250 and the editor smoke scenarios under 450
  (`cargo xtask verify file-length` reports the general limits). No exemption mechanism exists.
- Unit tests live in sibling files declared with `#[path = "tests/…"]`, never in an inline test
  module.
- Every tracked file under `tools/` describes the present: no ticket ids, no history words, no
  retired crate or file spellings, and no shell, Python or Node script names. Commit history owns
  history.

### Data beside the crates

`enfusion_mcp_node_package/` sits outside every crate root on purpose: `verify file-length` walks
whole crate folders and the structural walk refuses symlinks under `src/`, so a vendored `.rs`
inside an installed `node_modules/` would be subject to both. The other data folders
(`xtask/dedicated_server_profiles/`, `xtask/fixtures/`, `xtask/staging/`,
`map_assets/blueprint_compiler/test_fixtures/`) sit beside the code that reads them, and a layout
module names each once.

## Data

- `.cargo/config.toml`: the `xtask` alias.
- `tools/browser_testing/browser_gate_suites/gate-env.json`: the pinned Chromium, toolchain versions and limits
  `gate doctor` checks.
- `target/.repository-verification.lock` in the primary checkout: the shared verification lock
  (`GATE_LOCK_RELPATH` in `verification_core`).
- The CI workflows in `.github/workflows/` (`ci.yml`, `editor-gates.yml`, `mod-gates.yml`), which
  call `cargo xtask` tasks.

## Design

The tooling is layered so the cheapest crates carry the rules everything else leans on: the two
foundational libraries build in seconds and know nothing of the repository's products, the router
depends on them and on one heavy crate, and only that heavy crate links the map engine. A check
that cannot run says so, so a green run is proof.

## Open work

- Gate the foundational crates' and the fleet agent's tests and clippy (ticket
  `gate-ticket-engine-verification` in `ttm`, idea): CI, `ci-local` and the wave gate run `cargo
  test` and clippy for the foundational crates and the game server host agent.
- Consolidate developer_tools duplicate helpers, repo-root lookup and fixture paths (ticket
  `consolidate-developer-tools-duplicate` in `ttm`, idea): one root lookup in `developer_tools`,
  fixture paths through the layout module, and no `cargo run -p xtask` child from inside the crate.
- Tidy xtask verifications: redundant check, scattered paths, wave gate (ticket
  `tidy-xtask-verifications-redundant` in `ttm`, idea): verification paths move into the layout
  module.
- Consolidate xtask host-bridge copies and move slice worktree tests (ticket
  `consolidate-xtask-host-bridge` in `ttm`, idea): one owner for container detection and host-bridge
  wrapping.
- Rename xtask files named after one helper they hold (ticket `rename-xtask-files-named` in `ttm`,
  idea) and Rename developer_tools files named after one helper they hold (ticket
  `rename-map-tool-files` in `ttm`, idea): file names that say what the file holds.
- Rewrite xtask build and ci comments narrating the retired Makefile (ticket
  `rewrite-xtask-build-ci` in `ttm`, idea), Rewrite xtask help, error and doc texts contradicting
  the code (ticket `rewrite-xtask-help-error` in `ttm`, idea), Rewrite developer_tools help texts
  and comments contradicting the code (ticket `rewrite-developer-tools-help` in `ttm`, idea) and
  Rewrite stale ticket_engine and verification_core comments (ticket `rewrite-stale-ticket-engine`
  in `ttm`, idea): help texts and comments that match the code.
- Comment hygiene: design citations, ticket ids and history words (ticket
  `comment-hygiene-design-citations` in `ttm`, idea): the prose rules also catch design-document
  citations and reach ticket ids in Rust and EnfScript comments.
- Remove dead xtask code and move the font table generator (ticket `remove-dead-xtask-code` in
  `ttm`, idea): `gen font-table` leaves the file-length verification.
- Enforce or remove the unread enfusion_mcp_node_package .nvmrc (ticket
  `enforce-remove-unread-enfusion` in `ttm`, idea): the pinned Node version is checked, or the file
  goes.

## Decisions

- The foundational crates take no workspace dependency: they stay fast to build and test, and
  every other crate can use them without a cycle.
- `xtask` depends on pure-CPU tool crates only and starts the async and raster work as
  `developer_tools` child processes: the command surface does not rebuild when a server or an
  image codec changes.
- Three outcomes, never two: a missing prerequisite must not pass, so "did not run" has its own
  exit code and outranks a failure.
