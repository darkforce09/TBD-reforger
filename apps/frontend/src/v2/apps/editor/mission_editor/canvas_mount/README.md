# Canvas mount parts

The parts the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s canvas mount runs
once the canvas element loads: the document setup, the two boot tasks and their handshake, the
review workspace's restore, the item [registry](/documentation/glossary/n_to_z.md#registry) loading,
the chrome and dock reflow, and the page-level input listeners. Canvas sizing, engine creation,
the frame pump and resize tracking come from the shared map seam,
`apps/frontend/src/v2/core/map_view/`. The parent module,
`apps/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`, declares these modules
and runs them from `install_canvas_mount`.

## Contents

```text
apps/frontend/src/v2/apps/editor/mission_editor/canvas_mount/
├── boot_tasks.rs        document restore and render-engine start in parallel, and their handshake
├── dock_reflow.rs       chrome and dock latches, engine resize, the pane-centre camera hold
├── document_setup.rs    seeds the document, publishes its smoke bridge, registers document commands
├── input_listeners.rs   the gesture context; pointercancel and pointerleave; seam resize tracking
├── registry_effects.rs  the registry and compatibility feed, from the tab cache or the API
├── review_restore.rs    a review workspace's restore: exactly the reviewed version, nothing armed
└── signals.rs           `PageMountSignals`, the page signals the mount takes over
```

## How it works

`document_setup::initialize` builds the seeded document, writes the template comments of a new
[mission](/documentation/glossary/g_to_m.md#mission), publishes `window.__missionDoc` and hands the
document to the session's document commands. `boot_tasks::start` then runs two tasks that meet in a
handshake:

```text
document task
├── review mode holds this mission ──> review_restore: adopt the reviewed payload as
│                                      initialisation; no draft read, nothing armed
└── otherwise ──> draft from IndexedDB (shell::persist) ──> server hydrate (shell::hydrate)
                  ──> arm the debounced draft writer, the warm-session marker,
                      the flush on hide and the cross-tab sync
engine task: map_view::engine_mount::create_engine (12.8 km camera square, centre, zoom -2)
             ──> bind the slot and vehicle lanes ──> start_raf
             ──> world_assets::bootstrap (full scope) for the document's terrain (everon when unset)
handshake: restore settled and world ready ──> hand_over ──> BootPhase::Ready
engine failure ──> BootPhase::Failed on the stage it reached; map_disabled holds the reason
```

Whichever task finishes second rebinds the engine from the settled document, so the map never draws
the seed once the restore has landed. The registry effect reads the item registry and the
compatibility feed from the bridge's `registry_session` cache when an earlier editor mount in the
tab filled it, and otherwise fetches them; a retry bumps `registry_fetch_gen` and fetches again, and
a failure marks both catalogs failed and the feed unavailable. `input_listeners::attach` builds the
`EditorGestureContext` and attaches the canvas gestures and the chord listener from
`apps/frontend/src/v2/apps/editor/input/`; its own `pointercancel` drops an armed place, a
pending connection and an in-flight gesture, and `map_view::resize::observe_container_resize`
re-sizes the canvas backing store and the engine whenever the container or the window resizes.
`dock_reflow::install` mirrors the hide-chrome and dock-collapse signals into the shell layout and,
when a dock reflow moves the pane centre, shifts the camera so the world under it stays put.

## Boundaries

- Depends on: the shared map seam `apps/frontend/src/v2/core/map_view/` (handles, canvas
  sizing, engine creation, resize tracking); the parent's imports: the bridge's document host,
  editor context, boot machine,
  viewport and world-assets host in `apps/frontend/src/v2/apps/editor/bridge/`; the
  session's draft writer, hydrate, review mode, warm-session marker and document commands in
  `apps/frontend/src/v2/apps/editor/shell/`; the registry fetches in
  `apps/frontend/src/v2/apps/editor/mission_editor/registry_loading.rs`; the asset catalog
  and compatibility rules of `apps/frontend/src/v2/apps/editor/arsenal/`; the validation
  panel's compile findings; the input layer;
  `map_engine` (`frame::engine::RenderEngine`, `editing::persist::server_adoption`,
  `editing::hosted_commands`, `data::store`, `overlay::symbology`).
- Used by: the parent's `install_canvas_mount`, which the editor page
  `apps/frontend/src/v2/apps/editor/mission_editor.rs` calls; the source pins that read
  these files in `apps/frontend/src/v2/apps/editor/tests/`
  (`apps/frontend/src/v2/apps/editor/tests/mission_editor/source.rs` and the boot-progress,
  placement and hover-cursor pins) and in
  `apps/frontend/src/v2/apps/editor/shell/tests/review_mode/read_only_review.rs`.
- Rules: the review restore reads nothing from the draft store and arms no draft writer, flush,
  marker or writer election (`the_review_boot_restores_the_reviewed_version_and_arms_nothing` in
  `apps/frontend/src/v2/apps/editor/shell/tests/review_mode/read_only_review.rs`); a
  restored or reviewed document is applied under the init origin, so it opens with no undo step.

## Related documentation

- [Mission Creator feature inventory: map viewport and camera](/documentation/apps/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md) — the map view, terrain and camera at boot.
- [Mission Creator feature inventory: editor route and boot loading](/documentation/apps/frontend/apps/editor/feature_inventory/editor_route_loading.md) — the boot overlay.
