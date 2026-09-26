# Blueprint window prefab fixtures

Synthetic [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) window prefabs: a window frame that
maps a glass socket prefix and places two panes, and the pane with its destruction component.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Windows/
├── Glass.et        a pane: the mesh `Assets/Windows/Glass_01.xob` and a destruction phase model
├── Glass_Base.et   the pane base: placeholder mesh and `SCR_DestructionMultiPhaseComponent` with a hit zone
├── Window.et       a window frame that maps `socket_glass` to `Glass.et` and places two panes
└── Window_Base.et  the frame base: a `Building` with placeholder mesh, static rigid body, hierarchy
```

## How it works

`Prefabs/Houses/House_Base.et` maps the bone prefix `socket_win` to `Window.et` and places two
windows on `socket_win_01` and `socket_win_02`. `Window.et` inherits `Window_Base.et`, sets the
mesh `Assets/Windows/Win.xob`, maps `socket_glass` to `Glass.et`, and places a `$grp` of two panes
on `socket_glass_001` and `socket_glass_002`. `Glass.et` inherits `Glass_Base.et`, whose
destruction component holds a quoted-name `"Additional hit zones"` block with a typed
`SCR_WindowHitZone Default` entry.

The resolver test asserts the window's mesh, its one socket mapping and its two children (the
second on `socket_glass_002`), the pane's mesh and absent hierarchy pivot, and that the lowercase
path `prefabs/windows/glass.et` returns the same memoized prefab. The walker test places each frame
as a `WindowFrame` instance with two `Glass` instances under it, and a pane's synthetic model counts
as all glass with no cover.

## Format

- Encoding: ASCII Enfusion prefab text with invented GUIDs and resource paths; nothing here is game
  content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads.
- Adding a file: keep IDs and GUIDs unique and add its assertions to the resolver test.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers: `resolver_walks_inheritance_sockets_and_children` in
  `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs`, and
  `walker_places_door_set_window_and_furniture_from_fixtures` in
  `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`.

## Boundaries

- Depends on: the `.et` grammar `parse_et` reads; `Prefabs/Houses/House_Base.et`, which places the
  windows.
- Used by: the two tests above.
- Rules: the pivot names match the synthetic window model sockets the walker test builds; the mesh
  paths match the model paths it writes.
