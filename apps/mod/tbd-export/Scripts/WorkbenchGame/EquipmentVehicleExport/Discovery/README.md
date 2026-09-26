# Equipment and vehicle discovery

Finds the prefabs a complete equipment and vehicle export starts from: every loaded addon's
`Prefabs` tree is searched, and each prefab is classed as equipment, a vehicle or neither by the
native classes of its components. It runs in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
at the start of a generation and of a diagnostic action.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Discovery/
└── TBD_SourceResourceDiscovery.c  the prefab search and the equipment and vehicle census
```

## How it works

`Scan` runs `Workbench.SearchResources` for `.et` files under `$<addon id>:Prefabs` of every addon
`GameProject.GetLoadedAddons` returns, so the census is a function of the loaded addons. Each prefab
outside the deny list (structures, rocks, trees, debris, foliage, and the editor, system,
waypoint, trigger, composition, sound and UI prefab folders) is loaded, and the classes of its root,
components, additional actions and custom attributes are collected. Classes are compared by
inheritance through `TBD_SourceCapabilityRules.IsA`:

1. A character (`ChimeraCharacter` or a `CharacterControllerComponent`) is neither.
2. A vehicle (a `Vehicle` root, a vehicle simulation or a vehicle controller) with a compartment
   manager is a vehicle, unless its file is a destroyed or wreck variant (`_dst.et`, `_wreck_`,
   `_Wreck`); a vehicle without one is neither.
3. A non-inventory prefab with compartments that is a vehicle part (`/VehParts/`,
   `Prefabs/Vehicles/`) or not a static weapon is neither.
4. Equipment is anything with an inventory item, weapon, magazine, loadout cloth, gadget,
   binoculars, attachment or turret component, a fast-travel action, a static weapon (a `Turret`
   root, `/Tripods/`, `/Mortars/`) or a path under `/Ammo/`.

`m_aEquipment` and `m_aVehicles` come back sorted; a failed search, a prefab that does not load or
an empty equipment or vehicle list adds an error. `Inspect` classes an explicit resource list the
same way, for the `discovery` probe. Discovery only selects: every fact is read later by the
source reader.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `Workbench.SearchResources`, `GameProject` and `Resource`; `TBD_SourceCapabilityRules`
  in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Capabilities/`.
- Used by: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the complete
  generation in `Generation/`, `TBD_SourceDiagnosticExport` in `Plugins/`, and the `discovery` action
  of `EMCP_WB_SourceExport` in `Verification/`.
- Rules: a class counts by native inheritance, never by name alone; the census written into
  `generation.json` must be exported in full, which the validator's
  `missing_discovery_and_dependency_resources_fail` test
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`) holds.
