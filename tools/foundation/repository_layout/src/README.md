# Repository layout source

The checkout-root walk and the repository locations more than one tool names. Every location is a
relative path with `/` separators that a caller joins onto the root the walk returns.

## Contents

```text
tools/foundation/repository_layout/src/
├── agent_artifacts.rs      `ARTIFACTS_DIR` and the worktree base, verified-commit marker and verdict folder inside it
├── build_output.rs         `BUILD_OUTPUT_FOLDER`: the gitignored `target` folder at a checkout root
├── documentation.rs        `DOCUMENTATION_ROOT`, `ROADMAP` and `GAP_ANALYSIS`
├── error.rs                `Error` and `Result`: an unreadable working directory or a walk that found no marker
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── prelude.rs              the walk, the probe and `ROOT_MARKER` for glob import
├── repository_root.rs      `ROOT_MARKER`, `find_repository_root`, `find_repository_root_from` and `is_repository_root`
├── ticket_registry.rs      `TICKETS_DIR` and the schemas, vocabulary, wave lock, queue, receipts and estimates beside the tickets
├── upstream_references.rs  `REFERENCES_DIR` and the Coalition Reforger Framework and vanilla lanes
└── tests/                  unit tests for the walk and the shared locations
```

## How it works

- `repository_root.rs` pops one path component at a time from the start folder and stops at the
  first folder where `.ai/tickets/ROOT` is a file; a folder of that name is not a marker. When the
  path has no component left it returns `Error::RootMarkerNotFound` naming the start folder.
- `find_repository_root` starts from the working directory, so a command run in a slice worktree
  reads that worktree even when its binary was built from a sibling checkout.
- The location modules hold constants only. Each tree constant has no trailing slash, and each
  location inside a tree starts with the tree followed by `/`.

## Boundaries

- Depends on: `std::fs` metadata reads and `thiserror`.
- Used by: `ticket_engine`, `developer_tools`, `xtask` and `ticketboard`.
- Rules: `tests/repository_root_tests.rs` holds the nearest-marker rule over scratch checkouts,
  the error at the filesystem root, the tree containment of every location, and the presence of
  every committed location in this checkout.
