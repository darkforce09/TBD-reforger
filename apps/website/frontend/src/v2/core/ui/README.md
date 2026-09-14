# Aegis Design System Primitives (`src/v2/core/ui`)

This directory replaces the monolithic `1,611`-line `ui.rs` file by breaking its primitives into
focused modules.

## Primitives Catalog

The list below is the real set. There is **no** `button.rs`, `input.rs` or `tabs.rs`: buttons and
plain text inputs are written inline with the shared class recipes, and no tab switcher primitive
exists — the surfaces that look like tabs each build their own.

- **`icons.rs`**: the Material Symbols glyph wrapper, and the placeholder avatar data URI.
- **`page_header.rs`**: the page title block, and the `cn` class-string join every primitive uses.
- **`badge.rs`**: status-pill classes by variant name.
- **`gates.rs`**: `AuthGate` (signed in) and `AdminGate` (sufficiently privileged) content gates.
- **`slider.rs`**: the range control, with its track and handle painted rather than tinted.
- **`select.rs`**: the dropdown — a real `<select>`, with the browser's arrow replaced.
- **`search_box.rs`**: the filter field, with its own leading glyph and clear button.
- **`dialog.rs`**: the modal dialog — backdrop and centred panel, or no DOM at all.
- **`sheet.rs`**: the side sheet — the dialog's shape, anchored to an edge.
- **`modal_stack.rs`**: the overlay registry the dialog and sheet share, so one dismiss key closes
  exactly one surface and the paint order agrees with it.
- **`split_pane.rs`**: the master-detail layout, its row, its filter field, its empty state, and the
  shared match predicate.
- **`toast.rs`**: transient notices — the context pages raise them through, and the viewport.

## Invariants

- A primitive holds no state that belongs to its caller. The three form controls are uncontrolled:
  the value is read from the caller's signal and written to the DOM property.
- The overlay registry is the one exception — it has to know which surfaces are open.
