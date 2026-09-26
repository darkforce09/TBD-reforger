# Framework input contexts

The two input contexts of the framework [mod](/documentation_v2/glossary/g_to_m.md#mod): each groups the
framework's own key actions into a layer that scripts switch on by name, so the keys fire only while
that layer is active and never collide with gameplay bindings.

## Contents

```text
apps/mod/tbd-framework/Configs/System/ActionContext/
├── TBD_BrowserContext.conf         the admin and mission selector keys: F6, F7, F8 and F9
├── TBD_BrowserContext.conf.meta    its resource GUID, `{7BD1A70000000740}`
├── TBD_SpectatorContext.conf       the spectator keys: roster, next, previous, view, free camera
└── TBD_SpectatorContext.conf.meta  its resource GUID, `{7BD1A70000000741}`
```

## How it works

Each file is one `ActionContext` whose `ActionRefs` name actions defined in
`apps/mod/tbd-framework/Configs/System/Actions/`. A script activates a context through the input
manager and registers a listener per action; an action fires only while a context that lists it is
active.

| Context | Actions | Activated by |
|---|---|---|
| `TBD_BrowserContext` | `TBD_MissionCycle`, `TBD_MissionLoad`, `TBD_AdminMenu`, `TBD_MissionSelector` | the mission browser's modded `SCR_PlayerController`, once, when the local client first controls an entity |
| `TBD_SpectatorContext` | `TBD_SpecRoster`, `TBD_SpecNext`, `TBD_SpecPrev`, `TBD_SpecView`, `TBD_SpecFree` | `TBD_SpectatorCamera`, every frame while the spectator camera exists |

## Format

- File type: Enfusion config (`.conf`), plain text of the form `ActionContext <name> { ActionRefs
  { "<action>" … } }`; the context name is the one scripts pass to `ActivateContext`.
- Resource GUID: each `.conf` has a `.conf.meta` whose `Name "{GUID}Configs/…"` line holds its GUID
  and one `CONFResourceClass` block per platform. The GUIDs sit in the hand-assigned
  `7BD1A700000007xx` range the framework's configs share; a GUID never changes once assigned.
- Naming: `TBD_<Purpose>Context.conf`, one context per screen or mode.
- Adding a context: write the `.conf` and its `.meta` (in Workbench or by hand from a sibling),
  commit the pair together, open the addon in Workbench once so `resourceDatabase.rdb` lists the
  new file, and activate the context by name in the script that owns the keys.

## Referenced by

- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionBrowser.c` activates
  `TBD_BrowserContext` by name.
- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/TBD_SpectatorCamera.c` activates
  `TBD_SpectatorContext` by name (`CTX_SPECTATOR`).
- `apps/mod/tbd-framework/resourceDatabase.rdb` registers both files; no config or prefab names them
  by GUID.

## Boundaries

- Depends on: the actions in `apps/mod/tbd-framework/Configs/System/Actions/`, named by action
  name.
- Used by: the mission browser and spectator scripts named above.
- Rules: a context lists only actions that exist in the sibling `Actions/` folder; the `.conf` and
  its `.meta` are committed together; a context name, once scripts use it, stays stable.

## Related documentation

- [Spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
  — the spectator controls the context serves.
- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the screen `TBD_MissionSelector` opens.
