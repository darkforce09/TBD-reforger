# Standalone ORBAT Selection (`/events/:id/missions/:emid/orbat`)

One mission's slotting, reachable directly by link — the same selector the operation dossier
embeds, on a page of its own.

## Architecture
- **`page.rs`**: the route component. Fetches `GET /events/:id`, looks the mission up in it by
  mission id for the heading and the caller's standing on it — reservation state, waiting
  position, seat eligibility and place outlook — and renders the back link, the heading, the
  notices about that standing (the operation dossier's own, such as a released signup's reason) and
  the selector.

## Not present in the legacy page
- **`squad_roster.rs`**: there is no roster of this page's own. The squad and slot tree is the
  operation dossier's slotting selector, mounted here unchanged — one implementation, two places
  it appears.
