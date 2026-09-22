# Weapon Export Hub (`Scripts/WorkbenchGame/EquipmentExport/WeaponExport`)

Twelve Workbench extractors, one per weapon domain, that read the loaded addon set and write the
weapon half of the TBD arsenal catalog to `$profile:TBD_Export/equipment/`.

---

## Architecture Overview

Every domain answers the same question about a different piece of hardware: what does this prefab
genuinely declare? Extraction is ground truth only — omitted values serialize as JSON null, engine
localization tokens are preserved verbatim including the leading `#`, and no field is synthesized
to fill a gap. Nothing here runs at game time; these are Workbench plugins.

The domains do not know about each other. Cross-domain relationships are exported as foreign keys
(a rifle names its magazine wells, a handguard names its required attachment type) and resolved by
the platform against the sibling catalogs. `M16/` is the single deliberate exception and says so in
its own README.

```text
WeaponExport/
├── Ammo/                  <-- Magazines, ammo configs, and projectiles
├── Attachment/            <-- Breadth sweep over all non-optic attachments
├── Bayonet/               <-- Bayonets and mounted blades
├── Handguard/             <-- Handguards, rails, foregrips
├── Illuminator/           <-- Weapon lights, IR illuminators, laser pointers
├── M16/                   <-- M16 platform compatibility matrix (plugin disabled)
├── Muzzle/                <-- Suppressors, flash hiders, muzzle brakes
├── Optic/                 <-- Sights and scopes
├── Rifle/                 <-- Rifle-only deep intrinsic pass (plugin disabled)
├── Stock/                 <-- Buttstocks
├── Underbarrel/           <-- Underbarrel launchers and accessories
└── Weapon/                <-- Master arsenal sweep, bucketed by weapon class
```

---

## Subdirectories

| Subdirectory | Responsibility | Key Classes |
|---|---|---|
| **`Ammo/`** | Magazines and the projectiles they chamber, split by caliber. | `TBD_MagazineInfo`, `TBD_ProjectileInfo`, `TBD_AmmoScanner` |
| **`Attachment/`** | One pass over every non-optic attachment, producing per-family catalogs and the combined rollup. | `TBD_AttachmentInfo`, `TBD_AttachmentScanner` |
| **`Bayonet/`** | Bayonets and mounted blades, with melee combat properties. | `TBD_BayonetInfo`, `TBD_BayonetScanner` |
| **`Handguard/`** | Handguards, rail systems, and foregrips, with handling modifiers. | `TBD_HandguardInfo`, `TBD_HandguardScanner` |
| **`Illuminator/`** | Weapon lights, IR illuminators, and laser pointers, with lens emission properties. | `TBD_IlluminatorInfo`, `TBD_IlluminatorScanner` |
| **`M16/`** | Resolved compatibility matrix for the M16 platform. Plugin attribute commented out. | `TBD_M16WeaponVariantInfo`, `TBD_M16DeepScanner` |
| **`Muzzle/`** | Suppressors, flash hiders, and brakes, with their acoustic and ballistic modifiers. | `TBD_MuzzleInfo`, `TBD_MuzzleScanner` |
| **`Optic/`** | Sights and scopes, with magnification, field of view, and eye relief. | `TBD_OpticInfo`, `TBD_OpticScanner` |
| **`Rifle/`** | Rifle-only intrinsic pass, deeper than the master arsenal sweep. Plugin attribute commented out. | `TBD_RifleWeaponInfo`, `TBD_RifleScanner` |
| **`Stock/`** | Buttstocks. Structurally parallel to `Handguard/`. | `TBD_StockInfo`, `TBD_StockScanner` |
| **`Underbarrel/`** | Underbarrel launchers and accessories, including secondary muzzles. | `TBD_UnderbarrelInfo`, `TBD_UnderbarrelScanner` |
| **`Weapon/`** | Every weapon in every addon, bucketed into seven categories. | `TBD_WeaponInfo`, `TBD_WeaponScanner` |

---

## Technical Contracts

1. **The four-file shape:**
   A domain is `TBD_<Domain>Model.c` (data carriers, no behaviour), `TBD_<Domain>Extractor.c` (reads
   one prefab, fills one carrier), `TBD_<Domain>Scanner.c` (sweeps the addon set, buckets, and
   serializes), and `TBD_<Domain>ExportPlugin.c` (the Workbench entry point). `Rifle/` and `M16/`
   carry no extractor: their per-prefab walk lives inside the scanner.

2. **One-way dependency:**
   `ExportPlugin -> Scanner -> Extractor -> Model -> Core`. No domain imports another domain's
   classes, and no carrier reaches back to a scanner. The edge is a straight line in every folder.

3. **Shared infrastructure lives in `EquipmentExport/Core/`:**
   The export destination and JSON writing (`TBD_EquipmentExportConfig`, `TBD_EquipmentExportPaths`,
   `TBD_EquipmentExportJson`), the prefab component and ancestor walk
   (`TBD_EquipmentComponentGraph`), and the readers for a prefab's names and resource references
   (`TBD_EquipmentDisplayAttributes`, `TBD_EquipmentResourceNames`) all sit there. EnfScript
   resolves classes by global name, so a domain reaches them without any path reference, and they
   are never duplicated per domain.

   A domain declares its own copy only where its behaviour genuinely differs, under a name that
   says whose it is: the `Stock/` domain searches wider for a display name than the other six that
   share the Core reader, and the `Attachment/` domain reads its custom attributes under a
   different key. Two further helpers stay with their caller because they are per-scan policy, not
   shared behaviour: each scanner declares the `COMPONENT_DEPTH_CAP` it passes to the shared walk,
   and each domain picks which of the two resource-name resolvers its fields go through.

4. **Output root:**
   Everything lands under `$profile:TBD_Export/equipment/`, one subdirectory per domain
   (`ammunition/`, `attachments/`, `bayonets/`, `handguards/`, `illuminators/`, `muzzles/`,
   `optics/`, `stocks/`, `underbarrel/`, `weapons/`). Every data file is paired with a
   `_meta.json` sidecar carrying the row count and the UTC generation timestamp. The two disabled
   plugins write loose files at the equipment root instead: `rifles.json` and `m16_deep_export.json`.

5. **Workbench entry points:**
   Every plugin is `class TBD_<X>ExportPlugin : WorkbenchPlugin` carrying
   `[WorkbenchPluginAttribute(name: "Export …", category: "TBD")]`, which places it under
   `Plugins > TBD`. New `.c` files require a Workbench cold restart — the script list is built at
   load. `cargo xtask mod compile` does not cover this tree; `Scripts/WorkbenchGame` compiles only
   inside Workbench.
