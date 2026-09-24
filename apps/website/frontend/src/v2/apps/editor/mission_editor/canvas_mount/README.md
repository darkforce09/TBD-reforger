# Canvas mount parts

The parts the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s canvas mount runs
once the canvas element loads: the document setup, the two boot tasks and their handshake, the
review workspace's restore, the item [registry](/documentation_v2/glossary.md#registry) loading and
the page-level input listeners. The parent module,
`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`, declares these modules
and runs them from `install_canvas_mount`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/
├── boot_tasks.rs        document restore and render-engine start in parallel, and their handshake
├── document_setup.rs    seeds the document, publishes its smoke bridge, registers document commands
├── input_listeners.rs   the gesture context; pointercancel, pointerleave and resize listeners
├── registry_effects.rs  the registry and compatibility feed, from the tab cache or the API
├── review_restore.rs    a review workspace's restore: exactly the reviewed version, nothing armed
└── signals.rs           `PageMountSignals`, the page signals the mount takes over
```

## How it works

`document_setup::initialize` builds the seeded document, writes the template comments of a new
[mission](/documentation_v2/glossary.md#mission), publishes `window.__missionDoc` and hands the
document to the session's document commands. `boot_tasks::start` then runs two tasks that meet in a
handshake:

```text
document task
├── review mode holds this mission ──> review_restore: adopt the reviewed payload as
│                                      initialisation; no draft read, nothing armed
└── otherwise ──> draft from IndexedDB (shell::persist) ──> server hydrate (shell::hydrate)
                  ──> arm the debounced draft writer, the warm-session marker,
                      the flush on hide and the cross-tab sync
engine task: RenderEngine::create ──> bind the slot and vehicle lanes ──> start_raf
             ──> world_assets::bootstrap for the document's terrain (everon when unset)
handshake: restore settled and world ready ──> hand_over ──> BootPhase::Ready
engine failure ──> BootPhase::Failed on the stage it reached; map_disabled holds the reason
```

Whichever task finishes second rebinds the engine from the settled document, so the map never draws
the seed once the restore has landed. The registry effect reads the item registry and the
compatibility feed from the bridge's `registry_session` cache when an earlier editor mount in the
tab filled it, and otherwise fetches them; a retry bumps `registry_fetch_gen` and fetches again, and
a failure marks both catalogs failed and the feed unavailable. `input_listeners::attach` builds the
`EditorGestureContext` and attaches the canvas gestures and the chord listener from
`apps/website/frontend/src/v2/apps/editor/input/`; its own `pointercancel` drops an armed place, a
pending connection and an in-flight gesture, and its window `resize` resizes the canvas backing
store at the current device-pixel ratio.

## Boundaries

- Depends on: the parent's imports: the bridge's document host, editor context, boot machine,
  viewport and world-assets host in `apps/website/frontend/src/v2/apps/editor/bridge/`; the
  session's draft writer, hydrate, review mode, warm-session marker and document commands in
  `apps/website/frontend/src/v2/apps/editor/shell/`; the registry fetches in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/registry_loading.rs`; the asset catalog
  and compatibility rules of `arsenal/`; the validation panel's compile findings; the input layer;
  `website_map_engine` (`frame::engine::RenderEngine`, `editing::persist::server_adoption`,
  `editing::hosted_commands`, `data::store`, `overlay::symbology`).
- Used by: the parent's `install_canvas_mount`, which the editor page
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` calls; the source pins that read
  these files in `apps/website/frontend/src/v2/apps/editor/tests/` (`mission_editor/source.rs` and
  the boot-progress, placement and hover-cursor pins) and in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/review_mode/read_only_review.rs`.
- Rules: the review restore reads nothing from the draft store and arms no draft writer, flush,
  marker or writer election (`the_review_boot_restores_the_reviewed_version_and_arms_nothing` in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/review_mode/read_only_review.rs`); a
  restored or reviewed document is applied under the init origin, so it opens with no undo step.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the fast initial load, the hydrate gate and local persistence.
