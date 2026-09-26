# Audio panel view

The view of the audio panel: the positional sound emitters and the music cues a
[mission](/documentation_v2/glossary/g_to_m.md#mission) authors in its `audio` block, and the gesture that
places an emitter by clicking the map. The model and the document write live in the parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/audio_emitters.rs`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/audio_emitters/
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
  `armed_placement::begin_place_marker` in `apps/website/frontend/src/v2/apps/editor/bridge/`;
  `website_map_engine::editing::hosted_commands::marker_rows`.
- Used by: the parent module, which re-exports both functions. No surface mounts the panel: the
  Mission Settings dialog in `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`
  does not render it.
- Rules: placement reuses the marker gesture instead of a gesture of its own
  (`place_on_map_reuses_the_marker_gesture` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/audio_emitters/emitter_authoring.rs`).
