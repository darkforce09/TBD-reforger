# Framework interface texture root

The root of the framework's own interface images. Its one child holds every texture the framework
ships, under the `TBD` name so its resource paths never collide with the game's
`UI/Textures/` tree.

## Contents

```text
apps/mod/tbd-framework/UI/Textures/
└── TBD/  the framework's interface textures: rounded-shape disc, hero art, masks and icons
```

## How it works

[Enfusion](/documentation_v2/glossary.md#enfusion) merges an addon's files with the game's data
into one resource tree, so `UI/Textures/TBD/…` sits beside the vanilla `UI/Textures/…` folders
without replacing any of them. The framework's scripts and layouts address these textures by
resource GUID or path through `TBD_UILayouts` and `TBD_UIIcons`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`).

## Format

- File type: PNG sources, each beside the `.edds` texture and `.edds.meta` that
  [Workbench](/documentation_v2/glossary.md#workbench) writes on
  import; the child's README gives the details.
- Resource GUID: each texture's `.edds.meta`.
- Naming: framework textures live under `TBD/`, never directly here.
- Adding an asset: add it under `TBD/` as that folder's README describes.

## Referenced by

- The rounded shapes in `apps/mod/tbd-framework/UI/layouts/Common/`, by resource GUID.
- `TBD_UILayouts` and `TBD_UIIcons` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`, by
  resource GUID and by path.

## Boundaries

- Depends on: the vanilla `TextureUI.conf` import settings.
- Used by: the framework's layouts and screens, as listed above.
- Rules: every framework texture sits under `TBD/`, so no path shadows a vanilla texture.
