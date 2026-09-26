# Slot symbology surface and its tests

One flat re-export of the [slot](/documentation_v2/glossary/n_to_z.md#slot), vehicle and comment symbology
vocabulary (cell layout, colours,
sizes, packers, row patches and drag helpers), and the unit tests that exercise that vocabulary
end to end.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/instances/slots/
├── mod.rs  re-exports packers, patches, drag helpers, atlas layout, role tables
└── tests/  unit tests for instance layout, side tints, clustering, drag, atlas cells, roles and yaw
```

## How it works

`mod.rs` holds no logic: it re-exports items of `crate::overlay::symbology::instances`
(`symbols`, `patches`, `drag` and the graphics engine's `packing`), of
`crate::overlay::symbology::atlas::raster` and of `crate::overlay::symbology::roles::classify`.
The tests import that surface with `use super::*`, so one suite pins the behaviour of files in
three folders: the 20-byte instance stride, the three distinct side tints and the BLUFOR default,
the cluster gate and disc size, the drag transitions and previews, the atlas cells and their
shapes, the role and vehicle tables, and the compass yaw encoding.

## Boundaries

- Depends on: `crate::overlay::symbology::instances` (`symbols`, `patches`, `drag`, `packing`),
  `crate::overlay::symbology::atlas::raster` and `crate::overlay::symbology::roles::classify`.
- Used by: its own `tests/`; no code outside the folder imports the re-exports.
- Rules: every re-export names its source path in its doc line, and the folder adds no item of its
  own, so a behaviour change lands in the source file and the tests here follow it.
