# Faction library apply

Writes a faction library template, a flat list of roles and a vehicle pool, onto one side of the
[mission](/documentation_v2/glossary/g_to_m.md#mission) document as that side's
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat): one faction, one squad, a
[slot](/documentation_v2/glossary/n_to_z.md#slot) per role and the vehicles attached to the squad. It
refuses, before writing anything, when the side holds squads the author built by hand.

## Contents

```text
apps/website/map-engine/src/data/store/operations/apply_faction/
├── apply.rs       `apply_faction_library`: the refusal checks, the fold into one squad, the writes
├── authorship.rs  which squads read as hand-authored, the squad, slot and vehicle ids, new ids
├── library.rs     the library input types, the result, the refusal error and the terrain anchors
├── mod.rs         the module tree; re-exports the input, result and error types, anchors and apply
└── tests/         unit tests for the apply, the refusal, the fold and the anchors
```

## How it works

`apply_faction_library(doc, side, layer_id, lib)` runs in this order, reading the document through
`small_maps_json` and `slots_json` and writing through the `MissionDocCore` mutators:

1. A side other than `BLUFOR`, `OPFOR` or `INDFOR` is refused with `InvalidSide`.
2. The side's faction is `faction-<side>`. Every squad after its first that reads as authored (more
   than one slot, a name other than empty or a minted `Squad <n>`, a callsign, or a vehicle) blocks
   the apply: `WouldCollapseSquads` names each one and the slots at risk, and nothing is written.
3. The faction is created or renamed after the library, and the anchor is the centre of the
   document's terrain: 2048, 2048 on `arland`, and `APPLY_ANCHOR_X`, `APPLY_ANCHOR_Y` (6400, 6400)
   on every other terrain.
4. The side's first squad takes the library name (`Squad 1` when blank), or a new `squad-<side>-<n>`
   is added; every other squad's slots move into it, and a squad left empty is removed.
5. Role `i` overwrites the squad's slot `i` (role, tag, character, loadout), which keeps its id and
   map position; a role past the existing slots adds `slot-<side>-apply-<i>` in `layer_id`, at the
   anchor moved 15 m along x per role index; slots past the last role are removed.
6. The leader is the first role whose name contains "squad leader", in any case, else the first.
7. The squad's vehicles are removed, and each named library vehicle is added as
   `veh-<side>-apply-<j>`, offset from the anchor and 20 m apart, and attached to the squad; a
   vehicle's `label` is not written.

A minted id that is taken gains a `-2`, `-3` suffix. The result names the faction, the squad and
the leader slot, and counts the roles and vehicles applied.

## Boundaries

- Depends on: `crate::data::store::MissionDocCore` (its JSON views and its faction, squad, slot,
  loadout, leader and vehicle mutators); `serde_json`.
- Used by: `crate::data::store::operations::entity`, whose `orbat_apply_faction` maps a faction
  document onto `FactionLibraryInput` and runs the apply for the hosted command in
  `apps/website/map-engine/src/editing/hosted_commands/orbat_roster.rs`, and whose placement and
  roster code falls back on the anchors; `crate::data::store` re-exports the surface.
- Rules:
  - a refusal writes nothing (`apply_refuses_to_collapse_squads_and_writes_nothing` in
    `tests/cases_1.rs`), while squads a placement minted and nobody edited fold in
    (`two_placements_then_apply_folds_them_in`,
    `an_empty_minted_squad_folds_away_but_a_renamed_one_blocks`);
  - an apply replaces the side's roster rather than merging into it
    (`apply_faction_replace_not_merge`), and a re-apply keeps the ids and positions of the slots it
    overwrites (`reapply_keeps_overlapping_slot_ids_and_positions`);
  - the anchor is the centre of `crate::data::scenario::compile::terrain_bounds` for every terrain
    (`apply_anchor_matches_terrain_bounds`);
  - every source file here is on the place path that `cargo xtask verify editor-orbat-coherency`
    scans, which bans `ensure_default_squad` there, fails when a listed file is missing and runs
    the `apply_faction_` tests.
