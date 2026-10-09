# Paper doll scene source

The mannequin the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation/glossary/a_to_f.md#arsenal) paper doll draws and picks: its 14 clickable
equipment regions, the boxes and the tube that make up the soldier, the colours that show each
region's state, the unit meshes every part is scaled from, and the picks and callout anchors over
them. Pure data and functions, with no GPU and no browser.

## Contents

```text
crates/paper_doll/paper_doll_scene/src/
├── lib.rs             the crate root: the module tree and the test mount
├── part_meshes.rs     `mesh_cube` and `mesh_cylinder`: unit meshes with interleaved position and normal
├── prelude.rs         the names a renderer or host imports
├── region_picking.rs  `pick`: the region under a pixel; `anchor_px` and `anchor_world`: a region's callout point
├── soldier_parts.rs   `REGION_KEYS`, the region states, `instances()`, the state and decor colours
└── tests/             the regions, parts, meshes, colours, picks and anchors against their goldens
```

## How it works

`REGION_KEYS` names the 14 clickable regions in rail order: `primary`, `optic`, `magazine`,
`launcher`, `handgun`, `throwable`, `headCover`, `jacket`, `vest`, `armoredVest`, `backpack`,
`handwear`, `pants`, `boots`. A region's index in that array is its number everywhere: in each
`DollInstance.region`, in the `REGION_COUNT` state bytes the renderer takes, and in the index a
pick returns. `DECOR` (-1) marks body parts that are drawn but never picked.

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

```text
pick(yaw, w, h, x, y)
   │ inverse of camera_math::orbit::projection::view_proj_gl(yaw, w, h)
   ▼
ray through the pixel's near and far points
   │ into each clickable part's model space (inverse model matrix)
   ▼
slab test against the unit box ──▶ nearest hit's region, or -1
```

`anchor_world(region)` is the translation of the region's first part, and `anchor_px` projects it
with the same matrix to pixels of the view, or `None` when the region is unknown or the point sits
behind the camera.

## Public surface

- `soldier_parts`: `REGION_COUNT`, `REGION_KEYS`, `STATE_EMPTY`, `STATE_EQUIPPED`,
  `STATE_ACTIVE`, `DECOR`, `CLEAR_COLOR`, `DollInstance`, `MeshKind`, `instances`, `state_color`
  and `decor_color`.
- `part_meshes`: `mesh_cube` and `mesh_cylinder`.
- `region_picking`: `pick`, `anchor_px` and `anchor_world`.
- `prelude`: all of the above.

## Boundaries

- Depends on: `camera_math::matrix4` (`identity`, `multiply`, `translate_in_place`,
  `scale_in_place`, `invert`, `transform_vector`) and `camera_math::orbit::projection`.
- Used by: `paper_doll_renderer` (`crates/paper_doll/paper_doll_renderer/`), which packs the
  parts, uploads the meshes and calls the picks and anchors. The Arsenal in
  `crates/frontend/workspaces/mission_creator_arsenal/src/doll.rs` reaches the scene only through that
  renderer, and sends its region states in the order of its own `RAIL_REGIONS`
  (`crates/frontend/workspaces/mission_creator_state/src/arsenal_rules/paper_doll_and_weight.rs`).
- Rules:
  - `REGION_KEYS` and the frontend's `RAIL_REGIONS` list the same 14 keys in the same order,
    because the state bytes and the pick index are positional; no test compares the two lists
    across the crates;
  - there are 14 unique keys and every region has at least one part
    (`every_region_has_at_least_one_instance` in `tests/soldier_model_tests.rs`); the three state
    colours differ and each hover lifts its colour.

## Related documentation

- [Arsenal](/crates/frontend/workspaces/mission_creator_arsenal/src/README.md) — the workspace whose
  rail and loadout rows the regions stand for.
- [Orbit camera](/crates/geometry/camera_math/src/orbit/README.md) — the fixed-orbit camera the
  picks and anchors project with.
