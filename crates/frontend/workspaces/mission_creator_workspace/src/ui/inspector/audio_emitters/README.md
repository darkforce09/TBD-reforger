# Audio panel view

The view of the audio panel: the positional sound emitters and the music cues a
[mission](/documentation/glossary/g_to_m.md#mission) authors in its `audio` block, and the gesture that
places an emitter by clicking the map. The model and the document write live in the parent module,
`crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/audio_emitters.rs`.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/audio_emitters/
└── view.rs  `audio_emitters_panel` and `arm_place_on_map`: the emitter and music cue lists
```

## How it works

In the browser build the panel reads `meta.environment.audio` once per build and lists each
emitter (X, Z, Y, "Radius m", "Sound", "Loop", "Trigger id") and each music cue ("Event",
"Track"), with "Add emitter" and "Add cue". An accepted edit writes the whole rebuilt block as one
environment update, one undo step; a refused edit shows its reason under the lists and leaves the
document alone. "Place on map" arms the marker placement with the `point_of_interest` icon, and
"Use last marker" copies the position of the last marker on the map onto the emitter. The native
build renders nothing.

## Boundaries

- Depends on: the parent module (the row model, `apply_marker_xz`, `xz_from_last_marker`,
  `PLACE_MARKER_ICON` and the write through the bridge's `editor_context::update_environment`);
  `armed_placement::begin_place_marker` in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/`;
  `mission_editing_commands::hosted_commands::marker_rows`.
- Used by: the parent module, which re-exports `audio_emitters_panel`; the Mission Settings dialog
  (`crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/settings_modal/mission_dialog.rs`), which mounts it
  after the weather timeline.
- Rules: placement reuses the marker gesture instead of a gesture of its own
  (`place_on_map_reuses_the_marker_gesture` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/tests/audio_emitters/emitter_authoring.rs`).
