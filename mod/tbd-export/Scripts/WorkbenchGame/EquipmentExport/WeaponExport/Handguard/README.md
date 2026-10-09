# Handguard export

Handguards, rail systems and foregrips with their mounting, rail slots and handling modifiers,
written under `$profile:TBD_Export/equipment/handguards/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Handguard/
├── TBD_HandguardExportPlugin.c       the handguard plugin class; its menu attribute is commented out
├── TBD_HandguardExtractor.c          recoil and sway modifiers, mass and volume
├── TBD_HandguardModel.c              `TBD_HandguardInfo` with mounting, rail-slot, handling, physical and visual parts
├── TBD_HandguardMountingExtractor.c  attachment type, compatible and obstructed types, the rail slots offered
└── TBD_HandguardScanner.c            the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_HandguardScanner.RunScan()` has both extractors fill one `TBD_HandguardInfo` per prefab and
writes `handguards.json` and `handguards_meta.json`. Rail slots are exported as foreign keys
(`required_attachment_type`); what fits them is resolved against the attachment catalogs.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: a rail slot names the attachment type it requires and never an inlined list of what fits.
