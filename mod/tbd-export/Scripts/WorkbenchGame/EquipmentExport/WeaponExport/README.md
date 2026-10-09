# Weapon exports

Twelve [Workbench](/documentation/glossary/n_to_z.md#workbench) extractors, one per weapon
domain, that read the loaded addon set and write the weapon half of the equipment catalogs under
`$profile:TBD_Export/equipment/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/
├── Ammo/         magazines, ammunition configs, projectiles and ballistic tables
├── Attachment/   the breadth sweep over every non-optic attachment, by family
├── Bayonet/      bayonets and mounted blades with their melee properties
├── Handguard/    handguards, rails and foregrips with their handling modifiers
├── Illuminator/  weapon lights, infrared illuminators and laser pointers
├── M16/          the M16 platform compatibility matrix
├── Muzzle/       suppressors, flash hiders and muzzle brakes
├── Optic/        sights and scopes with magnification, field of view and eye relief
├── Rifle/        a rifle-only pass deeper than the master weapon sweep
├── Stock/        buttstocks with their handling modifiers
├── Underbarrel/  underbarrel launchers and accessories, including secondary muzzles
└── Weapon/       every weapon in every addon, in nine categories
```

## How it works

Every domain answers the same question about a different piece of hardware: what does this prefab
declare? A value the prefab omits serializes as JSON null, an engine localization token keeps its
leading `#`, and no field is made up to fill a gap.

A domain is `TBD_<Domain>Model.c` (data carriers, no behaviour), one or more extractors (each reads
part of one prefab and fills part of one carrier), `TBD_<Domain>Scanner.c` (sweeps the addon set,
buckets and serializes through `RunScan()`) and `TBD_<Domain>ExportPlugin.c`. The number of
extractors follows the carrier: a domain whose extraction fits one file has
`TBD_<Domain>Extractor.c`; otherwise each extractor is named for the part it fills
(`TBD_WeaponMuzzleExtractor`, `TBD_AttachmentMountingExtractor`). `Ammo/` splits by object instead,
since a magazine and a projectile have different carriers.

"Export All Equipment" (`TBD_EquipmentExportPlugin` in
`mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`) runs the `Weapon/`, `Attachment/`,
`Optic/` and `Ammo/` scanners. Every domain plugin's `[WorkbenchPluginAttribute]` is commented
out, so the other eight domains run only when their attribute is restored.

The domains share readers only in one direction: `Weapon/` and `Optic/` read mounting through
`TBD_AttachmentMountingExtractor` from `Attachment/`, and `Weapon/` reads sights through
`TBD_OpticSightsExtractor` from `Optic/`. Cross-domain relationships are exported as foreign keys
(a rifle names its magazine wells, a handguard names the attachment type its rail requires), and
the reader resolves them against the sibling catalogs. `M16/` is the one domain that resolves
compatibility at scan time and says so in its README. Every domain writes one subdirectory
(`ammunition/`, `attachments/`, `bayonets/`, `handguards/`, `illuminators/`, `muzzles/`, `optics/`,
`stocks/`, `underbarrel/`, `weapons/`), each data file paired with a `_meta.json` sidecar; `Rifle/`
and `M16/` write `rifles.json` and `m16_deep_export.json` at the equipment root instead.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (destination, JSON
  writing, component walk, display attributes, resource names) and `TBD_ItemInventoryExtractor` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`.
- Used by: `TBD_EquipmentExportPlugin`; the static weapon extractor in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/StaticExport/`, which reads muzzles and
  attachment slots through the `Weapon/` extractors; the verification handler in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/ExportDestination/`.
- Rules: dependencies run plugin to scanner to extractor to model to the shared core in every folder, and
  a domain uses another domain's classes only for the shared readers above; a domain declares its
  own copy of a shared core reader only where its behaviour differs, under a name that says whose it
  is (`TBD_StockNaming`, `TBD_AttachmentCustomAttributes`); each scanner declares the `COMPONENT_DEPTH_CAP` it passes to
  the shared component walk.
