**Status:** live

# Arsenal loadout editor

The Arsenal tab of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
Attributes dialog: a full loadout editor in which a mission maker dresses and arms one
[slot](/documentation_v2/glossary.md#slot) from the item
[registry](/documentation_v2/glossary.md#registry), sees it on a 3D doll, fills its containers
with cargo, and copies loadouts across a selection. It is the
[arsenal](/documentation_v2/glossary.md#arsenal), not the mission's
[armory](/documentation_v2/glossary.md#armory).

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/editor/arsenal/`](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md),
  with `ArsenalTab` in `mod.rs`, the loaded view in `tab_content.rs` and its
  [sections](/apps/website/frontend/src/v2/apps/editor/arsenal/tab_content/README.md), the
  [rules](/apps/website/frontend/src/v2/apps/editor/arsenal/rules/README.md), the
  [loadout core](/apps/website/frontend/src/v2/apps/editor/arsenal/loadout/README.md) and the
  document writes in `loadout_commands.rs`; the
  [panels](/apps/website/frontend/src/v2/apps/editor/ui/arsenal/README.md) and
  [cargo editor](/apps/website/frontend/src/v2/apps/editor/ui/arsenal/panels/README.md) in
  `apps/website/frontend/src/v2/apps/editor/ui/arsenal/`; the 3D doll in the map engine's
  [doll preview](/apps/website/map-engine/src/doll/README.md).
- Entry: the Attributes dialog
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal.rs`) mounts
  `ArsenalTab` on its fourth tab, "Arsenal". `open_arsenal(id)` in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/attributes_modal.rs`
  opens the dialog on that tab; the map's context menu entry "Edit Loadout..." and the ORBAT
  Manager slot inspector's "OPEN ARSENAL" button call it.
- Related features: the [Attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md),
  whose entry ATTR-TAB-004 inventories the tab; the
  [asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md),
  which reads the same registry through the arsenal's catalog trees.

## Behaviour

### Opening the tab

1. The tab mounts for the slot the dialog opened on and widens the dialog (`max-w-6xl` instead of
   `max-w-lg`). It shows "Loading catalog…" until the registry rows arrive, then the loaded view.
2. When the slot's loadout has no `cargo` key, the tab first asks the map engine to seed the
   character's default cargo, from the server's aggregated `character_default_cargo` edges.
3. The loaded view stacks, top to bottom: the header badges, a four-column grid (region rail, item
   list, doll, compatibility panel), the cargo editor, the verdict and file row, the loadout buffer
   row, and the status section.

### Picking an item

1. The rail lists the 14 loadout regions of `LOADOUT_ROWS`: "Primary", "Optic", "Magazine",
   "Launcher / 2nd rifle", "Handgun", "Throwable", "Helmet", "Jacket", "Pants", "Boots",
   "Vest (chest rig)", "Armored vest", "Backpack" and "Gloves". A click on a rail button, a doll
   region or an SVG hotspot makes that region active; the primary weapon is active at first.
2. The item list shows the active region's label, its option count, a filter box ("Filter
   {region}…") that clears when the region changes, "— None —", and the options grouped under
   their registry category in capitals, "OTHER" when an item has none.
3. Optic and magazine are edge rows: their options are the items the compatibility graph accepts
   on the picked primary weapon, and until the graph has loaded they offer only the current pick.
   Every other row lists the registry's items of its kind, without abstract or variant rows,
   sorted by name. A current pick the graph rejects stays listed, in the error colour, so it can
   be seen and cleared.
4. A click on an option writes the loadout to the [mission](/documentation_v2/glossary.md#mission)
   document at once, as one undo step. There is no Save button.
5. For a weapon region the compatibility panel adds "Attachments", the one multi-select: each
   toggle writes the weapon's whole attachment set as one pick.

### The doll

1. The doll column mounts the map engine's 3D doll: a drag turns the soldier, a click under the
   drag threshold (4 px) selects a region, the hovered region shows a tooltip, and the active one
   carries a name chip on a leader line.
2. When the 3D renderer cannot be created, and always in the native build, the column draws an
   SVG paper doll instead: one hotspot per region, dashed while empty and tinted once equipped.
3. Either doll only changes the active region; neither writes the loadout.

### Cargo

1. The cargo editor shows one group per worn container (`vest`, `pants`, `jacket`, `backpack`)
   that has a garment picked or holds cargo, with "−", "+" and "✕" on each row and an
   "+ Add item…" picker of magazines, ammunition, gear items, throwables and explosives.
2. Each group's budget line sums weight and volume against the garment's catalogued
   `max_weight_kg` and `max_volume_cm3` and turns to the alert colour when either is exceeded.
3. Every cargo change writes the loadout at once, like a pick. The first cargo edit adds the
   `cargo` key, so the default cargo is never seeded again; Remove Everything leaves `cargo: []`
   for the same reason.

### Verdict, import and export

1. The header shows the compatibility feed ("Compat active", "Compat loading…" or
   "Compat unavailable"), the catalogued capacity of the active container's garment ("max {kg} kg"
   and its grid), and the weight of the picks and their attachments ("{kg} kg · {n} items", or
   "≥ {kg} kg · {n} items without weight data" when some items lack a weight).
2. The verdict chip reads "Loadout valid" or "{n} issue(s)"; the issues are incompatible edge
   picks, attachments without or against their weapon, cargo over capacity and cargo in a
   container no picked garment wears. Each issue also shows on the row whose pick fixes it.
3. "Download loadout JSON" saves `loadout-export.json`, stamped with the catalog's `modpackId`.
   While any container is over capacity the button is disabled and the row reads
   "Export blocked — {n} container(s) over the catalogued capacity", with each reason in its
   tooltip. Cargo in an unworn container only warns: the slot's kit may supply the garment.
4. "Import loadout JSON" opens a file picker and checks the file in order: valid JSON, the shipped
   loadout-export schema, then the compatibility, attachment and capacity rules. A refused file
   changes nothing and lists every refusal; an accepted one replaces the picks and cargo in one
   write, and its receipt ends "One Ctrl+Z undoes the whole import." A file from another modpack
   only warns.

### The loadout buffer

1. "Copy" buffers the loadout of every selected entity; the buffer lives in the map engine and
   outlives the dialog, so a mission maker copies in one Arsenal and applies from another. The row
   shows "{n} buffered".
2. "Apply" writes one buffered loadout to each selected entity, drawn at random from a seeded,
   reproducible draw when several are buffered. The plan passes the import rules first and is all
   or nothing: a refusal reads "Nothing was applied — every selected loadout is unchanged." and
   names each bad row.
3. "Remove Everything" clears every wear row, weapon and cargo row on the selection.
4. Apply and Remove Everything ask the browser to confirm when they would change more than ten
   entities ("This will {verb} {n} entities. Continue?"). Their receipts count the writes the
   document took, not the plan. Picks and cargo act on the one open slot; the buffer row says
   "Buffer verbs: whole selection."

### Persistence line

The status section ends with one of three lines. "The mission has no unsaved changes." and "The
mission has unsaved changes — Save Version publishes them to the server." repeat the top strip's
unsaved marker, which the dialog's backdrop hides. When the slot is gone (deleted, or undone away
while the tab is open) the last pick is refused and the line says so instead, starting "That last
pick did NOT reach the mission document"; a refused write mints no undo step and leaves the
mission clean.

### Known discrepancies

- The SVG paper doll's comment calls its hotspots keyboard-accessible
  (`apps/website/frontend/src/v2/apps/editor/ui/arsenal/panels.rs:287-289`) — each hotspot has
  only a click handler (same file, :387), so Enter and Space do nothing on a focused hotspot.
- The Arsenal's over-capacity message ends in the caveat of
  `apps/website/frontend/src/v2/apps/editor/arsenal/rules/cargo_capacity_and_delivery.rs:136` —
  the save-time check in
  `apps/website/map-engine/src/data/scenario/validation/wire_safety/scan.rs:245` carries a
  different caveat from a hand copy of `CARGO_CONTAINERS` (same file, :241-242), so export and
  Save explain one fault differently.
- The Mission Creator's validation panel evaluates the rules with known asset ids only
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/validation_evaluation_and_findings.rs:12-15`)
  — the map engine's cargo and loadout policy rules never apply in the editor.

## Data

The editor page loads the registry and the compatibility feed once at boot, in
`apps/website/frontend/src/v2/apps/editor/mission_editor/registry_loading.rs`, and hands both to
the tab; the tab makes no call of its own.

- `GET /api/v1/registry?limit=500&offset=…` (`list_registry` in
  `apps/website/api_v2/src/missions/handlers/registry_items.rs`): read as `RegistryResponse` pages
  of `RegistryItem` until `total` is reached; the current modpack's flat catalog, for the
  `mission_maker` [role](/documentation_v2/glossary.md#role) and above, with a weak ETag.
- `GET /api/v1/registry/compat?edge_type=optic_on_weapon,mag_in_weapon,attachment_on_weapon`
  (`list_registry_compat` in `apps/website/api_v2/src/missions/handlers/registry_compat_graph.rs`):
  read as `RegistryCompatResponse`; the compatibility edges of those three families, which build
  `CompatGraph`.
- `GET /api/v1/registry/compat?view=cargo_defaults` (same handler): read as
  `RegistryCargoDefaultsResponse`; each character's default cargo, aggregated server-side from its
  `character_default_cargo` edges, which seeds a slot with no `cargo` key.
- The mission document: each pick writes the slot's `loadout` key as `SlotLoadoutV2` JSON through
  the map engine's `update_slot_loadout`; the buffer verbs commit a plan with
  `commit_loadout_writes`. The loadout reaches the server only when the mission maker saves a
  version: `POST /api/v1/missions/{id}/versions` (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`) refuses cargo over the
  catalogued capacity with a 400, from the same registry weights and volumes the Arsenal reads.
- `loadout-export.json`: the downloaded file, which follows
  `contracts_v2/definitions/loadout-export.schema.json`; the import gate checks the same schema,
  embedded at compile time.

## Design

- As built: a wide glass dialog; a 44 px icon rail, a 230 px item list, the doll filling the middle
  and a 230 px compatibility panel, then the cargo editor, the verdict and file row, the buffer row
  and the status lines. Every edit is live, with undo instead of a Save button.
- Design target: the [Loadout Forge blueprint](/documentation_v2/website/frontend/apps/editor/arsenal/visual_references/loadout_forge_blueprint/README.md)
  and the [Arsenal mock-up](/documentation_v2/website/frontend/apps/editor/arsenal/visual_references/arsenal_mockup/README.md),
  design-phase references that settle styling only.
- Differences from the target: the built tab has no template list, breadcrumb or
  "Save & Close Forge" and "Cancel" buttons, since every pick is written at once; its regions sit
  on a rail of 14 rather than a few slot cards around the figure; its attachments are compatibility
  toggles rather than optic and muzzle dropdowns; ammunition is cargo in a container rather than a
  magazine count; and "Apply Kit to Entire Squad" and "Apply Kit to Entire Faction" are replaced by
  Copy and Apply over the current selection.

## Open work

- [T-068 — Virtual Arsenal (registry + loadout export)](/documentation_v2/tickets/specs/t068_virtual_arsenal_program.md)
  (deferred, no plan): the program closes once its last child, T-068.14, passes.
- [T-068.14 — Phase 2 E2E gate editor to player](/documentation_v2/tickets/specs/t068_14_phase2_e2e_gate.md)
  (queued, no plan): a human sign-off that a loadout authored here dresses a player in game.
- [T-309 — FactionDoc squad level for Apply Template](/documentation_v2/tickets/specs/t309_faction_doc_squads.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-309_plan.md)): a saved faction template,
  which carries each role's loadout, keeps its squads when the ORBAT Manager applies it.
- [T-1035 — Fix arsenal paper-doll hotspots ignoring Enter and Space](/.ai/tickets/T-1035.toml)
  (idea, no plan): the SVG doll's hotspots answer the keyboard.
- [T-1040 — Fix hand-copied cargo container table and diverged capacity caveat](/.ai/tickets/T-1040.toml)
  (idea, no plan): export and Save share one container table and one caveat.
- [T-1047 — Decide whether Mission Creator validation gets cargo and loadout policy](/.ai/tickets/T-1047.toml)
  (idea, no plan): the validation panel checks cargo and loadouts, or those rules go.
- [T-1064 — Add guards for map binary formats and doll shader layout](/.ai/tickets/T-1064.toml)
  (idea, no plan): a test ties the 3D doll's regions to the rail's.

## Decisions

- Every pick and cargo edit writes the document at once, with no Save button: one undo step per
  pick matches the rest of the editor, and the dialog repeats the unsaved state because its
  backdrop hides the top strip.
- A refused write says so instead of reporting the pick saved: the persistence line reads the
  document's answer and the dirty flag, never a local counter.
- The rules never invent a capacity the catalog lacks, and an unworn-container warning never
  blocks export or Apply: the slot's kit may supply the garment.
- Clothing rows are never constrained by the compatibility graph: mixing garments is deliberate;
  only optic, magazine and attachments follow the graph.
- Apply draws one buffered loadout per target, uniform and independent per target and
  reproducible from the session seed: several copied kits spread over a selection, and the same
  gesture gives the same result.
- Mass application works on the current selection through the loadout buffer; the squad- and
  faction-wide "Standardization" of the design phase is deferred, as the
  [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md) records for the
  ORBAT Manager.
