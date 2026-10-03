# Repository layout source

The checkout-root walk and the repository locations more than one tool names. Every location is a
relative path with `/` separators that a caller joins onto the root the walk returns; the contract,
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
├── error.rs                `Error` and `Result`: an unreadable working directory or a walk that found no marker
├── map_assets.rs           the served terrain and glyph trees and the per-island export scratch, joined onto a given root
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── prelude.rs              the walk, the probe and `ROOT_MARKER` for glob import
├── repository_root.rs      `ROOT_MARKER`, `find_repository_root`, `find_repository_root_from` and `is_repository_root`
├── ticket_registry.rs      `TICKETS_DIR` and the schemas, vocabulary, wave lock, queue, receipts and estimates beside the tickets
├── tool_inputs.rs          the dedicated-server profiles, the recorded MCP transcripts and the staging load data the commands load
├── upstream_references.rs  `REFERENCES_DIR`, the Coalition Reforger Framework and vanilla lanes, and the PlayableSelector lane
├── vanilla_reference_lanes.rs  the extracted scripts, Script API pages, source pages and reconstructed sources inside the vanilla lane
└── tests/                  unit tests for the walk and the shared locations
```

## How it works

- `repository_root.rs` pops one path component at a time from the start folder and stops at the
  first folder where `.ai/tickets/ROOT` is a file; a folder of that name is not a marker. When the
  path has no component left it returns `Error::RootMarkerNotFound` naming the start folder.
- `find_repository_root` starts from the working directory, so a command run in a slice worktree
  reads that worktree even when its binary was built from a sibling checkout.
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

- Depends on: `std::fs` metadata reads and `thiserror`.
- Used by: the ticket crates in `tools/tickets/`, `developer_tools`, `xtask` and `ticketboard`.
- Rules: each location module's test file in `tests/` pins its committed locations against this
  checkout and its derived locations by shape; `tests/repository_root_tests.rs` holds the
  nearest-marker rule over scratch checkouts, the error at the filesystem root, the tree
  containment of every location, and the presence of every committed location in this checkout;
  `tests/command_locations_tests.rs` holds the deployment, tool input and documentation locations
  (every documentation item classified as a location the checkout holds or an exemption with its
  reason).
