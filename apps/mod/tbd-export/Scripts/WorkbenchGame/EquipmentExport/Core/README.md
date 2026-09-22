# EquipmentExport/Core

Everything more than one equipment domain uses, grouped by the concern it serves.

A domain folder under `WeaponExport/` answers one question about one class of hardware. The parts
that are not domain-specific — where output goes, how a prefab is flattened into its components,
how a prefab's player-facing strings are read — live here instead, so each has exactly one
definition and a domain reaches it by global class name with no path reference.

```text
Core/
├── ExportDestination/   <-- Where catalog output goes and how it is written
├── ComponentGraph/      <-- Flattening a prefab into the components it declares
└── PrefabNaming/        <-- Reading a prefab's names, icons, and resource references
```

### Roles & Responsibilities
- `ExportDestination/TBD_EquipmentExportConfig.c`: `TBD_EquipmentExportConfig` carries the destination directory and the dialog parameters every plugin reads in `Configure()`.
- `ExportDestination/TBD_EquipmentExportPaths.c`: `TBD_EquipmentExportPaths` normalizes directory paths, creates them segment by segment under `$profile:`, and resolves category subfolders.
- `ExportDestination/TBD_EquipmentExportJson.c`: `TBD_EquipmentExportJson` escapes strings for JSON, performs checked writes that report a short write as an error, and stamps the UTC generation timestamp every catalog and `_meta.json` sidecar carries.
- `ComponentGraph/TBD_EquipmentComponentGraph.c`: `TBD_EquipmentComponentGraph` flattens a prefab into the components it declares, following both nested component arrays and the ancestor chain, and answers membership questions about the result.
- `PrefabNaming/TBD_EquipmentDisplayAttributes.c`: `TBD_EquipmentDisplayAttributes` reads the display name, description, and inventory icon an attachment prefab declares, searching the component and attribute ancestor chains.
- `PrefabNaming/TBD_EquipmentResourceNames.c`: `TBD_EquipmentResourceNames` turns a resource string into the form the catalog exports — the canonical `{GUID}path` where the reference is resolved, the separator-normalized path where it is not — and derives the catalog id from a prefab path.

### Call Flow & Contracts
Nothing here calls into a domain; the dependency runs one way, `Scanner -> Extractor -> Core`. Two
rules keep that edge honest.

**The nesting depth of a component walk belongs to the caller, not to the walk.** A per-hardware
scan stops at 4; the discovery sweep and the M16 compatibility scan need 8 to reach components
buried inside nested slots. `CollectComponentChain` therefore takes the cap as an argument, and each
scanner declares its own `COMPONENT_DEPTH_CAP` beside the scan it governs. `ANCESTOR_CAP` is the
same for every caller and stays here.

**`ResolveCanonicalResourceName` and `NormalizePathSeparators` are different functions, not two
spellings of one.** The first loads the resource to recover its `{GUID}path`; the second only squares
up separators and returns the reference as written. The master arsenal, ammunition, discovery, rifle,
and M16 scans resolve; the eight attachment domains do not. Which one a field went through decides
what the catalog holds, so calling the wrong one changes exported data.
