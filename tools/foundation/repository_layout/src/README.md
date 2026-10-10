# Repository layout source

The repository locations more than one tool names. Every location is a relative path with `/`
separators that a caller joins onto the checkout root `repository_root` returns; the contract,
map-asset and npm package modules also offer functions that do that join for a given root.

## Contents

```text
tools/foundation/repository_layout/src/
├── browser_gate_environment.rs  `BROWSER_GATE_ENVIRONMENT`: the committed pin file of the headless browser gates
├── build_output.rs         `BUILD_OUTPUT_FOLDER`, the purpose subfolders inside it, `build_output_subfolder` and the retired root-level names
├── contracts.rs            `CONTRACTS_DIR` and the definition, rule, catalog and fixture locations inside it, joined onto a given root
├── deployment.rs           `DEPLOY_DIR` and the deploy and API settings files, their examples, the compose file, the Caddy site and the systemd units inside it
├── documentation.rs        `DOCUMENTATION_ROOT`
├── documentation_locations.rs  the runbooks and the areas, roots and exemptions of the documentation gates
├── enfusion_mod_folders.rs  the folder name and checkout folder of each Enfusion mod addon (framework, export, MCP bridge) under the mod folder
├── enfusion_mcp_node_package.rs  the pinned `enfusion-mcp` npm package folder and the server module installed in it
├── map_assets.rs           the served terrain and glyph trees and the per-island export scratch, joined onto a given root
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── prelude.rs              the top-level trees the other locations lie under and the checkout-root finder's names, for glob import
├── tool_inputs.rs          the dedicated-server profiles, the recorded MCP transcripts and the staging load data the commands load
├── upstream_references.rs  `REFERENCES_DIR`, the Coalition Reforger Framework and vanilla lanes, and the PlayableSelector lane
├── vanilla_reference_lanes.rs  the extracted scripts, Script API pages, source pages and reconstructed sources inside the vanilla lane
├── workspace_folders.rs    the Enfusion mod, library crate and tool folders, the API server crate, and the API database crate's migration and seed folders
├── workstation_folders.rs  `WORKSTATION_DIR` (machine-local state, gitignored), `WORKTREES_DIR` and the wave driver's verified-commit marker and verdict folder
└── tests/                  unit tests for the shared locations
```

## How it works

- The location modules hold constants; `contracts.rs`, `map_assets.rs` and
  `enfusion_mcp_node_package.rs` add one function per location that joins it onto the root the
  caller passes and never touches the filesystem; `build_output.rs` adds
  `build_output_subfolder` and `is_retired_root_level_build_folder`, pure as well. Each tree
  constant has no trailing slash, and each location inside a tree starts with the tree followed by
  `/`.
- `build_output.rs` names every purpose subfolder a tool builds into under `target/`, each its own
  `CARGO_TARGET_DIR` with its own cargo lock; no name collides with an entry cargo writes inside a
  target directory (profile folders, target triples, its bookkeeping files).

## Boundaries

- Depends on: `repository_root`, whose finder names `prelude.rs` re-exports; the tests find this
  checkout with it.
- Used by: `xtask` (the checkout root, through the prelude) and the check and command crates.
- Rules: `tests/shared_locations_tests.rs` holds the tree containment of every location (the run
  records under `.workstation/`, the reference lanes under `mod/References/`) and the presence of
  every committed location in this checkout.
