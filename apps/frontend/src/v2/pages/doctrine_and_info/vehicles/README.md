# Vehicle database page

The `/vehicles` page: the vehicle identification index, a faction-grouped list beside the dossier
of the selected vehicle, with its photograph, armour class, amphibious flag and primary threat. An
administrator also adds, edits and deletes vehicles here.

## Contents

```text
apps/frontend/src/v2/pages/doctrine_and_info/vehicles/
├── delete_confirmation.rs  the confirmation before `DELETE /vehicle-database/{id}`, with its refusal line
├── mod.rs                  the module tree; re-exports `VehicleDatabasePage`
├── page.rs                 `VehicleDatabasePage`: the list fetch, the rows, selection, search, split view and dialogs
├── spec_drawer.rs          the dossier: safe photograph or placeholder, chips, threat, telemetry, admin actions
├── tests/                  unit tests for the validation, requests, refusal wording, rows, image and admin gate
├── vehicle_draft.rs        `VehicleDraft`, `FormTarget` and the validation that mirrors the backend's rules
├── vehicle_form_dialog.rs  the one form dialog that adds a vehicle and edits a stored one
├── vehicle_grid.rs         the master pane: the "Add vehicle" action, the search box and the faction groups
├── vehicle_rows.rs         where a saved row goes, which row leaves, and which row the dossier shows
├── vehicle_writes.rs       `VehicleRequest`: the POST, PUT and DELETE requests, and the call that sends one
└── write_refusal.rs        the sentence a refused write shows, per status
```

## How it works

`VehicleDatabasePage` renders inside `AuthGate` and fetches the vehicle list once, decoded as typed
`Vehicle` rows; both panes of the `GlassSplit` read that one list, held in a signal. The selection
starts on the first row and falls back to it whenever the selected id names no vehicle.
`faction_order` groups the rows by faction in the order each faction first appears, which is the
name order the [API](/documentation/glossary/a_to_f.md#api) returns; a row without a faction is
left out of the list. The search matches a vehicle's name, armour class or faction, and a group
whose rows all filter out disappears. The dossier renders every optional field as possibly empty:
the photograph loads only when `safe_image_src` admits the stored URL (`https://…` or a site path
`/…`), so an empty or unsafe URL shows a vehicle icon; an empty amphibious value drops its chip and
reads "—" in the readouts, and an empty threat shows a fixed line. The amphibious chip is a warning
for yes, `y` or `true`, a success for no, `n` or `false`, and neutral otherwise.

`is_admin` is a memo over `has_min_role_authed` and the session's
[role](/documentation/glossary/n_to_z.md#role), so "Add vehicle" and the dossier's Edit and
Delete appear only for a signed-in administrator, never for anyone else or while the session
restores. One form dialog serves both writes: its `FormTarget` seeds the text (empty, or the stored
row) and picks the request (`POST` a new vehicle, `PUT` a stored one); every opening re-creates the
form. On submit, `VehicleDraft::validate` applies the backend validator's rules: each value is
trimmed (the byte order mark included), a limit counts characters, `name` (at most 120), `faction`
(at most 60) and `armor_type` (at most 60) are required, `amphibious` (at most 60) and
`primary_threat` (at most 120) are optional, and `profile_image_url` is empty or a safe image URL. A
broken rule marks its field and sends nothing; a valid draft sends the trimmed values. A refused
write keeps the dialog open with its text and shows `refusal_sentence`: the backend's own sentence
for a `400`, and a fixed sentence for a `413`, `403`, `404`, `401` or an unreadable answer. Delete
asks first, then sends `DELETE`. An accepted write changes the fetched list in place: a saved row
replaces its stored copy and takes its place by name, then id; a deleted row leaves and the
selection falls back to the first row. Paths carry the id percent-encoded as one segment.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/vehicles` | `VehicleDatabasePage` | route tier `none`; the list renders only for a signed-in viewer, the write actions only for `admin` | full-bleed inside the navigation frame; breadcrumb Doctrine & Info / Vehicle Database |

## Data

- `GET /api/v1/vehicle-database`: read as `DataEnvelope<Vehicle>` (`dto::vehicles::Vehicle`):
  `id`, `name`, `faction`, `armor_type`, and `amphibious`, `primary_threat` and
  `profile_image_url`, which are absent when empty.
- `POST /api/v1/vehicle-database`: sends a `VehicleWrite` with every field, read back as the new
  `Vehicle`.
- `PUT /api/v1/vehicle-database/{id}`: sends a `VehicleWrite` with every field, read back as the
  stored `Vehicle`.
- `DELETE /api/v1/vehicle-database/{id}`: no body, read back as the deleted `Vehicle`.
- The writes go through the refusal-keeping verbs (`api_post_keeping_refusal`,
  `api_put_keeping_refusal`, `api_delete_keeping_refusal`), so a refusal arrives as an
  `ApiRefusal`. The page reads the `AuthStore` and the toast queue from context. The requests run
  in the browser build only; a native build shows the failure line and its buttons send nothing.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load vehicles." |
| empty | "No vehicles in the database." |
| loaded | "Vehicle Database" and the "Search assets..." box over the faction groups, each row showing a name and armour class; the dossier with "ARMOR: <class>", "AMPHIB: <value>" when set, the faction chip, the name, "Primary Threat" and "Telemetry" with "Faction", "Armor" and "Amphibious" |
| loaded, administrator | also "Add vehicle" beside "Vehicle Database", and "Edit" and "Delete" over the photograph |
| no threat recorded | "No primary threat recorded." |
| form | "Add vehicle" or "Edit vehicle"; "Name", "Faction", "Armor type", "Amphibious (optional)", "Primary threat (optional)" and "Profile image URL (optional)", each bounded field labelled "· at most <n> characters"; "Cancel" and "Add vehicle" or "Save changes" ("Saving…") |
| form refused by a rule | under the field: "<Field> is required.", "<Field> must be at most <n> characters." or "Profile image URL must be an https:// URL or a site path such as /uploads/…." |
| write refused | the backend's sentence for a `400`, else "The vehicle could not be saved." or "The vehicle could not be deleted."; "The vehicle is too large to send. Shorten its fields and try again." (`413`); "Only administrators can change the vehicle database." (`403`); "This vehicle is no longer in the database. Reload the page to see the current list." (`404`); "Your session has ended. Sign in again, then retry." (`401`); "The platform could not be reached, or its answer could not be read. Reload the page to see whether the change landed before trying again." |
| delete confirmation | "Delete this vehicle?", "The vehicle leaves the vehicle database for every member. No page can bring it back.", the vehicle's name, "Cancel" and "Delete vehicle" ("Deleting…") |
| toasts | `Saved "<name>"` and `Deleted "<name>"` |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, the refusal-keeping verbs, `ApiRefusal`,
  `DataEnvelope`, `dto::vehicles::{Vehicle, VehicleWrite}` and `endpoints::encode_path_segment`);
  `crate::v2::core::auth` (`has_min_role_authed`, `Role`, the `AuthStore` context);
  `crate::v2::core::ui` (`AuthGate`, `Dialog`, the `split_pane` primitives, `MaterialIcon`, the
  toasts); `crate::v2::core::utils::safe_url::safe_image_src`.
- Used by: the `/vehicles` route in `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/router.rs`; the sidebar's "Vehicle Database" link in
  `apps/frontend/src/v2/pages/navigation/nav_config.rs`; the DOM oracle's `vehicles`
  capture in `tools/developer_tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: groups keep the order each faction first appears (`factions_preserve_first_seen_order` in
  `tests/vehicle_grid.rs`); the form's rules match the backend validator's boundaries
  (`each_limit_admits_its_boundary_and_refuses_one_character_more` and
  `profile_image_url_admits_https_and_site_paths_and_refuses_everything_else` in
  `tests/vehicle_draft.rs`); the write body carries exactly the contract's keys
  (`the_write_body_carries_exactly_the_keys_the_contract_names` in `tests/vehicle_writes.rs`); an
  unsafe image URL shows the placeholder (`an_empty_or_unsafe_image_url_shows_the_placeholder` in
  `tests/spec_drawer.rs`); the write actions follow the signed-in administrator role
  (`the_write_actions_follow_the_signed_in_administrator_role` in `tests/page.rs`); every pane
  reads the one fetched list, which a write changes in place.

## Related documentation

- [Vehicle database page](/documentation/apps/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
  — the page's behaviour and design.
- [Vehicle database handlers](/apps/api/src/community_content/handlers/vehicle_database/README.md)
  — the routes, the validator and the soft delete.
- [Vehicle database contract](/contracts/definitions/vehicle-database.schema.json) — the row,
  the list and the write bodies.
