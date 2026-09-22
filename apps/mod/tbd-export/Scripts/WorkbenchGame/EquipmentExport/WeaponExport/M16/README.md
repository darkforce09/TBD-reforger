# WeaponExport/M16

The M16 platform compatibility matrix: every M16 variant paired against every magazine, optic, and attachment that fits it.

The Workbench menu entry is currently disabled: the `[WorkbenchPluginAttribute]` block in
`TBD_M16ExportPlugin.c` is commented out, so `Export M16 Deep Analysis & Compatibility` does not
appear under `Plugins > TBD`. The scanner and its models compile and are ready to run once the
attribute is restored.

### Roles & Responsibilities
- `TBD_M16Model.c`: `TBD_M16WeaponVariantInfo` with its muzzle and attachment-slot carriers, plus `TBD_M16CandidateMagazine` and `TBD_M16CandidateAttachment` — the pools the scanner matches slots against.
- `TBD_M16DeepScanner.c`: Collects every M16 variant and, separately, every magazine and attachment in every loaded addon, then computes the compatibility pairing between them. This domain has no separate extractor; the per-prefab walk lives inside the scanner.
- `TBD_M16ExportPlugin.c`: Workbench entry point, disabled. Writes `$profile:TBD_Export/equipment/m16_deep_export.json`.

### Call Flow & Contracts
Menu action -> `TBD_M16ExportPlugin.Run()` -> `TBD_M16DeepScanner.Scan()` builds the variant list and the candidate pools -> matches magazine wells and attachment types across them -> `equipment/m16_deep_export.json` plus `m16_deep_export_meta.json`, written to the equipment root rather than a category subdirectory.

This is the one domain that deliberately inverts the subtree's foreign-key contract. Every other export leaves cross-product resolution to the platform; this one is a compatibility matrix, so `m_aCompatibleMagResourceNames` and `m_aCompatibleItemResourceNames` are resolved at scan time and inlined.
