# M16 compatibility export

The M16 platform compatibility matrix: every M16 variant paired with every magazine, optic and
attachment that fits it, written to `$profile:TBD_Export/equipment/m16_deep_export.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/M16/
├── TBD_M16CandidateExtractor.c  magazine wells, capacities and attachment types of the candidate pool
├── TBD_M16DeepScanner.c         collects variants and candidates, pairs them and writes the matrix
├── TBD_M16ExportPlugin.c        the M16 plugin class; its menu attribute is commented out
├── TBD_M16Extractor.c           one variant's muzzles, fire modes, attachment slots and zeroing
├── TBD_M16Model.c               `TBD_M16WeaponVariantInfo` and the candidate magazine and attachment carriers
└── TBD_M16Naming.c              names for variants, magazines and attachments alike
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_M16DeepScanner.RunDeepScan()` builds the variant list and, separately, the magazine and
attachment pools from every loaded addon, then matches magazine wells and attachment types across
them. `AttachTypeFits` in `TBD_M16CandidateExtractor.c` is the pairing rule: an attachment fits a
slot when its declared type equals the slot's required type or inherits from it. The result goes to
`m16_deep_export.json` and its `_meta.json` sidecar at the equipment root.

This domain deliberately resolves compatibility at scan time: `m_aCompatibleMagResourceNames` and
`m_aCompatibleItemResourceNames` are inlined, where every other domain exports foreign keys.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: the component walk runs to depth 8 to reach components in nested slots; the pairing follows
  `AttachTypeFits` alone.
