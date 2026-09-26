**Status:** live

# T-1092 — Progress checkpoint

The resume file for [T-1092](/documentation_v2/tickets/specs/t1092_mod_script_modularisation.md).
To resume: read this file, then continue the roster of the
[plan](/documentation_v2/tickets/plans/t-1092_plan.md) from the first row that is not `done`.
Statuses: `pending`, `running`, `done — awaiting commit`, `done`, `blocked`.

## Frozen class names

Referenced from `.et`, `.layout`, `.conf` or `.ent` files in `apps/mod/tbd-framework`; no slice
renames them. Moving their files is fine.

`TBD_AdminScreen` `TBD_BriefingScreen` `TBD_ChipComponent` `TBD_DebriefScreen`
`TBD_DropdownComponent` `TBD_EndScreen` `TBD_FrameworkManager` `TBD_KeyValueRowComponent`
`TBD_ListBox` `TBD_ListBoxRow` `TBD_LoadoutEquipComponent` `TBD_LobbyComponent`
`TBD_LobbyFactionRowComponent` `TBD_LobbyScreen` `TBD_LobbySlotRowComponent`
`TBD_LobbySquadCardComponent` `TBD_MarkerComponent` `TBD_MissionCardComponent`
`TBD_MissionSelectorScreen` `TBD_NavItemComponent` `TBD_NumberedCardComponent` `TBD_ObjectiveHud`
`TBD_ObjectivesComponent` `TBD_PanelComponent` `TBD_PlayAreaComponent` `TBD_PreSlotComponent`
`TBD_PrimaryNavItemComponent` `TBD_RadioComponent` `TBD_SafestartManager`
`TBD_SearchBoxComponent` `TBD_SectionComponent` `TBD_SessionBottomBar` `TBD_SessionTopBar`
`TBD_ShellScreen` `TBD_SpawnManager` `TBD_SpectatorComponent` `TBD_SpectatorScreen`
`TBD_TabStripComponent` `TBD_TerrainRowComponent` `TBD_TopicNavItemComponent` `TBD_UIButton`

Also frozen: every `ChimeraMenuPreset` member name (configs reference them), every `EMCP_` class
name in tbd-emcp (the MCP broker calls them by name).

## Environment at program start

Another session holds uncommitted work in `apps/mod/tbd-export`, `tools_v2/xtask`
(`commands/mod_ops`, `commands/deploy`, `commands/generate`, `Cargo.toml`), `tools_v2/developer-tools`,
`apps/website`, `contracts_v2`, `assets_v2/equipment`, `CLAUDE.md`, `.gitignore` and `Cargo.lock`.
Commits of this program stage by pathspec only.

## Roster

| Id | Status | Commit | Result |
|---|---|---|---|
| P0 | done — awaiting commit | | Tickets T-1092.1–.6, spec, plan, checkpoint |
| P1-1 | pending | | |
| P1-2 | pending | | |
| P1-3 | pending | | |
| P2-1 | pending | | |
| P2-2 | pending | | |
| P3-1 | pending | | |
| P3-2 | pending | | |
| P3-3 | pending | | |
| P3-4 | pending | | |
| P3-5 | pending | | |
| P3-6 | pending | | |
| P3-7 | pending | | |
| P3-8 | pending | | |
| P3-9 | pending | | |
| P3-10 | pending | | |
| P3-11 | pending | | |
| P3-12 | pending | | |
| P3-13 | pending | | |
| P3-14 | pending | | |
| P3-C | pending | | |
| P4-1 | pending | | |
| OP-1 | pending | | operator playtest |
| P5-1 | pending | | |
| P6-1 … P6-6 | blocked | | waits for the other session's tbd-export commit |
| P6-C | pending | | |

## Amendments

Additions to a launch prompt beyond concrete values, by slice id.

| Slice | Addition |
|---|---|

## Comment gate baseline

Filled by P1-2 and P1-3.

## Shared helper index

Filled by P2-1: path | class.method signature | replaces (file:method) | parameter notes.

## Tick-order baseline

Filled by P2-2.

## Forwarders

Left by P3 slices for P3-C to remove: forwarder | owner file | external callers.

## Leftovers

Per slice, for the closing runs.

## Ticket batch

Bugs noticed but not fixed, collected by P3-C and P6-C.
