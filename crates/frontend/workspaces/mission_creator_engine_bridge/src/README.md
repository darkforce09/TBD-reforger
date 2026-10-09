# Mission Creator engine bridge source

The source tree of `mission_creator_engine_bridge`: the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s side of the engine seam and
the input layer over it.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/
├── bridge/     the engine seam: boot, frame timing, document host and undo, host state, overlays, tactical graphics, hover
├── input/      pointer and keyboard events turned into map-engine commands; the browser half of the map tools
├── lib.rs      the crate root: the module tree
└── prelude.rs  the hosted document handle, the undo drive and the gesture context most callers name
```

## How it works

`bridge/` holds the live handles: the hosted mission document and its undo drive, the editor
context the hosted commands read, the selection and the armed placement, the boot machine and the
frame-timing belt, and the overlays laid over the map. `input/` turns the operator's pointer and
keyboard into calls on those handles and on the hosted commands of `mission_editing_commands`; it
hands a right click to the opener the workspace registers at mount. Each folder's README walks
through its files.

## Boundaries

- Depends on: `mission_creator_state`, the foundation crates, the mission, editing, streaming,
  line-of-sight and rendering crates (the rendering and streaming host only on `wasm32`).
- Used by: the crate's callers through `lib.rs`.
- Rules: a module that touches `web_sys` or a live engine handle is `wasm32`-only, and its `pub mod`
  line carries the same gate; an authored change reaches the document only through the hosted
  commands.
