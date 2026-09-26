# Undo drive

Undo, redo and the post-change tail for the [mission](/documentation_v2/glossary/g_to_m.md#mission)
document the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) edits. There is no
second stack: the drive steps the document's own undo manager, which tracks only the local origin,
so operator edits are undoable and a boot seed, a restore or a hydrate is not.

## Contents

```text
apps/website/map-engine/src/editing/history/
├── drive.rs  `undo`, `redo` and `after_local_edit`: one step of the document's stack, then the tail
├── host.rs   `HistoryHost` and `install_host`: the one post-change hook the host supplies
└── mod.rs    the module tree; re-exports the drive and the host table
```

## How it works

`undo` and `redo` borrow the hosted document mutably through `crate::editing::host::with_doc_mut`,
step its stack, drop the borrow, and only then run the host's `after_document_change` hook, because
that hook opens read borrows of the same document. A step that changed nothing runs no hook, so a
button fired against an empty stack costs nothing. Every hosted command calls `after_local_edit`
after its own commit, so an edit, an undo and a redo end in the same tail. `after_document_change`
is one hook, not several, because the order of what follows an edit (prune the selection, rebind
lanes, bump the version, mark unsaved, schedule a save, refresh readouts) is the host's to decide.
Without an installed host the drive still undoes and redoes, and tells nobody.

## Boundaries

- Depends on: `crate::data::store::MissionDocCore` (`undo`, `redo`) and `crate::editing::host`.
- Used by:
  - every module of `crate::editing::hosted_commands`, through `after_local_edit`;
  - the Mission Creator's document host
    (`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`), which installs
    the hook and routes the toolbar, keyboard and test-bridge undo and redo here.
- Rules: the mutable borrow ends before the tail runs, and a no-op step runs no tail; this module
  is the only undo path, so toolbar buttons, shortcuts and bridges all reach the same stack.

## Related documentation

- [Mission document store](/apps/website/map-engine/src/data/store/README.md) — the undo manager,
  its local origin and the grouping clock.
