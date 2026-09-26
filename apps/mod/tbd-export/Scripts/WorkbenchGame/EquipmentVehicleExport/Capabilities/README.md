# Equipment and vehicle capability records

Turns a resource's source facts into its organized record: the facts grouped by capability
(inventory, weapon, magazine, sights, vehicle systems and the rest) under snake_case field names,
its English names, its references and the native types it uses. It also resolves class inheritance
for the whole exporter and writes the generation's type hierarchy. It runs in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) once per exported resource.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Capabilities/
├── TBD_SourceCapabilityBuilder.c  the `resource_record` document and the resource identity
└── TBD_SourceCapabilityRules.c    class-to-capability rules, field names and the type hierarchy
```

## How it works

`TBD_SourceCapabilityRules.IsA` answers every "is this class a kind of that one" question in the
exporter through `TypeName.IsInherited`, so capability membership and discovery follow native
inheritance and never a file name. `Capability` maps a class to one of fourteen capabilities
(`inventory`, `storage`, `weapon`, `magazine`, `projectile`, `attachment`, `sights`, `protection`,
`medical_effects`, `communications`, `vehicle_systems`, `utility`, `visuals`, `physics`), or to
none.

`TBD_SourceCapabilityBuilder.Build` walks the effective nodes a `TBD_SourceContainerReader` captured.
A node whose class maps to a capability, or which sits below a node that does, becomes one
instance of that capability: its node id and every one of its facts, unchanged, under the field
name `Field` gives the property. So repeated installations (muzzles, compartments, pockets, wheels,
effects, attachments) stay separate, ordered instances, and a capability appears only when the
configuration holds it. `Field` turns a property name into snake_case, except `AmmoTemplate`,
which becomes `default_projectile`, and the vector `Trigger Offset`, which becomes
`trigger_offset_vector3` so it cannot collide with the scalar `TriggerOffset`; two properties that
would share a field name are an error. For each `UIInfo` node the builder adds a name entry: the
source `Name` fact, with its localization key, and `display_name_en`, the text
`WidgetManager.Translate` returns while the generation holds the `en_us` locale, or `unavailable`
when nothing resolves.

A resource's identity is `guid:<resource GUID>`, or `resource:<exact resource name>` when the name
carries no GUID (`ResourceIdentity`). The record also lists the source addons, every reference and
every type name.

`TBD_SourceTypeHierarchy` collects every class name the reader meets. Before it is written into
`generation.json`, `TBD_SourceDeclaredAncestors` reads the class declarations in every loaded
addon's `Scripts/`, addon-defined classes included, to add their script-declared ancestors as
candidates; every relationship is then checked with `TypeName.IsInherited`. A native class with no
script `TypeName` is written as `unavailable` for that reflection, and its properties are still
exported.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the reader's node, fact and reference types in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Source/`; `TBD_SourceExportJson`
  in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Serialization/`; the engine's
  `TypeName`, `WidgetManager`, `GameProject` and `FileIO`.
- Used by: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the generation
  in `Generation/` (records, identities, record file names, the type hierarchy), the discovery in
  `Discovery/` and the diagnostic filter in `Plugins/` (`IsA`), and the reader in `Source/`
  (`TBD_SourceTypeHierarchy`).
- Rules: an organized fact is the same fact as in the source snapshot, never a recomputed one;
  no capability block is invented for a missing capability; the validator's
  `organized_capability_cannot_silently_omit_properties`,
  `ancestor_installations_cannot_leak_into_effective_capabilities` and
  `type_inventory_and_hierarchy_must_close` tests
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`) hold the
  written form, and the capability names are the schema's closed list.

## Related documentation

- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the
  `resource_record` document and the capability names.
- [Field mapping](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/field-mapping/README.md)
  — which source facts each organized field carries.
