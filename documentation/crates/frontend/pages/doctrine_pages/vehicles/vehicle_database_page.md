**Status:** live

# Vehicle database page

The `/vehicles` page, titled "Vehicle Database": signed-in members look up the vehicles the
community fields and faces, grouped by faction, and read one vehicle's identification dossier: its
armour class, whether it swims and what threatens it. An administrator also adds, edits and deletes
vehicles from the same page.

## Where it lives

- Code: [`crates/frontend/pages/doctrine_pages/src/vehicles/`](/crates/frontend/pages/doctrine_pages/src/vehicles/):
  `page.rs` holds the route component `VehicleDatabasePage`, the list fetch and the write
  callbacks; `vehicle_grid.rs` the faction-grouped list and the "Add vehicle" action;
  `spec_drawer.rs` the dossier with its Edit and Delete actions; `vehicle_form_dialog.rs` and
  `delete_confirmation.rs` the two dialogs; `vehicle_draft.rs` the form's validation;
  `vehicle_writes.rs` the requests. The folder's
  [README](/crates/frontend/pages/doctrine_pages/src/vehicles/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/crates/frontend/pages/doctrine_pages/src/vehicles/README.md#routes).
- Related: the [doctrine wiki page](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md),
  which holds the written manuals; the [API](/documentation/glossary/a_to_f.md#api)'s
  [vehicle database handlers](/crates/api/api_community_content/src/handlers/vehicle_database/README.md),
  which own the vehicle rows.

## Behaviour

The page body sits in `AuthGate`; the session, loading, failure, empty, form, refusal and
confirmation texts are in the README's
[States](/crates/frontend/pages/doctrine_pages/src/vehicles/README.md#states).

### Reading

1. The page fetches the whole vehicle list once, as typed rows, and both panes read it.
2. The list, headed "Vehicle Database", groups the vehicles by faction in the order each faction
   first appears; since the API sorts by name, a faction's place follows its first vehicle by
   name. A row without a faction is left out. Each row shows the name and the armour class.
3. "Search assets..." narrows the list to vehicles whose name, armour class or faction matches; a
   faction whose rows all drop out disappears.
4. The first vehicle is selected on arrival, and the selection falls back to it whenever the
   selected vehicle is no longer in the list.
5. The dossier shows the profile image, or a vehicle icon when the image URL is empty or is not a
   safe image source (`https://…` or a site path `/…`, checked by `safe_image_src`, the same policy
   the API applies when it stores the URL), then "ARMOR: <class>", "AMPHIB: <value>" when one is
   recorded, the faction chip and the name, "Primary Threat" (or "No primary threat recorded."),
   and "Telemetry" with "Faction", "Armor" and "Amphibious", where a missing amphibious value reads
   "—".
6. The amphibious chip warns for yes, `y` or `true`, reads as success for no, `n` or `false`, and
   stays neutral for anything else.

### Writing (administrators)

1. The write actions follow the signed-in role: "Add vehicle" beside the list heading, and "Edit"
   and "Delete" over the dossier's photograph, render only for an administrator, and never while
   the session restores.
2. "Add vehicle" opens the vehicle form empty; "Edit" opens the same form on the stored values. The
   form has one input per field: "Name", "Faction" and "Armor type" are required, "Amphibious",
   "Primary threat" and "Profile image URL" are optional, and each bounded field's label states its
   limit.
3. On submit the form applies the API's own rules before it sends anything: each value is trimmed,
   the byte order mark included; a limit counts characters (name 120, faction 60, armour type 60,
   amphibious 60, primary threat 120); a required field may not be blank; the image URL is empty or
   a safe image source. A broken rule marks its field with a sentence and sends nothing, so a
   refusal after that is never a field rule the form did not show.
4. A valid form sends the trimmed values: `POST` for a new vehicle, `PUT` with every field for a
   stored one. The accepted answer is the stored row: it replaces the row's old copy, or joins the
   list, in name order, and becomes the selection; a toast reads `Saved "<name>"`.
5. A refused write keeps the form open with its text and shows why: the API's own sentence for a
   `400`, and a fixed sentence for a request too large (`413`), a caller who is not an
   administrator (`403`), a vehicle that is gone (`404`), an ended session (`401`) or an answer
   that never arrived.
6. "Delete" asks first: "Delete this vehicle?", with the vehicle's name. "Delete vehicle" sends
   `DELETE`; once accepted, the vehicle leaves the list, the selection falls back to the first row
   and a toast reads `Deleted "<name>"`. A refused delete shows its sentence in the confirmation.
7. No write fetches the list again: the accepted row updates the fetched list in place, and the
   other rows keep the order the API sent.

### Known discrepancies

- A profile image URL that passes the policy but fails to load shows the browser's broken image:
  the dossier falls back to the vehicle icon only for an empty or unsafe URL (`spec_drawer.rs` in
  `crates/frontend/pages/doctrine_pages/src/vehicles/` sets no error handler).
- A saved row takes its place by comparing names as text, while the API orders the list in the
  database's collation; the two orders can differ for mixed case until the page is loaded again.

## Data

The README's [Data](/crates/frontend/pages/doctrine_pages/src/vehicles/README.md#data)
lists the calls and the fields the page reads and sends. Server-side, in
`crates/api/api_community_content/src/handlers/vehicle_database/`:

- `GET /api/v1/vehicle-database` (`list_vehicles` in `reads.rs`): any signed-in member; every
  vehicle row that is not deleted, ordered by name, then id; an empty amphibious value, threat or
  image is left off the row.
- `POST /api/v1/vehicle-database` (`create_vehicle` in `create_and_replace.rs`): `admin` only;
  answers 201 with the new row.
- `PUT /api/v1/vehicle-database/{id}` (`replace_vehicle`, same file): `admin` only; replaces every
  field and answers 200 with the stored row.
- `DELETE /api/v1/vehicle-database/{id}` (`delete_vehicle` in `delete.rs`): `admin` only; a soft
  delete that stamps the row deleted and answers 200 with it. A deleted row leaves the list and
  answers 404 on every route that names it; no route restores it.
- The writes share one validator (`validation.rs`) with the rules the form mirrors, refuse unknown
  keys, answer 400 with a sentence naming the field, and answer 413 with
  `details.code = request_too_large` for a body over the request limit. Each accepted write
  appends an audit line in the same transaction. The API also serves
  `GET /api/v1/vehicle-database/{id}` and `PATCH /api/v1/vehicle-database/{id}`; the page calls
  neither.

## Design

- A `GlassSplit` with an 18rem master list and the dossier as its detail. The dossier opens with a
  wide image area, then the chips, the name, the threat panel and the telemetry grid. The
  administrator's Edit and Delete sit as frosted buttons in the image area's top corner; the form
  and the delete confirmation are the shared `Dialog`.
- The dossier shows only the stored fields; it invents no speed, crew, armament or tactical
  directive.
- Design target: the [vehicle dossier blueprint](/documentation/crates/frontend/pages/doctrine_pages/vehicles/visual_references/vehicle_dossier_blueprint/README.md)
  and the [BTR-70 render](/documentation/crates/frontend/pages/doctrine_pages/vehicles/visual_references/btr_70_render/README.md),
  both design-phase references. The built page differs from the blueprint:
  - no wiki-style sidebar of manual sections, no "Publish Revision", "Archived" or "Export PDF"
    actions; editing is the form dialog instead;
  - the list is one searchable column grouped by the stored faction, with no flags;
  - the dossier has no description paragraph, no "CRITICAL" identification warning, no mobility,
    defence or capacity figures, no armament list and no row of threat icons: the stored row holds
    none of them;
  - the image area shows the stored profile image, not a render.

## Open work

None.

## Decisions

- The page shows only what a row stores: a missing value reads as absent, never as an invented
  figure, so the dossier cannot mislead a player identifying a vehicle.
- Vehicle identification is its own page, not a wiki manual: its rows are structured data with a
  faction and an armour class to search and group by.
- One fetch feeds both panes: search, selection and every accepted write cost no further request.
- The form checks the API's own field rules before it sends, so an administrator learns every
  broken field at once rather than one refusal per attempt; the API still checks every write.
- An edit sends `PUT` with every field rather than a `PATCH` of the changed ones: the form always
  holds the whole row, and replacing it keeps the stored row equal to what the form showed.
