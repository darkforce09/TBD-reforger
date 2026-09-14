# Hub 2: Operations (`src/v2/pages/operations`)

Governs the lifecycle of live milsim events: scheduling, briefings, and pre-match ORBAT slotting.

## Pages
1. **`schedule/` (`/events`)**: Operational calendar and event list.
2. **`event_detail/` (`/events/:id`)**: Comprehensive operation dossier and interactive slotting (replaces `event_hub.rs`).
3. **`orbat_selection/` (`/events/:id/missions/:emid/orbat`)**: Dedicated full-screen squad slotting view.
