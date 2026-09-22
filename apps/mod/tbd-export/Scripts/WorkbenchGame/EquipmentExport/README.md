# Equipment Export Hub (`Scripts/WorkbenchGame/EquipmentExport`)

Workbench extractors that read the loaded Reforger addon set and write the TBD arsenal catalog to
`$profile:TBD_Export/equipment/`, together with the infrastructure every extractor shares.

---

## Architecture Overview

This directory holds three things: the shared infrastructure under `Core/`, the unfiltered equipment
discovery pass, and the `WeaponExport/` subtree where the finished per-domain extractors live.

The discovery pass is the broad, shallow sweep — it walks every addon, rejects the world-building
prefab trees outright, and classifies what remains by prefix (`Vest_`, `Uniform_`, `Helmet_`) to
produce a census. The weapon subtree is the narrow, deep counterpart: twelve domains that each read
one class of hardware down to its declared components. Gear extraction beyond the census — vests,
helmets, uniforms, gadgets — is not built yet and will sit alongside `WeaponExport/` when it is.

```text
EquipmentExport/
├── Core/                          <-- Shared infrastructure, grouped by concern
├── TBD_EquipmentScanItem.c        <-- Discovery: one discovered prefab record
├── TBD_EquipmentScanner.c         <-- Discovery: unfiltered sweep and prefix classification
├── TBD_EquipmentExportPlugin.c    <-- Discovery: Workbench entry point (attribute commented out)
└── WeaponExport/                  <-- Twelve per-domain weapon extractors
```

---

## Subdirectories

| Subdirectory | Responsibility | Key Classes |
|---|---|---|
| **`Core/`** | Everything more than one domain uses: the export destination, the prefab component-graph walk, and the readers for a prefab's names and resource references. See its own [README](Core/README.md). | `TBD_EquipmentExportPaths`, `TBD_EquipmentExportJson`, `TBD_EquipmentComponentGraph`, `TBD_EquipmentDisplayAttributes`, `TBD_EquipmentResourceNames` |
| **`WeaponExport/`** | Twelve deep extractors, one per weapon domain: ammunition, attachments, bayonets, handguards, illuminators, M16, muzzles, optics, rifles, stocks, underbarrel devices, and the master arsenal. See its own [README](WeaponExport/README.md). | `TBD_WeaponScanner`, `TBD_AmmoScanner`, `TBD_OpticScanner`, … |

---

## Technical Contracts

1. **Shared infrastructure is declared in `Core/` and nowhere else:**
   `Core/ExportDestination/` owns the destination directory, path resolution, and JSON writing;
   `Core/ComponentGraph/` owns the prefab component and ancestor walk; `Core/PrefabNaming/` owns
   the display-name, description, and icon readers and the two resource-name resolvers. Every
   scanner and extractor calls them by global class name — EnfScript resolves across directories,
   so there is exactly one copy of each.

2. **A domain keeps a helper only when its behaviour genuinely differs:**
   `Core/` carries the form the majority of domains share. Where one domain reads a prefab
   differently — the Stock domain's wider display-name search, the Attachment domain's custom
   attribute key — that domain declares its own under a name that says whose it is. A helper is
   never duplicated to save a call.

3. **Discovery is a census, not an extraction:**
   `TBD_EquipmentScanner` produces counts and prefab paths, classifying by name prefix. It does not
   read components beyond what classification needs. Anything needing real component data belongs in
   a `WeaponExport/` domain, or in the gear domains yet to be written.

4. **The discovery plugin is currently disabled:**
   The `[WorkbenchPluginAttribute]` block in `TBD_EquipmentExportPlugin.c` is commented out, so
   `Export All Equipment (Discovery)` does not appear under `Plugins > TBD`. The scanner compiles
   and is ready to run once the attribute is restored.

5. **Output root:**
   `$profile:TBD_Export/equipment/` (Proton: `.../compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/`).
   Discovery writes `equipment_all.json` at that root; each weapon domain writes its own
   subdirectory.

6. **Compilation:**
   `cargo xtask mod compile` covers only `Scripts/Game` of `TBD_Framework`. This tree compiles
   inside Workbench alone, and new `.c` files need a Workbench cold restart because the script list
   is built at load.
