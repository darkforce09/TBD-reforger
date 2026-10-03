# Shared interface primitives

The design-system primitives every page and panel is assembled from: the icon, the page header,
the status pill, the three form controls and the state classes they wear, the list-and-detail
layout, the notices, and the two overlay surfaces with the stack they share. The bottom of the
foundation: nothing here imports the session, the transport or a page.

## Contents

```text
apps/frontend/src/foundation/ui/
├── badge.rs        `badge_class`: the status pill's border, fill and text colours by variant name
├── dialog.rs       `Dialog`: the modal surface, a backdrop and centred panel, or no DOM when closed
├── icons.rs        `MaterialIcon`, the Material Symbols glyph, and the placeholder avatar
├── modal_stack.rs  the stack the dialogs and sheets share: paint order and the one dismiss key
├── mod.rs          the module tree; re-exports the primitives
├── page_header.rs  `PageHeader`, the page title block, and `cn`, the class-string join
├── search_box.rs   `SearchBox`: a search input with its own glyph and clear button
├── select.rs       `Select`: a real `<select>` with the browser's arrow replaced by an icon
├── sheet.rs        `Sheet`: the dialog's shape anchored to an edge, for dossiers and detail panels
├── slider.rs       `Slider`: the range input with its track and handle painted
├── split_pane.rs   `SplitPane` and its row, filter field, empty state and match predicate
├── tests/          unit tests for the modal stack, the form controls and the split pane
├── toast.rs        `Toasts`: transient notices, the context that raises them and their viewport
└── tokens.rs       `HOVER_FILL` and `DISABLED_GLYPH`: the hover and disabled state classes
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

`Toasts`, provided once at the shell root, adds a notice
through `success`, `error` or `message` that removes itself after four seconds, and its viewport
renders no DOM while the list is empty. `SearchBox`, `Select` and `Slider` take their hover and
disabled classes, `HOVER_FILL` and `DISABLED_GLYPH`, from `tokens.rs`; the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s chrome layout
(`apps/frontend/src/workspaces/editor/session/layout.rs`) re-exports the same two, so the chrome
and the form controls share one hover fill and one disabled dimming. `dialog.rs`, `select.rs`,
`slider.rs` and `toast.rs` compile into the browser (wasm32) build only, because every view that
uses them is a browser view. `icons.rs`, `modal_stack.rs`, `page_header.rs`, `split_pane.rs` and
`tokens.rs` also compile into the native test build, where their pure parts (the class joiner,
the overlay order, the list search match) and the chrome layout's re-export of the tokens are
checked.

## Boundaries

- Depends on: `leptos`; `web-sys` and `wasm-bindgen` for the modal stack's key listeners and
  timer; the Material Symbols Outlined font `apps/frontend/index.html` loads.
- Used by: the route guard, the content gates, the clipboard write and the avatar sanitiser
  elsewhere in `apps/frontend/src/foundation/`; the app frame under `apps/frontend/src/shell/`; the features
  and pages under `apps/frontend/src/features/` and `apps/frontend/src/pages/`; the Mission
  Creator under
  `apps/frontend/src/workspaces/editor/`, whose dialogs and menus join the modal stack and whose
  chrome layout re-exports the state classes.
- Rules: only the topmost open overlay answers Escape
  (`only_the_topmost_open_overlay_answers_escape` in `tests/ui.rs`); the form controls never
  re-render per event (`neither_control_re_renders_per_event`, among the range and select tests
  under `tests/`); the form controls consume the state classes rather than re-typing a hover fill
  (`both_controls_consume_the_t668_recipes` in `tests/ui_t633_range_and_select.rs`).

## Related documentation

- [Design tokens](/documentation/design_system/design_tokens.md) — the design token
  reference: palette, typography, spacing, radii and motion.
- [Interaction patterns](/documentation/design_system/interaction_patterns.md) — the split
  pane, create-over-list dialog and slide-over sheet the pages build from these primitives.
- [Frontend documentation](/documentation/apps/frontend/README.md#design) — the design references
  and tokens the primitives follow.
