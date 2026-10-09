# Shared equipment export infrastructure

Everything more than one equipment domain uses, grouped by the concern it serves: where output
goes and how it is written, how a prefab is flattened into its components, and how a prefab's
player-facing strings and resource references are read.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/
├── ComponentGraph/     flattening a prefab into the components it declares
├── ExportDestination/  the destination, JSON writing, native value reads and the verification handler
└── PrefabNaming/       reading a prefab's names, icons and resource references
```

## How it works

A domain folder under `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` answers one
question about one class of hardware. The parts that are not domain-specific live here, so each has
exactly one definition, and a domain reaches it by global class name with no path reference. The
dependency runs one way, scanner to extractor to this folder, with two exceptions: the display reader
in `PrefabNaming/` reads item attributes through `TBD_ItemInventoryExtractor` from the item export, and
the verification handler in `ExportDestination/` runs the domain scanners.

Two rules keep that edge clean:

- The nesting depth of a component walk belongs to the caller. A per-hardware scan stops at 4; the
  discovery sweep and the M16 compatibility scan need 8 to reach components inside nested slots.
  `TBD_EquipmentComponentGraph.CollectComponentChain` therefore takes the cap as an argument, and
  each scanner declares its own `COMPONENT_DEPTH_CAP`; `ANCESTOR_CAP` is the same for every caller
  and stays in `ComponentGraph/`.
- `ResolveCanonicalResourceName` and `NormalizePathSeparators` in `PrefabNaming/` are different
  functions. The first loads the resource to recover its `{GUID}path`; the second only squares up
  separators and returns the reference as written. Which one a field goes through decides what
  the catalog holds.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `BaseContainer` reflection, `Resource`, `JsonSaveContext`, `FileIO` and
  `FileHandle`; Workbench's `NetApiHandler`.
- Used by: every domain folder of `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and
  the vehicle extractors in `mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`, which share
  the component walk and the JSON writer.
- Rules: no class here names a domain class beyond the two exceptions above; the scripts compile
  only when Workbench loads `tbd-export`.
