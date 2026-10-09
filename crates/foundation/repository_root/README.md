# Repository root

The `repository_root` crate: the one walk that finds the checkout root, the folder every
repository-relative path is joined onto. The tools, the [API](/documentation/glossary/a_to_f.md#api)'s
tests and the frontend crates' tests all find the root through it.

## Contents

```text
crates/foundation/repository_root/
├── Cargo.toml  the package: `thiserror`, layout tier 0, any target
└── src/        the walk, its error type and the prelude
```

## How it works

```text
find_repository_root()            working directory ──┐
find_repository_root_from(start)  any folder ─────────┴─► walk up to the nearest folder holding .ai/tickets/ROOT
```

A folder is the checkout root when it holds the file `.ai/tickets/ROOT` (`ROOT_MARKER`); the walk
stops at the nearest one, so a slice worktree nested under another checkout resolves to itself,
and a walk that reaches the filesystem root is an `Error` naming the folder it started from and
the marker. A folder named like the marker does not count. `find_repository_root` starts from the
working directory, so a command run in a worktree reads that worktree's files even when the binary
was linked from a sibling checkout sharing the build folder; a test that must find the checkout it
was compiled in starts from its own `env!("CARGO_MANIFEST_DIR")` with `find_repository_root_from`.

Why the marker is `.ai/tickets/ROOT` and not the Cargo workspace root (a `Cargo.lock` beside a
`Cargo.toml` declaring `[workspace]`):

- **Every checkout carries it.** It is a tracked file, so every clone, every CI checkout (the
  workflows check out the whole tree, never a sparse one) and every `git worktree` holds it. The
  release image's build context (`.dockerignore`) leaves `.ai/` out, but that build compiles the
  API binary alone, which links neither this crate nor any root walk: only test code and the tools
  do.
- **Nothing fakes it by accident.** Tests across the tools build throwaway Cargo workspaces (the
  repository-law tests write a `[workspace]` manifest into a temporary checkout), and a Cargo
  marker would take any of them for a root. A throwaway folder is a root here only when a test
  plants the marker on purpose, as the ticket registry tests do.
- **One stat, no parse.** The probe asks whether one file exists; the Cargo marker has to read the
  root manifest and match a `[workspace]` line, which a comment or a reformatted header breaks.
- **One answer.** In this repository both markers name the same folder
  (`the_marker_root_is_the_cargo_workspace_root_above_the_crate` pins that), so the frontend tests
  that used the Cargo marker read the same files through this one.

## Getting started

Run from the repository root:

```bash
cargo test -p repository_root   # the walk from this checkout and from planted temporary folders
```

A crate adds `repository_root = { workspace = true }` to `[dependencies]` (a tool command) or
`[dev-dependencies]` (a test that reads repository files), then joins its relative locations onto
`find_repository_root()?`.

## Configuration

None: no feature, no environment variable. The working directory is the walk's start for
`find_repository_root`.

## Public surface

- `find_repository_root() -> Result<PathBuf>`: the root above the working directory.
- `find_repository_root_from(start: &Path) -> Result<PathBuf>`: the root at or above `start`.
- `is_repository_root(candidate: &Path) -> bool`: whether `candidate` holds the marker file.
- `ROOT_MARKER`: `.ai/tickets/ROOT`, relative to the root.
- `Error` (`CurrentDirectory`, `RootMarkerNotFound { start }`) and `Result`.
- `prelude`: the three functions and `ROOT_MARKER`.

## Boundaries

- Depends on: `thiserror`; at run time, the filesystem from the start folder upward.
- Used by: the tool crates under `tools/` (every command that reads the checkout, and
  `tool_test_support`'s `test_repo_root`), the API's and `api_community_content`'s tests, and
  `frontend_test_support`, whose repository file reads walk up from the calling crate's manifest
  folder. `repository_layout` names the locations the tools join onto the answer.
- Rules: foundation tier 0, so the crate depends on no workspace crate (`cargo xtask verify
  crate-tiers`); the one root walk of the workspace, so no other crate walks up to a marker of its
  own.

## Related documentation

- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tools find the
  checkout and the locations they share.
- [Ticket registry](/.ai/tickets/README.md) — the folder that holds the marker file.
