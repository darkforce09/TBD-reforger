**Status:** live

# Architecture

The living description of how the repository is laid out: its top-level folders, its Cargo
workspace members and where code, contracts, assets and documents live. Anyone new to the
repository, and every agent before it moves or adds a folder, starts here.

## Contents

```text
documentation/architecture/
└── workspace_layout.md  the top-level folders, the workspace members, where things live
```

## How it works

[Workspace layout](/documentation/architecture/workspace_layout.md) describes the tree as it is at
the latest commit, never as planned: a change that moves a top-level folder, adds or removes a
workspace member or changes where a kind of file lives updates it in the same commit. The
[directory atlas](/CLAUDE.md#2-monorepo-directory-atlas) in `CLAUDE.md` names the folders one
level deeper, and the [crate boundary rules](/documentation/standards/crate_boundary_rules.md)
state the laws every member is held to.

## Code

- [Root workspace manifest](/Cargo.toml) — the workspace members the layout lists.
- [Crates](/crates/README.md), [tools](/tools/README.md), the [game mod](/apps/README.md),
  [contracts](/contracts/README.md) and [assets](/assets/README.md) — the top-level code and data
  folders.

## Boundaries

- Depends on: the root `Cargo.toml` and the tracked tree, which every statement is checked
  against; the [documentation standards](/documentation/standards/documentation_standards.md).
- Used by: the [documentation entry](/documentation/README.md), `CLAUDE.md`, the Cursor platform
  rule and the archive topics whose layout plans it replaces.
- Rules: describes only what exists; a planned path is written as plain text, never as a
  backticked path.

## Related documentation

- [Architecture blueprint draft](/documentation/archive/restructure_research/00_architecture_blueprint_draft.md)
  — the archived first draft of the crate workspace, which this document succeeds as the living
  description.
- [Where does X go?](/documentation/standards/where_does_x_go.md) — where a new file belongs.
