# Repository layout source

The repository locations more than one tool names. Every location is a relative path with `/`
separators that a caller joins onto the checkout root `repository_root` returns; the contract,
map-asset and npm package modules also offer functions that do that join for a given root.

## Contents

```text
tools/foundation/repository_layout/src/
├── agent_artifacts.rs      `ARTIFACTS_DIR` and the worktree base, verified-commit marker and verdict folder inside it
├── browser_gate_environment.rs  `BROWSER_GATE_ENVIRONMENT`: the committed pin file of the headless browser gates
├── build_output.rs         `BUILD_OUTPUT_FOLDER`, the purpose subfolders inside it, `build_output_subfolder` and the retired root-level names
├── contracts.rs            `CONTRACTS_DIR` and the definition, rule, catalog and fixture locations inside it, joined onto a given root
├── deployment.rs           `DEPLOY_DIR` and the settings file, its example, the compose file, the Caddy site and the systemd units inside it
├── documentation.rs        `DOCUMENTATION_ROOT`, `ROADMAP` and `GAP_ANALYSIS`
├── documentation_locations.rs  the runbooks, the API readiness register and the areas, roots and exemptions of the documentation gates
├── enfusion_mcp_node_package.rs  the pinned `enfusion-mcp` npm package folder and the server module installed in it
├── map_assets.rs           the served terrain and glyph trees and the per-island export scratch, joined onto a given root
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── prelude.rs              the top-level trees the other locations lie under and the checkout-root finder's names, for glob import
├── ticket_registry.rs      `TICKETS_DIR` and the schemas, vocabulary, wave lock, queue, receipts and estimates beside the tickets
├── tool_inputs.rs          the dedicated-server profiles, the recorded MCP transcripts and the staging load data the commands load
├── upstream_references.rs  `REFERENCES_DIR`, the Coalition Reforger Framework and vanilla lanes, and the PlayableSelector lane
├── vanilla_reference_lanes.rs  the extracted scripts, Script API pages, source pages and reconstructed sources inside the vanilla lane
├── workspace_folders.rs    the applications, library crate and tool folders, and the API database crate's migration and seed folders
└── tests/                  unit tests for the shared locations
```

## How it works

- The location modules hold constants; `contracts.rs`, `map_assets.rs` and
  `enfusion_mcp_node_package.rs` add one function per location that joins it onto the root the
  caller passes and never touches the filesystem; `build_output.rs` adds
  `build_output_subfolder` and `is_retired_root_level_build_folder`, pure as well. Each tree
  constant has no trailing slash, and each location inside a tree starts with the tree followed by
  `/`; the API readiness evidence prefix alone ends in `/`, because the fingerprint matches it with
  `starts_with`.
- `build_output.rs` names every purpose subfolder a tool builds into under `target/`, each its own
  `CARGO_TARGET_DIR` with its own cargo lock; `tests/build_output_tests.rs` proves no name collides
  with an entry cargo writes inside a target directory (profile folders, target triples, its
  bookkeeping files).

## Boundaries

- Depends on: `repository_root`, whose finder names `prelude.rs` re-exports; the tests find this
  checkout with it.
- Used by: the ticket crates in `tools/tickets/`, `xtask` (the checkout root, through the prelude),
  the check and command crates and `ticketboard`.
- Rules: each location module's test file in `tests/` pins its committed locations against this
  checkout and its derived locations by shape; `tests/shared_locations_tests.rs` holds the tree
  containment of every location (the root marker among the ticket registry's files) and the
  presence of every committed location in this checkout;
  `tests/command_locations_tests.rs` holds the deployment, tool input and documentation locations
  (every documentation item classified as a location the checkout holds or an exemption with its
  reason).
