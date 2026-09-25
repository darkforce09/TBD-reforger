# Export addon prefabs

The entity templates of the export addon. The addon has one, the game mode that the export world
places to run the road export when the export
[mission header](/documentation_v2/glossary.md#mission-header) is played.

## Contents

```text
apps/mod/tbd-export/Prefabs/
└── Systems/  the export game mode, carrying the runtime road exporter component
```

## How it works

Enfusion names a prefab by its resource GUID and its path inside the addon. The export world's
layer places the game mode from `Systems/` by GUID, and the game mode carries
`TBD_RoadExportComponent`, the class in `apps/mod/tbd-export/Scripts/Game/TBD/Export/` that writes
the road files.

## Format

- File type: Enfusion entity templates (`.et`), each derived from a vanilla prefab and holding only
  what it overrides.
- Resource GUID: each `.et` has a `.et.meta` file whose `Name` line holds the GUID other resources
  refer to it by; a referenced GUID never changes.
- Naming: `TBD_Export_<Subject>.et`, grouped in a subfolder per role (`Systems/` for the game mode).
- Adding a prefab: create it in [Workbench](/documentation_v2/glossary.md#workbench) inside this
  addon and commit the `.et` with its `.meta`.

## Referenced by

- `apps/mod/tbd-export/worlds/TBD_Export_Everon_Layers/default.layer` places
  `Systems/TBD_Export_GameMode.et` by resource GUID.

## Boundaries

- Depends on: the vanilla game mode prefab and the script classes in
  `apps/mod/tbd-export/Scripts/Game/`.
- Used by: the export world in `apps/mod/tbd-export/worlds/`.
- Rules: prefabs here name no `tbd-framework` resource or class; a prefab and its `.meta` are
  committed together.
