# Editor payload and export envelope

Turns the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document into the
editor payload a [mission](/documentation_v2/glossary.md#mission) version saves, wraps a payload in
the JSON export envelope, and writes the request body of a version save. The module is exposed as
`data::scenario::compile`.

## Contents

```text
apps/website/map-engine/src/data/scenario/compiler/payload/
├── export.rs          `version_body` and `version_body_to_writer`: the body of a version save
├── mod.rs             the module tree; re-exports the compile functions, key list and bounds
├── serialization.rs   `compile_payload` and `compile_export`: the saved payload and the envelope
├── terrain_bounds.rs  `terrain_bounds`: a terrain's playable rectangle in metres
└── tests/             unit tests for the save and export shapes, row order, extras and the body
```

## How it works

`compile_payload(small_maps_json, slots_json, include_orbat)` reads the by-id maps and the
[slots](/documentation_v2/glossary.md#slot) that the document store projects to JSON
(`MissionDocCore::small_maps_json` and `slots_json` in `crate::data::store`) and builds the payload:

- `schemaVersion` (the document's, else 1), `map` with `terrain` (`everon` when unset) and
  `bounds` (`terrain_bounds` of that terrain when unset), `environment` and `loadouts`;
- the `objectives`, `vehicles`, `entities` and `markers` lists and the `editor` graph's
  `factions`, `squads`, `slots` and `editorLayers`, each in the document's `entityOrder`, then the
  rows the order does not name;
- `title` when the document's title is not blank;
- `orbat` only when `include_orbat` is true (the Export path), derived with
  `crate::data::scenario::orbat`; a saved payload has none, and the server derives the
  [ORBAT](/documentation_v2/glossary.md#orbat);
- the authored blocks, which `crate::data::scenario::extensions` copies from the environment bag
  to the payload root, and every `payloadExtras` key that no known key or authored block claims;
  the store hands its `zones`, `compositions`, `triggers`, `comments` and `connections` rows over
  that way.

`compile_export` wraps a payload in the envelope a mission maker downloads: `exportFormatVersion`
1, the document's mission id (else the argument), the title (`Untitled Mission` when blank), the
terrain, weather and time of day (`everon`, `clear` and `06:00` when unset), the library blurb as
`briefing`, the version and `exportedAt`. `version_body` builds `{semver, editor_notes, payload}`,
the body of `POST /api/v1/missions/{id}/versions`, as a `serde_json::Value`, which clones the
payload; `version_body_to_writer` streams the same bytes into a writer without that copy.
`terrain_bounds` answers a 4,096 m square for `arland` and a 12,800 m square for every other
terrain.

## Boundaries

- Depends on: `crate::data::scenario::orbat` (`derive_orbat_from_editor`) and
  `crate::data::scenario::extensions` (the authored-block list); `serde_json`.
- Used by:
  - `crate::data::scenario::flatten` and `validate` (`terrain_bounds`), the document operations
    in `crate::data::store::operations` (`terrain_bounds`), and `crate::editing::persist`, which
    compares a local draft with the server's version through `compile_payload`;
  - the Mission Creator's Save and Export
    (`apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`), its validation
    panel's payload (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`) and
    its zone inspector (`apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/`);
  - the mission library's document upload, through `version_body_to_writer`
    (`apps/website/frontend/src/v2/pages/mission_hub/library/dossier_upload_panel.rs`).
- Rules:
  - a saved payload has no `orbat` key, and an export's ORBAT runs faction, squad, then `index`
    (`save_payload_omits_orbat_and_has_editor_shape`,
    `export_orbat_is_faction_then_squad_then_index_sorted` in `tests/cases_1.rs`);
  - both version-body builders write identical bytes
    (`both_doors_onto_create_version_serialise_identical_bytes`);
  - `payloadExtras` never overwrites or re-emits a known key
    (`payload_extras_merge_does_not_overwrite_known_keys`,
    `payload_extras_key_name_never_promoted_onto_wire`), and `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`
    leaves out the five store rows that travel as extras
    (`editor_only_and_transitional_keys_stay_absent_from_compile_known_list`);
  - the envelope's `briefing` is the library blurb, never a faction's briefing
    (`export_envelope_briefing_is_the_row_blurb_not_the_faction_block`).
