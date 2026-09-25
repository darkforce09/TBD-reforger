# Eden editor data

[Workbench](/documentation_v2/glossary.md#workbench)'s per-world editor data for Eden, the vanilla
Everon world the TBD Dev POC world is a sub-scene of. Nothing in the game reads it.

## Contents

```text
apps/mod/tbd-framework/worlds/Eden/
└── .EditorData/  Workbench editor state for Eden: an empty user map list, `UserMaps.desc`
```

## Format

- File type: `.EditorData/UserMaps.desc` is a Workbench `UserMapDescClass` descriptor, plain text,
  empty here.
- Resource GUID: none; the file has no `.meta`.
- Naming: Workbench writes it under `worlds/<world>/.EditorData/`, mirroring the path of the
  world it edits, `worlds/Eden/Eden.ent` in the game's data.
- Adding a file: nothing is added by hand; Workbench writes this folder when Eden is edited with
  the addon open.

## Referenced by

- Workbench, when it opens the Eden world with this addon loaded. No script, prefab, config or
  [mission header](/documentation_v2/glossary.md#mission-header) names the file.

## Boundaries

- Depends on: the vanilla Eden world in the game's data.
- Used by: Workbench only.
- Rules: the addon holds no copy of Eden itself, only this editor data; a Workbench session that
  rewrites the file is editor churn, reviewed before it is committed.
