# Platform Navigation Frame (`pages/navigation`)

This directory houses the persistent navigation frame (Sidebar, Top Navigation Bar, and Mobile Drawer) that surrounds all standard platform pages.

---

## Responsibilities
- **`layout.md`**: Main application frame container (`AppLayout`). Renders the persistent 240px sidebar, the top navigation header with live server pulse and user profile avatar, and wraps `<main>`.
- **`nav_config.md`**: Central navigation registry defining sidebar links, grouping into the command hubs, role-based visibility, and active link highlights.

## Invariants
- When a user navigates between standard platform pages, this frame persists—only the `<main>` content swaps.
- For chromeless full-bleed routes (such as the Mission Creator in `apps/editor`), this navigation frame completely yields 100% of the viewport.
