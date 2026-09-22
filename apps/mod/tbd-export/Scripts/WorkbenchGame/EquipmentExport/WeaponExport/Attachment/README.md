# WeaponExport/Attachment

The umbrella sweep over every non-optic weapon attachment, producing the combined attachment catalog.

### Roles & Responsibilities
- `TBD_AttachmentModel.c`: `TBD_AttachmentInfo` plus one carrier per attachment family (muzzle, bayonet, illuminator, handguard, mount, stock, camouflage, bipod). These families overlap with the dedicated `Muzzle/`, `Bayonet/`, `Illuminator/`, `Handguard/`, and `Stock/` domains, which extract the same hardware in greater depth.
- `TBD_AttachmentExtractor.c`: Reads what an attachment is as an object — mass, volume, dimensions, inventory footprint, and the mesh it renders as.
- `TBD_AttachmentMountingExtractor.c`: Reads how an attachment fits a weapon — the type it presents, every type it is compatible with by walking the type's inheritance chain, the types it obstructs, and the nested slots it offers to further attachments.
- `TBD_AttachmentFamilyExtractor.c`: Decides which family an attachment belongs to, then reads the properties only that family has — suppression for a muzzle device, melee damage for a bayonet, beam colour and range for an illuminator.
- `TBD_AttachmentNaming.c`: Reads display name, description, and icon, falling back to a stem derived from the prefab filename more often than a single-family domain needs to.
- `TBD_AttachmentCustomAttributes.c`: Reads the custom attribute list under the key this domain uses, `m_aAttributes`, where Core's reader uses `CustomAttributes`.
- `TBD_AttachmentScanner.c`: Sweeps every loaded addon, buckets each attachment into its family, and writes one catalog per family plus the `attachments_all` rollup.
- `TBD_AttachmentExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Weapon Attachments`. Writes `$profile:TBD_Export/equipment/attachments/`.

### Call Flow & Contracts
Menu action -> `TBD_AttachmentExportPlugin.Run()` -> `TBD_AttachmentScanner.Scan()` -> the four extractors -> family carrier -> `WriteCategoryCatalog()` emits `muzzles.json`, `bipods.json`, `handguards.json`, `illuminators.json`, `bayonets.json`, `stocks.json`, `mounts.json`, and `camouflage.json` under `equipment/attachments/`, followed by `attachments_all.json`. Each file carries a `_meta.json` sidecar. This domain is the breadth pass; the per-hardware domains are the depth pass, and the two write to different directories so neither overwrites the other.
