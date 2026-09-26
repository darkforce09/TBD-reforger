# Mission overview page

The `/missions/:id` page: one [mission](/documentation_v2/glossary/g_to_m.md#mission)'s dossier, with its
briefing, details and [armory](/documentation_v2/glossary/a_to_f.md#armory), and for its author and
administrators the Edit Armory dialog and the mission's review record.

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/overview/
├── armory_dialog.rs   the Edit Armory dialog: one faction at a time, the whole armory saved at once
├── armory_editor.rs   `ArmoryEditor`: the dialog's signals, faction keys, draft rows, guards, body
├── dossier_body.rs    the shared read-only dossier and the formatters the library reads it through
├── header.rs          the title, the attribution line and the Edit Armory button
├── intel_briefing.rs  the Tactical Briefing section, with its empty-briefing rule
├── mod.rs             the module tree; re-exports the page, `dossier_body`, `mission_status_label`
├── page.rs            the route component: the mission fetch, the edit predicate and the layout
└── tests/             unit tests for the faction keys, armory guards and body, briefing and status
```

## How it works

`MissionOverviewPage` renders inside `AuthGate`. The signed-in half fetches the mission, keyed on
`:id` and on the armory editor's saved counter, so a saved armory reads the mission again. The
viewer's account id and administrator standing come from the session through a memo, and the page
mirrors the [API](/documentation_v2/glossary/a_to_f.md#api)'s predicate: the author or an administrator,
never a [role](/documentation_v2/glossary/n_to_z.md#role) tier, gets the Edit Armory button and the review
record (`MissionReviewRecord` from `apps/website/frontend/src/v2/pages/mission_hub/mission_review/`)
under the dossier.

`dossier_body` is the one rendering of a mission's facts: the badges, the Tactical Briefing, the
detail grid and The Armory with a tab per faction the armory rows name. The library's slide-over
renders the same body, so it stays read-only; the page's one authoring surface is the Edit Armory
dialog, which only this route opens. `mission_status_label` is the platform's one status label
("Draft", "Open for review", "Live", "Returned", "Archived"), shared with the library's cards. There
is one briefing section, not a tabbed one, and no map preview or per-side
[slot](/documentation_v2/glossary/n_to_z.md#slot) census.

The dialog edits one faction at a time but always holds every faction's rows, because a save
replaces the mission's whole armory. The faction keys come from the mission's own
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat), compared byte for byte by the
[event](/documentation_v2/glossary/a_to_f.md#event) hub, so they are picked, never typed; a stored key the
ORBAT no longer names stays on offer, marked, so saving does not delete its rows. The guards run
before anything is sent: an item needs a name, and a quantity is a number or blank for unlimited.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id` | `MissionOverviewPage` | route tier `none`; the data renders only for a signed-in viewer; the Edit Armory button and the review record only for the author or an administrator | padded inside the navigation frame; breadcrumb Mission Hub / Mission Overview |

## Data

- `GET /api/v1/missions/{id}`: the mission, read as `MissionDetail`.
- `PUT /api/v1/missions/{id}/armory` with `{ "items": [...] }`, each item's `faction`, `category`,
  `item_name`, `quantity` (null for unlimited) and `sort_order`; the reply is read as
  `serde_json::Value` and a refusal's message shows as is.
- The review record's calls, as `MissionReviewRecord` makes them:
  `GET /api/v1/missions/{id}/reviews`, `POST /api/v1/missions/{id}/review-comments` and, to
  resubmit, `POST /api/v1/missions/{id}/submit`.
- The page reads the session (the viewer's Discord id and role) from the `AuthStore` context and
  the `:id` parameter from the router. It does not read the `role_notice` query parameter the route
  guard adds when it sends a viewer here from a `mission_maker` route. Every call runs in the
  browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| loaded | the title, "by <author> — Terrain: <terrain> — v<semver>", the badges, "Tactical Briefing", the "Weather", "Time", "Max Players" and "Status" rows, and "The Armory" with quantities as "x<n>" |
| no briefing | "No briefing provided." under "Tactical Briefing" |
| author or administrator | the "Edit Armory" button and the review record under the dossier |
| armory dialog | "Edit Armory", the faction tabs, "No items for this faction yet.", the "Item (e.g. M4A1)", "Category" and "Qty" fields with "Add", "Blank quantity = unlimited (∞).", "N items across M factions" and "Save Armory", or "Clear Armory" when no rows are left |
| no ORBAT factions | "This mission has no ORBAT factions yet." in the dialog |
| saving | "Saving…"; then the toast "Armory saved — N items" or "Armory cleared", or the refusal, or "Could not save the armory" |
| draft refused | a toast "An item under <faction> has no name. The armory endpoint rejects a blank item_name." or "“<item>” has a non-numeric quantity. Leave it blank for unlimited." |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` and `api_put` client, `MissionDetail`),
  `crate::v2::core::auth` (`AuthStore`, `Role`), `crate::v2::core::ui` (`AuthGate`, `Dialog`,
  `MaterialIcon`, the toasts) and `MissionReviewRecord` from the review record folder.
- Used by: the `/missions/:id` route in `apps/website/frontend/src/app_routes.rs`; the library in
  `apps/website/frontend/src/v2/pages/mission_hub/library/` renders `dossier_body` in its
  slide-over and labels its cards with `mission_status_label`; `mission_overview_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs` reads its source files.
- Rules: the dossier body stays read-only, and the card badge and the dossier grid share one status
  label (`the_card_badge_and_the_dossier_grid_share_one_label_mapper` in
  `tests/mission_overview.rs`); faction keys come verbatim from the ORBAT
  (`derives_editor_faction_keys_verbatim`); the draft guard flags exactly what the endpoint refuses
  (`draft_problem_flags_exactly_what_the_endpoint_refuses`); a whitespace briefing reads as empty
  (`tactical_briefing_trims_whitespace_only_to_empty_affordance`).

## Related documentation

- [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
  — the page's behaviour and design.
