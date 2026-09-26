# Equipment and vehicle source reader

Reads one [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) resource's configuration tree, as [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
has it loaded, into typed source facts: every property of every container, with its native type,
value, origin and the native method that read it. The result is the source snapshot of a resource
and the raw material of its organized record.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Source/
├── TBD_SourceContainerReader.c  walks a container tree into nodes, facts and references
├── TBD_SourceExportModel.c      the fact, node and reference types and their JSON form
└── TBD_SourcePropertyReader.c   reads one scalar or array property by its native type
```

## How it works

`TBD_SourceContainerReader.Capture` starts at a resource's root `BaseContainer` and walks it depth
first: its children, the containers its object properties hold, and its ancestor. Each container
becomes a `TBD_SourceExportNode` whose id is its structural path from the root (`root`,
`root/children/0`, `root/properties/components/3`, `…/ancestor`), in one of two views:
`effective`, the installation as the engine resolves it, and `ancestor`, the inherited container
kept beside it as evidence. A container shared by both views gets a node in each, so an ancestor
never replaces an effective installation. A node keeps the native container id (the `{…}` resource
name of an editor container) when there is one, and falls back to the structural path otherwise.

Every property the container enumerates (`GetNumVars`) becomes a `TBD_SourceExportFact`:

- the value, read by `TBD_SourcePropertyReader` for scalars, strings, vectors, colours and their
  arrays, or as node links for objects and object arrays, in order, with `null` entries and
  explicit empty arrays kept;
- the origin: `declared` when the container sets it directly, `inherited` when an ancestor does,
  `engine_default` otherwise;
- the native type, the enum names and values the engine declares, the object base class, and the
  method that read it (`BaseContainer.Get`, `GetObject` or `GetObjectArray`);
- a unit only where the engine documents one: `Weight` in kg, `ItemVolume` in cm3 and
  `ItemDimensions` in cm on `ItemPhysicalAttributes`. No value is converted.

A property the reader cannot read gets status `error` and adds an extraction error; zero, false,
empty text, empty arrays and null object references are values, never absences. Vectors keep the
native order (x/y, x/y/z) and colours r/g/b/a.

Resource-name properties and a node's own resource name become `TBD_SourceExportReference`s: kind
`gameplay` for prefabs, configs, game materials and ragdolls (`.et`, `.conf`, `.gamemat`,
`.ragdoll`), which the generation then exports as resources of their own, and kind `binary` for
models, textures, audio and every other asset, which stay external references. A reference found
through the node's resource name carries the method `BaseContainer.GetResourceName` and points at
node metadata; one read with `BaseContainer.Get` points at a property. The walk stops with an error
on a cycle, past 128 levels (`MAX_DEPTH`) or past 100,000 nodes (`MAX_NODES`). `Snapshot` writes the
nodes as the `source_snapshot` document.

Native container ids are scoped to the loaded configuration: Workbench can assign new ones to
unauthored editor containers after a script reload, so the structural path and the resource GUID,
not the container id, identify an installation across runs.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `BaseContainer`, `BaseContainerList` and `DataVarType`;
  `TBD_SourceExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Serialization/`
  and `TBD_SourceTypeHierarchy` in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Capabilities/`,
  which collects every class name the walk meets.
- Used by: the rest of `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`: the
  generation in `Generation/`, the capability builder in `Capabilities/`, the diagnostic filter in
  `Plugins/`, and the probes and reader verification in `Verification/`.
- Rules: every enumerated property is read or fails the extraction, and nothing is converted or
  dropped here; a fact keeps the property's source spelling. The reader verification in
  `Verification/` compares every fact against a direct native read on installed prefabs, and the
  validator's `explicit_zero_false_and_empty_values_are_preserved`,
  `shared_ancestor_objects_cannot_replace_effective_objects` and
  `native_enums_vectors_units_and_precision_survive_without_conversion` tests
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`) hold the
  written form.

## Related documentation

- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the
  `source_snapshot` document, facts, nodes and references.
- [Acceptance evidence](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md)
  — the recorded reader checks and field mapping.
