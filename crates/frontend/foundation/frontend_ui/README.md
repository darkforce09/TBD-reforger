# Frontend UI

The `frontend_ui` crate: the design-system primitives every page and panel of the single-page app
is assembled from, and the small helpers with no domain of their own. The primitives are the icon,
the page header, the status pill, the three form controls and the state classes they wear, the
list-and-detail layout, the notices, and the two overlay surfaces with the stack they share; the
helpers format byte counts, dates and countdowns, handle UTC instants without the browser clock,
apply the content URL policy and the avatar sanitiser, and make the app's one clipboard write. The
bottom of the frontend foundation: nothing here imports the session, the transport or a page.

## Contents

```text
crates/frontend/foundation/frontend_ui/
├── Cargo.toml  the package: `leptos`, `http_url_guard`, the browser crates for wasm32, layout tier 1
└── src/        the primitives, the overlay stack and the helpers, every module at the crate root
```

## How it works

A primitive holds no state that belongs to its caller. The three form controls are uncontrolled:
each reads its value from the caller's signal and writes it to the DOM property, so the caller
owns the value and a fast-changing one costs a property write rather than a render. `Dialog` and
`Sheet` render no DOM while closed and, while open, register with `modal_stack`, the one piece of
state here: the last-opened surface paints on top and is the only one the Escape key closes.
Every helper is total: an input it cannot read gives a placeholder, never a panic. The
[source tree README](src/README.md) describes each module.

Only code that calls a browser API directly is gated to the wasm32 build: the overlay stack's
window key listeners and polling timer, the browser's date object behind `datefmt` and
`countdown`, and `clipboard`. Everything else, the components included, compiles natively too, so
the crate's tests and every dependent crate's native tests reach it.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_ui   # the overlay stack, the form controls, the split pane, the helpers
```

## Configuration

None: no feature, no environment variable. The Material Symbols Outlined font the icons draw
with is loaded by `crates/frontend/shell/frontend_application/index.html`.

## Public surface

- Components: `Dialog`, `Sheet`, `MaterialIcon`, `PageHeader`, `SearchBox`, `Select`, `Slider`,
  `split_pane::{SplitPane, SplitPaneEmpty, GlassSplit, SidebarSearch, ListDetailItem}`,
  `toast::{Toasts, ToastViewport}`.
- Classes and constants: `cn`, `badge_class`, `DEFAULT_AVATAR`, `tokens::{HOVER_FILL,
  DISABLED_GLYPH}`, `split_pane::search_matches`.
- The overlay stack: `modal_stack::{ModalHandle, TransientCloserHandle, register, unregister,
  register_transient_closer, unregister_transient_closer, close_registered_transients,
  is_topmost_open, is_top_by_open_order, any_open, escape_consumed, mark_escape_consumed,
  z_class}`.
- Helpers: `byte_formatting`, `datefmt`, `countdown`, `utc_timestamp::UtcTimestamp`, `safe_url`,
  `safe_avatar_url`, `clipboard::write_clipboard` (wasm32).
- `prelude`: the components, `cn`, `badge_class`, `DEFAULT_AVATAR`, `Toasts` and
  `safe_avatar_url`.

## Boundaries

- Depends on: `leptos`; `http_url_guard`, for the avatar sanitiser; `time_source`, for the
  countdown's clock; `web-sys`, `js-sys`,
  `wasm-bindgen` and `wasm-bindgen-futures` in the wasm32 build only.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`): its route guard, content gates, app frame,
  features, pages and the Mission Creator, whose dialogs and menus join the overlay stack and
  whose chrome layout re-exports the state classes.
- Rules: only the topmost open overlay answers Escape
  (`only_the_topmost_open_overlay_answers_escape`); the form controls never re-render per event
  (`neither_control_re_renders_per_event`); `safe_url` answers as the API's content URL policy
  does (`the_case_lists_match_the_backend_policy_tests`); the crate depends on no frontend crate
  (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Design tokens](/documentation/design_system/design_tokens.md) — the palette, typography,
  spacing, radii and motion the primitives follow.
- [Interaction patterns](/documentation/design_system/interaction_patterns.md) — the split pane,
  create-over-list dialog and slide-over sheet the pages build from these primitives.
- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md#design) — the design references
  and tokens of the app.
