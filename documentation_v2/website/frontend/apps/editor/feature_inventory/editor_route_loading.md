**Status:** live

# Editor route and boot loading

How a browser reaches the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) and
what it shows while the editor boots: the route and its access tier, the single application
bundle, and the progress overlay that covers the map until the
[mission](/documentation_v2/glossary/g_to_m.md#mission) and its terrain are loaded.

## Where it lives

- Code: the route in `apps/website/frontend/src/app_routes.rs` and its flags in
  `apps/website/frontend/src/router.rs`; the boot phases in
  `apps/website/frontend/src/v2/apps/editor/bridge/boot.rs`
  ([bridge README](/apps/website/frontend/src/v2/apps/editor/bridge/README.md)); the overlay in
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` (the
  [app README](/apps/website/frontend/src/v2/apps/editor/README.md)).
- Entry: `/missions/:id/edit`, component `MissionEditorPage`; the app README's
  [Routes](/apps/website/frontend/src/v2/apps/editor/README.md#routes) gives the route and the
  review workspace route that mounts the same page.
- Related features: [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md)
  (what the "Loading mission…" phase restores).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| FILE-ROUTE-001 | Editor code loaded as a separate chunk on first visit | not built |
| FILE-BOOT-001 | Boot progress overlay with retry | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
FILE-BOOT-001 is a row added for shipped code.

### FILE-ROUTE-001 — Route and bundle

1. The SPA is one client-side Leptos build served by Trunk; the editor ships inside the single
   WebAssembly bundle, so there is no lazy chunk and no chunk-loading fallback.
2. The route carries `auth: "mission_maker"`, `full_bleed: true` and `chromeless: true`: the
   platform's sidebar and top bar stay hidden and the editor fills the viewport.
3. The route guard checks the tier in the browser after mount; a viewer below
   `mission_maker` goes to the mission's overview, `/missions/:id?role_notice=mission_maker`.

### FILE-BOOT-001 — Boot overlay

1. While the page boots, an overlay covers the map with a progress bar in four segments:
   "Loading mission…", "Loading terrain…", "Loading satellite…" and "Loading world objects…".
   The mission segment tracks the download of the saved version.
2. A failed segment shows "{segment} failed" with the error, a "Retry" button that reloads the
   page and a "Continue without map" button that keeps editing without the terrain.
3. The overlay fades once the page reaches its ready phase.

### Known discrepancies

- `apps/website/frontend/src/router.rs` says in its header that the route table drives the
  router's `<Routes>` — the routes are a separate hand-kept list in
  `apps/website/frontend/src/app_routes.rs`, and the table feeds the route guard.

## Data

- The route itself makes no call; the boot's server and local-draft restore is described in
  [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md),
  and the terrain loads from `/map-assets`.

## Design

- The overlay is a blurred full-bleed layer with the segment captions over the bar.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  No design reference shows the overlay.

## Open work

- [T-717 — Continue-without-map before hydrate resurrects the boot overlay forever](/.ai/tickets/T-717.toml)
  (deferred, no plan): "Continue without map" dismisses the overlay for good whenever it is
  pressed.

No open ticket covers splitting the editor into its own chunk.

## Decisions

- The editor ships in the one bundle: the Leptos build is client-side only and has no route-level
  code splitting, so the boot overlay, not a chunk fallback, covers the load.
- A viewer below the tier is sent to the mission's overview with a role notice rather than shown
  a refusal inside the editor: the chromeless editor has no frame to show one in.
