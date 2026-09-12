# UI/layouts/Session/Lobby

Fullscreen ORBAT slotting screen shell and modular dock sub-layouts for team selection and role assignment.

## Layouts

- **`TBD_LobbyScreen.layout`:** The primary screen shell. Contains named layout docks (`HeaderDock`, `SidebarDock`, `CenterRosterDock`, `InspectorDock`, `FooterDock`) into which the sub-layouts below are dynamically instantiated.
- **`TBD_LobbyHeader.layout`:** Top banner dock displaying scenario metadata, match status, player count, and staging countdown timer.
- **`TBD_LobbyFooter.layout`:** Bottom action bar housing primary state transitions (Ready / Unready toggle, Spectate, Switch Faction, Leave).
- **`TBD_LobbySidebar.layout`:** Left-hand navigation panel listing available factions, team sizes, and side balance indicators.
- **`TBD_LobbyFactionRow.layout`:** Repeater item layout for selectable faction tabs within the sidebar.
- **`TBD_LobbyCenterRoster.layout`:** Center scrollable container organizing squads and fireteams into cards.
- **`TBD_LobbySquadCard.layout`:** Squad group container wrapping individual slot rows under a squad header (e.g. Alpha 1-1).
- **`TBD_LobbySlotRow.layout`:** Interactive slot row showing role icon, slot name, occupant player name, ping indicator, and lock/ready state.
- **`TBD_LobbyInspector.layout`:** Right-hand detail panel displaying selected role attributes, loadout description, and kit details.
- **`TBD_LoadoutPreview.layout`:** Sub-panel inside the inspector rendering visual loadout gear cards and weapon specifications.
