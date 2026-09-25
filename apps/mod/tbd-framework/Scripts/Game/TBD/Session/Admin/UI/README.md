# Admin screen

The in-game admin menu: the mission and its validation, the stage, every connected player with
their one-life state, and the audit trail, with one primary action that recovers the selected
player. It renders only what the server sent this client.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/
└── TBD_AdminScreen.c  `TBD_AdminScreen` over the shared shell, and the `TBD_UIAdmin` menu preset
```

## How it works

`TBD_AdminScreen` extends `TBD_ShellScreen` and uses the shared shell layout through the
`TBD_UIAdmin` preset that the file's `modded enum ChimeraMenuPreset` adds and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds. `TBD_AdminClient` opens it through
`TBD_MenuStack`, from the `TBD_AdminMenu` key (F8) or the `#tbd menu` chat command. On open it
draws the cached `TBD_AdminPayload`, asks for a fresh snapshot, and asks again every `REFRESH_MS`
(3 s) while open.

The list has four sections: MISSION (the validation verdict, whose findings open on a pick), STAGE
(forcing the next stage, armed by one pick and confirmed by a second), PLAYERS (name, admin tag,
faction, group and role, and `LIFE SPENT` or `NO BODY`) and ADMIN ACTIONS (the audit trail, opened
on a pick). The one primary action follows the selected player: "RESPAWN <name>" for a spent life,
"DEPLOY <name>" for a live player with no body, sent with `TBD_AdminClient.Act`. The server's
answer lands in the footer status line; the screen opens no modal. For a non-admin the payload
holds only the server's refusal, so the screen has nothing else to draw.

## Authority

- Server: nothing here; every request goes back to `TBD_AdminService` over the admin RPCs on
  `SCR_PlayerController` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionBrowser.c`.
- Client: everything; the screen reads `TBD_AdminClient` only.
- Owner: nothing.
- RPCs: none in this folder.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_AdminClient`, `TBD_AdminPayload` and `TBD_EAdminAction` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`; `TBD_ShellScreen`, `TBD_MenuStack`,
  `TBD_UITheme` and `TBD_ListBox` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: `TBD_AdminClient`, which opens and toggles the `TBD_UIAdmin` preset; the preset entry
  in `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`.
- Rules: the screen holds no permission of its own and no path to a power; every colour comes from
  `TBD_UITheme`; one primary action at a time; lines added stay ASCII and `cargo xtask mod compile`
  checks that the scripts compile.

## Related documentation

- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the pause menu and admin screen as built, and the design target
- [Admin help ticket specification](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md)
  — the admin help ticket and tickets module, designed and not built
