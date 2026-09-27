# WeaponExport/M16

The M16 platform compatibility matrix: every M16 variant paired against every magazine, optic, and attachment that fits it.

The Workbench menu entry is currently disabled: the `[WorkbenchPluginAttribute]` block in
`TBD_M16ExportPlugin.c` is commented out, so `Export M16 Deep Analysis & Compatibility` does not
appear under `Plugins > TBD`. The scanner, its extractors, and its models compile and are ready to run once the attribute is
restored.

### Roles & Responsibilities
- `TBD_M16Model.c`: `TBD_M16WeaponVariantInfo` with its muzzle and attachment-slot carriers, plus `TBD_M16CandidateMagazine` and `TBD_M16CandidateAttachment` — the pools the scanner matches slots against.
- `TBD_M16Extractor.c`: Reads what one M16 variant intrinsically declares — its muzzles with their magazine wells and fire modes, its attachment slots, and its zeroing distances.
- `TBD_M16CandidateExtractor.c`: Reads a pool entry — the magazine well a magazine fits, its capacity, the attachment type an attachment presents, and the mass and volume of either. Owns `AttachTypeFits`, the rule the pairing is made on: an attachment fits a slot when its declared type equals the slot's required type or inherits from it.
- `TBD_M16Naming.c`: Names variants, magazines, and attachments alike, so its stem fallback carries more cases than a single-family domain needs.
- `TBD_M16DeepScanner.c`: Collects every M16 variant and, separately, every magazine and attachment in every loaded addon, then computes the compatibility pairing between them and serializes it.
- `TBD_M16ExportPlugin.c`: Workbench entry point, disabled. Writes `$profile:TBD_Export/equipment/m16_deep_export.json`.

### Call Flow & Contracts
Menu action -> `TBD_M16ExportPlugin.Run()` -> `TBD_M16DeepScanner.RunDeepScan()` builds the variant list and the candidate pools through the three readers -> matches magazine wells and attachment types across them -> `equipment/m16_deep_export.json` plus `m16_deep_export_meta.json`, written to the equipment root rather than a category subdirectory.

This is the one domain that deliberately inverts the subtree's foreign-key contract. Every other export leaves cross-product resolution to the platform; this one is a compatibility matrix, so `m_aCompatibleMagResourceNames` and `m_aCompatibleItemResourceNames` are resolved at scan time and inlined.
