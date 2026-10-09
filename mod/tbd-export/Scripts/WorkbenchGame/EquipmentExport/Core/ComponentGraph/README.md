# Prefab component graph

The walk that flattens a prefab into the components it declares, following nested component arrays
and the ancestor chain, and answers membership questions about the result.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/ComponentGraph/
└── TBD_EquipmentComponentGraph.c  component collection, native type tests, installation ownership
```

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `BaseContainer` reflection and native type relationships.
- Used by: every scanner and extractor of
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and the vehicle extractors in
  `mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`.
- Rules: `CollectComponentChain` takes its nesting depth from the caller, and only `ANCESTOR_CAP`
  (16) is fixed here; a type test uses the engine's type relationship, never a class-name
  fragment; ancestors are walked for configuration and never count as installations, so a child
  entity's components cannot change the catalog membership of the entity that owns it.
