# Modpacks Page (`/modpacks`)

Master-detail modpack manifests: the list of packs beside the addon manifest of the selected one.
Administrators can create a pack, edit it in place, make it the current pack, or delete it.

## Architecture
- **`page.rs`**: route component — fetches `GET /modpacks`, owns the selection and the read/edit
  mode, and composes the split view.
- **`preset_list.rs`**: the master pane — the title, the administrator's create button, the
  search box, and one row per pack with its version, addon count, size and active chip.
- **`mod_table.rs`**: the read dossier — the pack heading and totals, the table of included
  addons with their workshop ids and required flags, the launch and workshop links, and the
  make-current and delete actions.
- **`pack_editor.rs`**: the edit form — the pack fields, the editable addon rows, the row that
  appends an addon, and save / cancel.
- **`mode_toggle.rs`**: the read/edit switch and the mode it names, shared by both detail views.
- **`pack_edit.rs`**: the draft a pack is edited as, the reader that builds one from a fetched
  pack, the writer that turns one back into a request body, and the size formatter.
- **`tests/modpacks.rs`**: the guard that the create and edit affordances use the authenticated
  role check.

## Not present in the legacy page
- **1-Click Update button**: the dossier offers a launch button, which reports that launching
  needs the Reforger client, and a link to the workshop collection.
- **Checksum status and size columns in the addon table**: each addon row shows its name, its
  workshop id and whether it is required. Size is a per-pack total, not a per-addon column.
