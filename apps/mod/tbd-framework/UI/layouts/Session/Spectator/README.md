# Spectator screen layouts

The place for layouts that only the spectator interface uses. It holds none: the spectator roster,
`TBD_SpectatorScreen`, draws on the shared list shell
`apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`, repainted transparent so the
world stays visible behind it.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Spectator/
```

## Format

- File type: none here; a layout added for the spectator interface alone is an
  [Enfusion](/documentation_v2/glossary.md#enfusion) widget layout with its `.layout.meta`, named
  `TBD_Spectator<Element>.layout`, with a block from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c` and a `TBD_UILayouts`
  constant.

## Referenced by

None: no resource refers to this folder. The `TBD_Spectator` preset in
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` opens `TBD_ScreenShell.layout` with the
`TBD_SpectatorScreen` class from `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/`.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a layout goes here only when the spectator interface alone uses it; one another screen
  shares goes to `apps/mod/tbd-framework/UI/layouts/Common/`.

## Related documentation

- [Spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
  — the spectator as built, its policies and controls, and the design target
