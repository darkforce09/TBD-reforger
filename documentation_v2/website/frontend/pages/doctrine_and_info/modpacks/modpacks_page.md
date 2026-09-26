**Status:** live

# Modpacks page

The `/modpacks` page, titled "Modpacks": signed-in members see the modpacks the community's
servers run, which one is current, and the addons each needs, with a link to the Workshop
collection; administrators create, edit, make current and delete packs in place.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/`](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/):
  `page.rs` holds the route component `ModpacksPage` and the list fetch; `preset_list.rs` the pack
  list and "+ New"; `mod_table.rs` the read dossier; `pack_editor.rs` and `pack_edit.rs` the edit
  form and its request body. The folder's
  [README](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/README.md#routes).
- Related: the [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md), which owns the
  modpack routes; the servers and [events](/documentation_v2/glossary/a_to_f.md#event) that name a
  required modpack, which the API checks before a delete.

## Behaviour

The page body sits in `AuthGate`; the session, loading, failure and toast texts are in the
README's [States](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/README.md#states).

### Reading

1. The page fetches every pack with its addons once, and both panes read that list; the current
   pack comes first, then the newest.
2. The list, headed "Modpacks", shows each pack as "v<version> · <n> mods · <size>", with an
   "Active" chip on the current pack. "Search packs & mods…" matches a pack's name or any of its
   addon names.
3. The first pack is selected on arrival; choosing another returns the detail pane to reading.
4. The dossier shows the pack's name, "v<version>", "<size> total" and "<n> mods included", then
   each [mod](/documentation_v2/glossary/g_to_m.md#mod) with its Workshop id and "[ REQUIRED ]" when it is
   a key dependency.
5. "[ Launch Game & Auto-Download ]" launches nothing: it toasts "Launch requires the Reforger
   client". "View collection in Reforger Workshop ↗" opens the pack's Workshop URL in a new tab,
   and shows only when the pack has one.
6. Sizes print as `x.x GB` from a gigabyte up and as whole `MB` below.

### Administering

1. A signed-in administrator also gets "+ New", "[ read ]" and "[ edit ]", "Set current" on any
   pack that is not current, and "Delete". The controls read the signed-in, reactive role, so they
   never show while the session restores.
2. "+ New" creates a blank pack ("New Modpack", version `0.1.0`, no size, no addons), refetches
   the list and selects it.
3. "[ edit ]" opens the form: "Modpack name", "Version", "Workshop URL", "Mark as current modpack",
   and the addon rows, each with a "[ REQUIRED ]" toggle, a remove button and its Workshop id. A
   new addon takes a name and a Workshop id, then "Add".
4. The form seeds from the pack once, so an edit survives a refetch of the list. An empty name
   falls back to the stored name and an empty version to `0.0.0`. Each addon's position becomes its
   stored order, so the order changes only by removing and re-adding rows.
5. "Save Changes" ("Saving…") replaces the pack and its whole addon list, toasts
   `Saved "<name>"` and refetches; a refusal shows the API's sentence, or "Failed to save
   modpack", above the button. "Cancel" returns to reading.
6. "Set current" makes the pack the only current one. "Delete" deletes it at once, with no
   confirmation.

### Known discrepancies

- Every addon row in the edit form draws a drag handle, and the module header of
  `apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/pack_editor.rs` calls the rows
  reorderable, but no handler moves a row; `PackEdit::to_put_body` in `pack_edit.rs` stores each
  row's index as its order.
- The pack's download size shows in the list and the dossier, but the form has no size field: a
  pack created on the page keeps a size of 0 unless the API is called directly
  (`total_size_bytes` in `apps/website/api_v2/src/community_content/handlers/modpack_admin.rs`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/README.md#data)
lists each call with its DTO. Server-side:

- `GET /api/v1/modpacks` (`list_modpacks` in
  `apps/website/api_v2/src/community_content/handlers/modpack_catalog.rs`): any signed-in member;
  every pack with its addons, current first, then newest.
- `POST /api/v1/modpacks` (`create_modpack` in
  `apps/website/api_v2/src/community_content/handlers/modpack_admin.rs`): `admin` only; trims and
  requires the name, the version and every addon's name, refuses a negative size, and, when the
  new pack is current, clears the flag on every other pack in the same transaction.
- `PUT /api/v1/modpacks/{id}` (`replace_modpack`, same file): the same checks; replaces the pack's
  fields and deletes and re-inserts its whole addon list, so the pack and its addons never
  disagree.
- `POST /api/v1/modpacks/{id}/set-current` (`set_current_modpack`, same file): makes the pack the
  sole current one.
- `DELETE /api/v1/modpacks/{id}` (`delete_modpack`, same file): a hard delete of the pack and its
  addons, refused with 409 while a registry item, a server's required modpack or an event still
  names the pack; the refusal counts each.
- `GET /api/v1/modpacks/current` returns the current pack; the page does not call it.

## Design

- A `GlassSplit` with an 18rem pack list and the dossier or the form as its detail.
- Design target: the [modpacks blueprint](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/visual_references/modpacks_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Modpacks section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#7-modpacks).
  The built page differs from the blueprint:
  - a searchable list of packs sits beside one pack's dossier, instead of one wide card;
  - no explanatory line under the heading;
  - addons show their Workshop id and "[ REQUIRED ]", with no per-mod icon or "Verified" tag;
  - "[ Launch Game & Auto-Download ]" replaces "DIRECT CONNECT & AUTO-SYNC" and only toasts;
  - the administrator's controls and the edit form are additions.
- An earlier layout sketch drew an addons table with version, size and a Workshop link per addon
  and a "1-Click Update" button; the page shows neither per-addon sizes nor an update action.

## Open work

- [T-1038 — Fix modpack editor drag handles that cannot reorder addons](/.ai/tickets/T-1038.toml)
  (idea, no plan): the addon rows either reorder by drag or lose the handle.
- [T-135 — Mission modset manager](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-135_plan.md)): the page edits per-mission
  modset presets and shows how many missions use each, and export refuses a mission whose mods
  its preset does not carry.

## Decisions

- A save replaces the whole addon list: the pack and its addons are written together, so they can
  never disagree about which addons the pack holds.
- One pack is current at a time: making one current clears the flag everywhere else in the same
  transaction.
- A referenced pack cannot be deleted: servers, events and registry items name it without a
  foreign key, so the API counts the references and refuses rather than orphan them.
- The launch button does not pretend to launch: the browser cannot start the game, so it says so.
