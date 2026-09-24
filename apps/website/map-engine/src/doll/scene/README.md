# Doll scene

The mannequin the [arsenal](/documentation_v2/glossary.md#arsenal)'s doll preview draws and picks:
its 14 clickable equipment regions, the boxes and the tube that make up the soldier, the colours
that show each region's state, and the unit meshes every part is scaled from. Pure data and
functions, with no GPU and no browser.

## Contents

```text
apps/website/map-engine/src/doll/scene/
├── instances.rs  `REGION_KEYS`, the region states, `instances()`, the state and decor colours
├── mesh.rs       `mesh_cube` and `mesh_cylinder`: unit meshes with interleaved position and normal
├── mod.rs        the module tree
└── model/        a flat re-export of the scene, picking and orbit projection items, and their tests
```

## How it works

`REGION_KEYS` names the 14 clickable regions in rail order: `primary`, `optic`, `magazine`,
`launcher`, `handgun`, `throwable`, `headCover`, `jacket`, `vest`, `armoredVest`, `backpack`,
`handwear`, `pants`, `boots`. A region's index in that array is its number everywhere: in each
`DollInstance.region`, in the 14 state bytes the renderer takes, and in the index a pick returns.
`DECOR` (-1) marks body parts that are drawn but never picked.

`instances()` builds the soldier on a schematic 1.8 m frame: 22 parts, two of them decor (head and
neck), the rest spread over the 14 regions, with the jacket, pants, handwear and boots split into
several boxes. Each part carries a column-major `f64` model matrix that scales a unit mesh; the
launcher is the one cylinder, slung diagonally on the back. The rifle hangs across the chest, and
its optic and magazine are boxes of their own, composed onto the rifle's matrix, so each takes a
separate pick.

`state_color(state, hovered)` maps `STATE_EMPTY` (0), `STATE_EQUIPPED` (1) and `STATE_ACTIVE` (2)
to three opaque colours and lifts each channel by 1.22, capped at 1, on hover. `decor_color()` is
darker than every state, and `CLEAR_COLOR` is the background.

`mesh_cube()` gives 24 vertices and 36 indices of a cube of extent ±0.5 with per-face normals;
`mesh_cylinder(segments)` gives a unit cylinder along y (radius 0.5, height 1) of at least three
segments, with radial side normals and two capped fans. Both interleave six `f32` a vertex and
index with `u16`.

## Public surface

- `instances`: `REGION_KEYS`, `STATE_EMPTY`, `STATE_EQUIPPED`, `STATE_ACTIVE`, `DECOR`,
  `CLEAR_COLOR`, `DollInstance`, `MeshKind`, `instances`, `state_color` and `decor_color`, for
  `crate::doll::renderer`, `crate::doll::interaction` and the readback check in
  `crate::diagnostics::readback::doll`.
- `mesh`: `mesh_cube` and `mesh_cylinder`, for `crate::doll::renderer`, which uploads the cube and a
  16-segment cylinder.

## Boundaries

- Depends on: `crate::camera::math::glmat4` (`identity`, `multiply`, `translate_in_place`,
  `scale_in_place`) for the part matrices; `model/` also re-exports from
  `crate::doll::interaction::picking` and `crate::camera::orbit::projection`.
- Used by: `crate::doll::renderer`, `crate::doll::interaction` and
  `crate::diagnostics::readback::doll`. The arsenal preview in
  `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs` reaches the scene only through
  `DollEngine`, and sends its region states in the order of its own `RAIL_REGIONS`
  (`apps/website/frontend/src/v2/apps/editor/arsenal/rules/paper_doll_and_weight.rs`).
- Rules:
  - `REGION_KEYS` and the frontend's `RAIL_REGIONS` list the same 14 keys in the same order,
    because the state bytes and the pick index are positional; no test compares the two lists
    across the crates;
  - there are 14 unique keys and every region has at least one part
    (`region_keys_count_and_uniqueness` and `every_region_has_at_least_one_instance` in
    `model/tests/cases_1.rs`); the three state colours differ and each hover lifts its colour
    (`state_colors_distinct_and_hover_lifts`, same file).

## Related documentation

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md) — the workspace whose
  rail and loadout rows the regions stand for.
