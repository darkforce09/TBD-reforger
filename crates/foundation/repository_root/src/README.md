# Repository root source

The source of `repository_root`: the walk up to the checkout-root marker, the error of a walk that
finds none, and the crate root that exports them.

## Contents

```text
crates/foundation/repository_root/src/
├── error.rs             `Error` and `Result`: the working directory unreadable, or no marker up to the filesystem root
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── prelude.rs           the walk, the probe and `ROOT_MARKER` for glob import
├── root_marker_walk.rs  `ROOT_MARKER`, `find_repository_root`, `find_repository_root_from` and `is_repository_root`
└── tests/               unit tests of the walk
```

## How it works

`root_marker_walk.rs` pops one folder at a time from the start until `is_repository_root` finds
`ROOT_MARKER` as a file; a start with no such ancestor is `Error::RootMarkerNotFound`, whose
message names the start and the marker. `find_repository_root` reads the working directory first
(`Error::CurrentDirectory` when it cannot). `tests/root_marker_walk.rs` walks this checkout from the
working directory, the crate folder and its source folder, and plants markers in temporary folders
for the nested-worktree, throwaway-workspace, folder-named-like-the-marker and no-root cases.

## Boundaries

- Depends on: `thiserror`; the filesystem, read only.
- Used by: every caller through the crate root or `prelude`.
- Rules: no path here is joined onto the answer; a caller's locations live with the caller (the
  tools' shared ones in `repository_layout`).
