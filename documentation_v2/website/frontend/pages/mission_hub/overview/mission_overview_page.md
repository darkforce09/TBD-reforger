**Status:** live

# Mission overview page

The `/missions/:id` page, the standalone dossier of one
[mission](/documentation_v2/glossary/g_to_m.md#mission): its briefing, details and
[armory](/documentation_v2/glossary/a_to_f.md#armory) for any signed-in member who may see it, and, for
the mission's author and administrators, the Edit Armory dialog and the mission's review record.
The same dossier body renders inside the mission library's slide-over.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/mission_hub/overview/`](/apps/website/frontend/src/v2/pages/mission_hub/overview/):
  `page.rs` holds the route component `MissionOverviewPage`, the mission fetch and the edit
  predicate; `dossier_body.rs` the shared read-only dossier and `mission_status_label`;
  `armory_editor.rs` and `armory_dialog.rs` the Edit Armory dialog. The folder's
  [README](/apps/website/frontend/src/v2/pages/mission_hub/overview/README.md) describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/mission_hub/overview/README.md#routes).
- Related: the [mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md),
  whose dossier sheet renders the same body; the review record in
  [`apps/website/frontend/src/v2/pages/mission_hub/mission_review/`](/apps/website/frontend/src/v2/pages/mission_hub/mission_review/README.md),
  which links the [review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md);
  the [event](/documentation_v2/glossary/a_to_f.md#event) hub, which groups the armory by faction for
  the players of an event.

## Behaviour

The page body sits in `AuthGate`; the session, loading and failure texts are in the README's
[States](/apps/website/frontend/src/v2/pages/mission_hub/overview/README.md#states).

### Reading the dossier

1. The page fetches the mission named by `:id`. A mission that is not live and not the viewer's
   own reads "Failed to load data.", since the [API](/documentation_v2/glossary/a_to_f.md#api) answers
   404 for it.
2. The header gives the title and "by <author> — Terrain: <terrain> — v<semver>", the version
   part only when the mission has a current version.
3. The body shows the mode and version badges, "Tactical Briefing" ("No briefing provided." when
   the briefing is empty or whitespace), the rows "Weather", "Time", "Max Players" and "Status",
   and "The Armory" with one tab per faction the stored armory rows name, the first selected, and
   each item's quantity as "x<n>", or "∞" for unlimited.
4. The status reads through `mission_status_label`, the one status wording of the platform:
   "Draft", "Open for review", "Live", "Returned" or "Archived".
5. For the author or an administrator, the review record follows under the dossier: the approved
   [artifact](/documentation_v2/glossary/a_to_f.md#artifact), every review with its decision, the thread
   and a reply box, each review linking its read-only review workspace.

### Editing the armory

1. The author or an administrator sees "Edit Armory" in the header. It opens the dialog on a
   snapshot of the mission taken at the click.
2. The dialog offers the faction keys of the mission's [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat)
   as tabs, never a free-text field, because the event hub joins armory rows to
   [slot](/documentation_v2/glossary/n_to_z.md#slot) factions by exact byte equality. The keys come from the current version's payload, from its top-level ORBAT
   when that is present and well formed and from the Mission Creator's factions otherwise, and a
   faction with no slot is left out. A key already stored that the ORBAT no longer names is still
   offered, marked, so saving does not delete its rows; a key the API would refuse (blank or
   padded) is shown quoted.
3. A mission with no ORBAT factions shows "This mission has no ORBAT factions yet.".
4. Per faction the author adds rows with an item name, a category and a quantity, where a blank
   quantity means unlimited, and removes rows. The dialog holds every faction's rows at once,
   because a save replaces the whole armory.
5. Before sending, the dialog refuses a row with no item name or a quantity that is not a number,
   with a toast naming the row.
6. "Save Armory" (or "Clear Armory" when no rows are left) replaces the armory, toasts
   "Armory saved — N items" or "Armory cleared", and refetches the mission, so the read-only armory
   shows what was stored.

### Known discrepancies

- "Edit Armory" shows to the author at any role (`can_edit` in
  `apps/website/frontend/src/v2/pages/mission_hub/overview/page.rs`, whose comment says the
  server's tier is authorship alone), but `set_armory` in
  `apps/website/api_v2/src/missions/handlers/mission_armory.rs` takes `MissionMakerUser`: an
  author demoted below mission maker sees the button and the save answers 403 "insufficient role".
- The review record's reply box shows to the same author, but `add_mission_review_comment` in
  `apps/website/api_v2/src/missions/handlers/mission_reviews.rs` also takes `MissionMakerUser`, so
  that author's reply is refused the same way.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/mission_hub/overview/README.md#data) lists
each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/missions/{id}` (`get_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): a live mission, or any mission
  for its author or an administrator; anyone else gets 404. It returns the card fields with the
  author's name and avatar and the caller's bookmark, the armory rows in their `sort_order`, and
  the current version with its payload, which the dialog reads the ORBAT factions from.
- `PUT /api/v1/missions/{id}/armory` (`set_armory` in
  `apps/website/api_v2/src/missions/handlers/mission_armory.rs`): a mission maker who is the
  author, or an administrator. `items` is required: `{"items":[]}` clears the armory, while a
  missing body, a missing `items`, a blank item name, or a missing, blank or padded faction answers
  400 and leaves the rows untouched. Every row is checked before the transaction, which deletes
  the old armory and inserts the new one; item names are stored trimmed and factions verbatim.
- `GET /api/v1/missions/{id}/reviews`, `POST /api/v1/missions/{id}/review-comments` and
  `POST /api/v1/missions/{id}/submit` (`apps/website/api_v2/src/missions/handlers/mission_reviews.rs`
  and `mission_submission.rs`): the review history for the author or an administrator, a thread
  comment on the newest review's artifact, and the resubmission of a mission awaiting approval
  with no review under way.

## Design

- One centred column, at most 48rem wide: the header, one glass card holding the dossier body, and
  the review record in a second glass card.
- The armory tabs sit above the item rows of the selected faction.
- Design target: the [mission overview blueprint](/documentation_v2/website/frontend/pages/mission_hub/overview/visual_references/mission_overview_blueprint/README.md),
  a design-phase reference drawn as a slide-over dossier, which the library's sheet and this page
  share. The built dossier differs:
  - no zone line or coordinates under the title;
  - "The Armory" with faction tabs replaces "Required Assets" with requisition codes;
  - no "Order of Battle" section with squad fill, no map preview and no briefing tabs;
  - "LAUNCH TACTICAL PLANNER" sits only in the library sheet's footer, where it toasts that the
    planner is coming;
  - the review record, "Edit Armory" and the detail rows are additions.
- The standalone page carries no command buttons: opening the Mission Creator is the library
  dossier's footer action, and no AAR link exists. No ticket is open for the ORBAT section, the map
  preview or the planner.

## Open work

- [T-1030 — Fix mission version save, set-current and armory skipping write lock](/.ai/tickets/T-1030.toml)
  (idea, no plan): the armory save takes the mission's row lock like the other writes, so a
  concurrent delete, demotion or change of author cannot race it.
- [T-846 — role_notice query is written on editor denial but never read](/.ai/tickets/T-846.toml)
  (deferred, no plan): a viewer the route guard sends here from a `mission_maker` route under
  `/missions/:id/` learns why.

## Decisions

- The dossier body is read-only and shared: it renders inside the library's slide-over too, where
  a form would stack an overlay on an overlay, so the armory editor is a dialog only this route
  opens.
- Faction keys are picked from the ORBAT, never typed: the armory joins the event hub's faction
  cards by exact bytes, and a typed key that misses renders an empty card while the save succeeds.
- The armory save is wholesale: the dialog holds every faction's rows, and a stored key outside
  the ORBAT stays on offer, so a save never drops rows the author cannot see.
- The edit controls follow authorship, not a role tier, on the page; the API adds the
  `mission_maker` tier on writes (see Known discrepancies).
