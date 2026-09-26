# Shared interface primitives

The design-system primitives every page and panel is assembled from: the icon, the page header,
the status pill, the two content gates, the three form controls, the list-and-detail layout, the
notices, and the two overlay surfaces with the stack they share.

## Contents

```text
apps/website/frontend/src/v2/core/ui/
├── badge.rs        `badge_class`: the status pill's border, fill and text colours by variant name
├── dialog.rs       `Dialog`: the modal surface, a backdrop and centred panel, or no DOM when closed
├── gates.rs        `AuthGate` and `AdminGate`: render a subtree only to a viewer who may see it
├── icons.rs        `MaterialIcon`, the Material Symbols glyph, and the placeholder avatar
├── modal_stack.rs  the stack the dialogs and sheets share: paint order and the one dismiss key
├── mod.rs          the module tree; re-exports the primitives
├── page_header.rs  `PageHeader`, the page title block, and `cn`, the class-string join
├── search_box.rs   `SearchBox`: a search input with its own glyph and clear button
├── select.rs       `Select`: a real `<select>` with the browser's arrow replaced by an icon
├── sheet.rs        `Sheet`: the dialog's shape anchored to an edge, for dossiers and detail panels
├── slider.rs       `Slider`: the range input with its track and handle painted
├── split_pane.rs   `SplitPane` and its row, filter field, empty state and match predicate
├── tests/          unit tests for the gates, the modal stack, the form controls and the split pane
└── toast.rs        `Toasts`: transient notices, the context that raises them and their viewport
```

## How it works

A primitive holds no state that belongs to its caller. The three form controls are uncontrolled:
each reads its value from the caller's signal and writes it to the DOM property, so the caller
owns the value and a fast-changing one costs a property write rather than a render. There is no
button, text input or tab primitive: callers write buttons and plain inputs inline, and each
surface that looks like tabs builds its own.

`Dialog` and `Sheet` render no DOM at all while closed, so a closed one cannot catch a click.
While open, each registers with `modal_stack`, the one piece of state here: the last-opened
surface paints on top and is the only one the Escape key closes, so a confirmation stacked over
an edit form closes alone. A small popover registers a closer instead of a stack entry, and any
overlay that opens closes it.

The gates derive what they show from the auth store through a memo:

| Viewer | `AuthGate` shows | `AdminGate` shows |
|---|---|---|
| session still restoring | "Loading session…" | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` | the same as `AuthGate` |
| signed in below the admin tier | its children | "Admin access required." |
| signed in as an administrator | its children | its children |

Each gate rebuilds its children only when that answer changes, so a token rotation or a profile
poll never remounts the page under it. `Toasts`, provided once at the shell root, adds a notice
through `success`, `error` or `message` that removes itself after four seconds, and its viewport
renders no DOM while the list is empty. `SearchBox`, `Select` and `Slider` take their hover and
disabled classes, `HOVER_FILL` and `DISABLED_GLYPH`, from the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
`apps/website/frontend/src/v2/apps/editor/shell/layout.rs`.

## Boundaries

- Depends on: `leptos`; `crate::v2::core::auth`, for the gates; the Mission Creator's
  `crate::v2::apps::editor::shell::layout`, which `search_box.rs`, `select.rs` and `slider.rs`
  import for the form controls' classes; `web-sys` and `wasm-bindgen` for the modal stack's key
  listeners and timer; the Material Symbols Outlined font `apps/website/frontend/index.html` loads.
- Used by: the route guard, the clipboard write and the avatar sanitiser elsewhere in
  `apps/website/frontend/src/v2/core/`; the pages and navigation frame under
  `apps/website/frontend/src/v2/pages/`; the Mission Creator under
  `apps/website/frontend/src/v2/apps/editor/`, whose dialogs and menus join the modal stack.
- Rules: only the topmost open overlay answers Escape
  (`only_the_topmost_open_overlay_answers_escape` in `tests/ui.rs`); the form controls never
  re-render per event (`neither_control_re_renders_per_event`, among the range and select tests
  under `tests/`); a gate waits out the session restore and rebuilds only
  when its admission changes (`the_sign_in_gate_waits_out_the_session_restore` and
  `the_admin_gate_rebuilds_its_children_only_when_the_role_crosses_the_tier` in
  `tests/gates.rs`).

## Related documentation

- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the design token
  reference: palette, typography, spacing, radii and motion.
- [Interaction patterns](/documentation_v2/design_system/interaction_patterns.md) — the split
  pane, create-over-list dialog and slide-over sheet the pages build from these primitives.
- [Frontend documentation](/documentation_v2/website/frontend/README.md#design) — the design references
  and tokens the primitives follow.
