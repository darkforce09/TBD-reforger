# Equipment data viewer layout

The viewer's own layout pieces: resizable panels, pagers, search boxes, loading and error
feedback, and a virtual list for long navigation columns.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/data_viewer/layout/
├── mod.rs           `Pager`, `Search` and `Feedback`
├── panels.rs        `Panel`: pointer and keyboard resizing kept local to the viewer
└── virtual_list.rs  `VirtualList`: fixed-height rows, only the visible window plus overscan mounted
```

## Boundaries

- Depends on: `leptos`, and the viewer context of `page.rs` in
  `apps/website/frontend/src/v2/apps/debug/data_viewer/` for the pager.
- Used by: the tab components of `apps/website/frontend/src/v2/apps/debug/data_viewer/`.
- Rules: the pieces belong to the viewer and import nothing from the design system in
  `apps/website/frontend/src/v2/core/ui/`, so the bench stays independent of the app's chrome; a
  virtual list mounts only the rows in view plus its overscan.
