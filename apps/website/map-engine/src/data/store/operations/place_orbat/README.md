# Character placement into the ORBAT

Files a character placed on the map into its side's [ORBAT](/documentation_v2/glossary.md#orbat)
in the [mission](/documentation_v2/glossary.md#mission) document: the side's faction, the squad the
new [slot](/documentation_v2/glossary.md#slot) joins, its place in that squad and its leadership.

## Contents

```text
apps/website/map-engine/src/data/store/operations/place_orbat/
├── mod.rs        the module tree; re-exports `place_character_under_side` and `PlaceOrbatError`
├── placement.rs  the side check, the side's faction, the open squad, the slot write and the leader
└── tests/        unit tests for placement per side, squad reuse, authored squads and undo
```

## How it works

`place_character_under_side(doc, side, slot_id, layer_id, role, tag, asset_id, x, y, z, rotation)`
refuses a side other than `BLUFOR`, `OPFOR` or `INDFOR` with `PlaceOrbatError::InvalidSide`, then
adds the faction `faction-<side>` when the document has none. The slot joins the side's last squad
while that squad is still open (an empty or minted `Squad <n>` name, no callsign, no vehicles);
otherwise a new `squad-<side>-<n>` is added, named `Squad <k>` with `k` one past the side's squad
count. The slot is appended to the squad's `slotIds`, and becomes its leader only as the squad's
first member, so a placement never takes the lead from a squad that has one. The open squad is read
from the document on every call, so it is right again after an undo.

## Boundaries

- Depends on: `crate::data::store::MissionDocCore` (`small_maps_json`, `add_faction`, `add_squad`,
  `add_slot`, `set_leader`); `serde_json`.
- Used by: `crate::data::store::operations::entity`, whose armed character release and debug seed
  place through it; `crate::data::store` re-exports it; tests in
  `apps/website/map-engine/src/editing/tests/picking_selection.rs` and
  `apps/website/map-engine/src/data/store/operations/apply_faction/tests/`.
- Rules:
  - placements in a row fill one squad and keep every body
    (`five_places_build_one_squad_and_keep_every_body` in `tests/cases_1.rs`), each side keeps its
    own current squad (`each_side_keeps_its_own_current_squad`), and an authored squad is never
    grown (`placement_starts_a_new_squad_rather_than_growing_an_authored_one`);
  - the squad name a placement mints reads as minted to the open-squad test
    (`minted_names_match_what_placement_writes`), and the open squad is found again after an undo
    (`the_current_squad_is_re_derived_after_undo`);
  - both files are on the place path that `cargo xtask verify editor-orbat-coherency` scans for
    `ensure_default_squad`, and the gate runs the `place_` tests.

## Related documentation

- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — where a placed character lands in the ORBAT.
