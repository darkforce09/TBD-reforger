# macOS UX Methodology & Interaction Plan

This document focuses entirely on the underlying **User Experience (UX) Architecture**. The goal is to make the website *feel* and *behave* like a native Apple operating system through Context Retention, Progressive Disclosure, and Frictionless Action.

## Current UX Anti-Patterns (Code Audit Results)

Based on a deep code evaluation of the currently running frontend at `http://localhost:5173`, here are the specific structural UX issues that break the macOS methodology:

1. **The "Full Page Replace" (`EventsPage.tsx` -> `EventDetailPage.tsx`)**
   - Events are presented in a stacked vertical list. Clicking an event triggers a full route navigation to `/events/:slug`, entirely replacing the list view. To view another event, the user must hit "Back".
2. **The "Form-Over-List" Scrolling Nightmare (`AdminEventsTab.tsx`, `AdminAnnouncementsTab.tsx`)**
   - Inside the Admin tabs, there is a large "New/Edit" form that sits statically at the top of the page, pushing the actual list of events/announcements down below the fold. 
   - When a user clicks "Edit" on a list item, it populates the form at the top, forcing the user to mentally (and physically) scroll back up, completely losing visual context of the list.
3. **Heavy Toggles**
   - Changing simple states (like marking an event as published) currently requires opening the full edit form rather than a quick switch.

---

## Proposed macOS UX Architecture Refactor

To achieve the "Big Brain" macOS methodology without touching your beloved Sidebar, we need to fundamentally restructure the core interaction components:

### 1. The Apple Mail / Finder Layout (Split-Pane Architecture)
**Target Components:** `EventsPage.tsx`, `AnnouncementsPage.tsx`, and Admin sub-views.
- **The Fix:** We will convert these pages into strict **Split-Pane (Master-Detail)** layouts.
- **Interaction:** The left pane (approx 350px wide) will contain a persistent list of `EventCard`s. Clicking an event will update the URL, but the left pane *will not disappear*. Instead, the right pane instantly renders the `EventDetailPage.tsx`. 
- **macOS Vibe:** The selected event in the left pane will receive a prominent active/highlighted state (exactly like Apple Mail or Notes). You can rapidly click through 5 operations in 3 seconds.

### 2. Context Retention via Modals & Sheets
**Target Components:** `AdminEventsTab.tsx`, `AdminAnnouncementsTab.tsx`.
- **The Fix:** We will completely rip out the vertically stacked "New/Edit" forms that sit above the admin lists.
- **Interaction:** We will add a simple, primary `+` (Create) button to the top right of the lists. Clicking it, or clicking "Edit" on an existing item, will open a **centered frosted-glass Modal** (or a slide-down Sheet) *over* the list. 
- **macOS Vibe:** The user maintains visual context of the underlying list. When they click "Save," the modal vanishes, and they are exactly where they left off without any awkward scrolling or layout shifts.

### 3. Frictionless Action via Inline Editing
**Target Components:** Admin list rows (`AdminEventsTab.tsx`, `AdminAnnouncementsTab.tsx`).
- **The Fix:** Implement inline toggle switches directly within the list rows for simple boolean states.
- **Interaction:** If an admin needs to toggle `published`, `pinned`, or `signupsOpen`, they do not open an edit modal. They just click a toggle switch directly on the list row. 
- **macOS Vibe:** Status changes (Draft/Published/Live/Completed) will be handled via an inline dropdown menu on the list item itself. It saves instantly in the background, exactly like renaming a file in macOS Finder.

---

## Shipped macOS patterns (reference)

These pages already follow the methodology — use as implementation references:

| Pattern | Example | Spec |
|---------|---------|------|
| Split-pane master/detail | Event Schedule, Announcements, Wiki | [`event-schedule.md`](../../frontend/docs/pages/event-schedule.md) |
| Create-over-list Dialog | Event Manager "Schedule Operation" | [`event-manager.md`](../../frontend/docs/pages/event-manager.md), `admin.tsx` |
| Slide-over dossier (no route replace) | Mission Library card → Sheet | [`mission-library.md`](../../frontend/docs/pages/mission-library.md) |
| **Create-over-list Dialog (T-048)** | Mission Library **+ New Mission** → `CreateMissionDialog` | [`t048_library_create_dialog.md`](../../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md) |

**Anti-pattern removed by T-048:** standalone `/missions/create` full-page wizard + sidebar "Mission Creator" tab — creation is a transient action on the library surface, not a nav destination.
