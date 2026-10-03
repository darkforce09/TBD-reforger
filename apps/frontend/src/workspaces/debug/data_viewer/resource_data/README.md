# Equipment data viewer resource data

The data section of a resource: one scrolling surface that shows every native container of the
resource as a card with its fields, with no tree to navigate and no nested scrolling.

## Contents

```text
apps/frontend/src/workspaces/debug/data_viewer/resource_data/
├── card_grid.rs            `CardGrid`: windowed card rows that keep their anchor as content grows
├── component_card.rs       `ComponentCard`: one container, its field continuation and expanded evidence
├── contents_navigation.rs  `ContentsNavigation`: a flat, searchable menu that jumps to a container
├── field_row.rs            `FieldRow`: a field's value at once, its evidence on expansion
├── grid_layout.rs          measured row offsets for variable-height cards and scroll anchoring
├── grid_loading.rs         `BatchLoader` and `FocusCard`: paged card batches and exact-source jumps
├── inline_details.rs       `InlineDetails`: provenance and object links beside the expanded field
├── mod.rs                  `ResourceData`: the section's surface and the module tree
├── observers.rs            size observers that measure cards without adding scroll surfaces
└── value_details.rs        `ValueDetails`: large values expanded in place, in numbered continuations
```

## How it works

`ResourceData` mounts `ContentsNavigation` and a `CardGrid` for the selected resource. The grid
loads cards in paged batches through the viewer's requests (`BatchLoader`), renders only the rows
near the viewport, and measures each rendered card through `observers.rs` so `grid_layout.rs`
can place variable-height rows and keep the reader's anchor while cards above it grow. A link to
an exact source (a container and field) goes through `FocusCard`, which loads the batch that
holds it and scrolls it into view. Each `ComponentCard` pages its own fields; a `FieldRow`
shows the value at once, and expanding it opens `InlineDetails` or `ValueDetails` in place.

## Boundaries

- Depends on: the viewer's location, requests, browsing memory and layout in `apps/frontend/src/workspaces/debug/data_viewer/`, the value
  rendering in `apps/frontend/src/workspaces/debug/data_viewer/source_inspector/`, and `EquipmentResourceCardPage` and
  `EquipmentSourcePage` in `apps/frontend/src/foundation/transport/dto/equipment_data_viewer/`.
- Used by: `ResourceDetails` in `apps/frontend/src/workspaces/debug/data_viewer/resources/`.
- Rules: a card's anchor holds while content above it loads; an exact-source jump lands on the
  named container and field; navigating to a new source clears stale value ranges and search
  (`equipment_viewer_source_navigation_clears_stale_value_ranges_and_search` in
  `apps/frontend/src/workspaces/debug/data_viewer/tests/navigation.rs`).
