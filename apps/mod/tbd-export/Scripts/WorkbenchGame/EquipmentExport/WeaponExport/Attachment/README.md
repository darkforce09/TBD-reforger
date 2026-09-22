# WeaponExport/Attachment

The umbrella sweep over every non-optic weapon attachment, producing the combined attachment catalog.

### Roles & Responsibilities
- `TBD_AttachmentModel.c`: `TBD_AttachmentInfo` plus one carrier per attachment family (muzzle, bayonet, illuminator, handguard, mount, stock, camouflage, bipod). These families overlap with the dedicated `Muzzle/`, `Bayonet/`, `Illuminator/`, `Handguard/`, and `Stock/` domains, which extract the same hardware in greater depth.
- `TBD_AttachmentExtractor.c`: Reads one attachment prefab and fills the matching family carrier from its mounting, physical, and visual components.
- `TBD_AttachmentScanner.c`: Sweeps every loaded addon, buckets each attachment into its family, and writes one catalog per family plus the `attachments_all` rollup.
- `TBD_AttachmentExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Weapon Attachments`. Writes `$profile:TBD_Export/equipment/attachments/`.

### Call Flow & Contracts
Menu action -> `TBD_AttachmentExportPlugin.Run()` -> `TBD_AttachmentScanner.Scan()` -> `TBD_AttachmentExtractor` -> family carrier -> `WriteCategoryCatalog()` emits `muzzles.json`, `bipods.json`, `handguards.json`, `illuminators.json`, `bayonets.json`, `stocks.json`, `mounts.json`, and `camouflage.json` under `equipment/attachments/`, followed by `attachments_all.json`. Each file carries a `_meta.json` sidecar. This domain is the breadth pass; the per-hardware domains are the depth pass, and the two write to different directories so neither overwrites the other.
