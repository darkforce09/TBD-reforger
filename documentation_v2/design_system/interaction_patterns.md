**Status:** live

# Interaction patterns

How the website's pages keep the user in place: the three principles its layout follows, taken
from the way desktop apps such as Mail and Finder work, and the four patterns that carry them.
Developers and agents read it before laying out a list, a detail view or a create form.

## Where it lives

- Code: the primitives in
  [`apps/website/frontend/src/v2/core/ui/`](/apps/website/frontend/src/v2/core/ui/README.md):
  `SplitPane`, `GlassSplit` and their filter field and empty state (`split_pane.rs`), `Dialog`
  (`dialog.rs`), `Sheet` (`sheet.rs`) and the modal stack they share (`modal_stack.rs`).
- Pages: the page folders under `apps/website/frontend/src/v2/pages/`, indexed with their feature
  docs in the [pages documentation](/documentation_v2/website/frontend/pages/README.md).
- Related: the [design tokens](/documentation_v2/design_system/design_tokens.md), whose motion
  rules the overlays follow, and the [mod design](/documentation_v2/mod/tbd-framework/mod_design.md),
  whose screens follow the same principles in game.

## Behaviour

### Principles

1. Context retention: acting on one item never takes the list away. Choosing an item fills a pane
   beside the list, and creating or editing one opens a surface over it; closing that surface
   returns the user to exactly where they were, with no scroll to recover.
2. Progressive disclosure: a surface shows the list and the one thing chosen, and the detail of
   anything else waits until it is asked for, in a pane, a sheet or a dialog.
3. Frictionless action: a small change costs one click where it stands, not a trip through a
   form.

### Split pane

A fixed-width list beside a flexible detail pane: `SplitPane` draws the master column (22rem by
default, at most 90% of the viewport) with an optional header above it, and the detail pane
beside it; `SplitPaneEmpty` fills the detail pane until something is chosen. Choosing a row
updates the detail pane and leaves the list, its scroll and its filter as they were; the
announcements feed also writes the choice into the route (`/announcements/:id`), so a link opens
the same item. `GlassSplit` wraps the same layout in the frosted panel of the reference pages.

- `SplitPane`: the event schedule, the announcements feed, and in administration the approvals
  queue, the audit logs, the content manager and server control.
- `GlassSplit`: the wiki, the vehicle catalogue and the modpacks.

### Create over the list

Creating or editing opens a centred `Dialog` over the list rather than a form above it or a page
of its own. The event manager's "Schedule Operation" opens the schedule dialog, the mission
library's "New Mission" opens the create dialog, and the event edit, personnel role and armory
dialogs work the same way. No route exists for a creation wizard: creating a mission is an action
on the library, not a place in the navigation. `Dialog` renders no DOM while closed and joins the
modal stack while open, so a confirmation stacked over an edit form closes alone on Escape.

### Slide-over dossier

A detail too large for a pane opens as a `Sheet`, the dialog's shape anchored to an edge, over
the page that opened it. The mission library opens a mission's dossier this way, at 60% of the
viewport width from Tailwind's `md` breakpoint up, and the leaderboards, the event manager's access panel and server
control's scenario and credential sheets do the same. The page underneath keeps its state, and
closing the sheet returns to it.

### Inline toggles

A two-state setting is a switch (`role="switch"`), as in the content manager's editor form.
Toggles on the list rows themselves, which change a published, pinned or sign-up state without
opening the item, are not built: every such change goes through the item's dialog or form.

## Data

None: the patterns are layout. Each page's calls are in its README's Data section.

## Design

The design target is the desktop-app methodology the platform adopted for its pages. The split
pane, create-over-list dialog and slide-over dossier are built as described; the inline row
toggles are the one pattern not built, and no open ticket plans them. The
[Mission Creator](/documentation_v2/glossary.md#mission-creator) follows the principles in its own
chrome: collapsible docks beside the map, its dialogs on the same modal stack.

## Open work

None: no ticket in `.ai/tickets/` with status idea, queued, ready, running, review or deferred
covers these patterns.

## Decisions

- Pages are split panes rather than a list page and a detail page: the user can move through
  several items in seconds without losing the list.
- A create or edit form is a dialog over the list, never a form stacked above it, which pushed
  the list below the fold and forced a scroll back after each edit.
- Creation is a transient action on the surface that lists the things created, not a navigation
  destination.
