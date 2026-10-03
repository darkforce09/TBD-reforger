# Repository layout

The `repository_layout` crate: the one walk that finds a checkout root, and the repository
locations more than one tool names, spelled once each. A tool joins these relative paths onto the
root the walk found; a location only one tool names stays in that tool's own layout module
(`tools/developer_tools/src/repository_layout.rs`, `tools/xtask/src/core/repository_layout.rs`,
`tools/ticket_engine/src/repository.rs`).

## Contents

```text
tools/foundation/repository_layout/
├── Cargo.toml  the `repository_layout` library package: `thiserror` only, layout tier 0
└── src/        the root walk, its error, and the ticket registry, artifact, documentation, reference-lane and build-output locations
```

## How it works

```text
find_repository_root()            current directory ──┐
find_repository_root_from(start)  any folder ─────────┴─► walk up to the nearest folder holding .ai/tickets/ROOT
                                                            ├─ Ok(root)   ──► root.join(TICKETS_DIR | ARTIFACTS_DIR | …)
                                                            └─ Err(RootMarkerNotFound { start })
```

The walk stops at the nearest marker, so a slice worktree nested under another checkout resolves
to itself. A command answers from its working directory; a tool that must read the checkout it
was compiled from walks from its own `CARGO_MANIFEST_DIR`. `src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p repository_layout   # the walk over scratch checkouts, and the shared locations in this checkout
```

## Configuration

No feature and no environment variable; the walk reads the working directory and the
filesystem only.

## Public surface

- At the crate root: `find_repository_root`, `find_repository_root_from`, `is_repository_root`,
  `ROOT_MARKER`, `Error` and `Result`; the ticket registry paths (`TICKETS_DIR`, `SCHEMA`,
  `SCOPE_VOCAB`, `CORPUS_PINS`, `WAVE_LOCK`, `QUEUE_JSON`, `METRICS_DIR`, `METRICS_SCHEMA`,
  `ESTIMATES_DIR`, `ESTIMATES_SCHEMA`); the artifact tree (`ARTIFACTS_DIR`, `WORKTREES_DIR`,
  `LAST_VERIFIED_MARKER`, `VERDICTS_DIR`); the reference lanes (`REFERENCES_DIR`,
  `CRF_FRAMEWORK_REFERENCE`, `VANILLA_REFERENCE`); `BUILD_OUTPUT_FOLDER`.
- `documentation`: `DOCUMENTATION_ROOT`, `ROADMAP` and `GAP_ANALYSIS`.
- `prelude`: the walk, the probe and `ROOT_MARKER`.

## Boundaries

- Depends on: `thiserror` only.
- Used by: `ticket_engine`, `developer_tools`, `xtask` and `ticketboard`, for every checkout-root
  lookup and every location listed above.
- Rules: tier 0 of `tools/foundation` with no workspace dependency (`cargo xtask verify
  crate-tiers`; `foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`); no other tool defines a root walk.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tooling crates
  fit together.
