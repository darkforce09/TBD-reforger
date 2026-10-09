# Weapon attachment export

The breadth sweep over every non-optic weapon attachment, producing one catalog per attachment
family and the combined attachment catalog under `$profile:TBD_Export/equipment/attachments/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Attachment/
├── TBD_AttachmentCustomAttributes.c   the custom attribute list under this domain's key, `m_aAttributes`
├── TBD_AttachmentExportPlugin.c       the attachment plugin class; its menu attribute is commented out
├── TBD_AttachmentExtractor.c          mass, volume, dimensions, inventory footprint and mesh
├── TBD_AttachmentFamilyExtractor.c    the family decision and the properties only that family has
├── TBD_AttachmentModel.c              `TBD_AttachmentInfo` and one carrier per attachment family
├── TBD_AttachmentMountingExtractor.c  attachment type, compatible and obstructed types, nested slots
├── TBD_AttachmentNaming.c             display name, description and icon, with a filename stem fallback
└── TBD_AttachmentScanner.c            the addon sweep, the family buckets and the catalogs
```

## How it works

"Export All Equipment" runs `TBD_AttachmentScanner.RunScan()` in its fifth phase; the domain's own
plugin attribute is commented out.

The scanner has the four extractors read each attachment into a family carrier (muzzle, bipod,
handguard, illuminator, bayonet, stock, mount or camouflage) and writes `muzzles.json`,
`bipods.json`, `handguards.json`, `illuminators.json`, `bayonets.json`, `stocks.json`, `mounts.json`
and `camouflage.json`, then `attachments_all.json`, each with its `_meta.json` sidecar. The mounting
extractor finds every compatible type by walking the attachment type's inheritance chain.

This domain is the breadth pass. The muzzle, bayonet, illuminator, handguard and stock domains
beside this one extract the same hardware in depth and write their own directories, so neither pass
overwrites the other.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names); `TBD_ItemInventoryExtractor` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`.
- Used by: `TBD_EquipmentExportPlugin` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`;
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Weapon/` and
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Optic/` read mounting
  through `TBD_AttachmentMountingExtractor`, and the weapon domain uses `TBD_AttachmentSlotInfo`.
- Rules: custom attributes are read under `m_aAttributes` here, where the shared reader uses
  `CustomAttributes`, so the domain keeps its own reader.
