# Session/Lobby

Pre-game ORBAT slotting, faction selection, player reservations, pre-slot camera, and kit inspection screens.

### Roles & Responsibilities
- `TBD_LobbyData.c`: Serializable wire models representing slot occupancy, locks, and squad groupings.
- `TBD_LobbyService.c`: Server authority validating slot claim/unclaim requests and enforcing roster limits.
- `TBD_LobbyController.c`: Player controller RPC interface routing slotting commands to the server.
- `TBD_LobbyClient.c`: Client-side static cache tracking ORBAT state; raises `ScriptInvoker`s for UI components.
- `TBD_LobbyComponent.c` & `TBD_LobbyStage.c`: Lifecycle managers handling lobby session states and automatic screen display. Since 2026-09-12 the stage watcher raises the **Mission Selector** first on LOBBY; the lobby is its `Lobby` tab, and the soft-modal re-raise stands down while any pre-game screen is open.
- `TBD_PreSlotCamera.c`: Client-side camera providing a smooth scenic orbit overlooking the terrain while a player is in the lobby before body deployment.
- `TBD_PreSlotComponent.c`: `SCR_BaseGameModeComponent` hosting the pre-slot camera lifecycle on `TBD_GameMode.et`.
- `TBD_LobbyCatalog.c`: the read + intent surface behind the rebuilt screen (2026-09-13): factions, squads, slots, kits, `Claim`/`Release`, `GetOnChanged()`. Wire-shaped (mirrors `TBD_LobbyRoster`); `TBD_LobbyMock` builds it today, a `TBD_LobbyClient` adapter implements the same class and calls `Set()` later.
- `UI/TBD_LobbyScreen.c`: `TBD_LobbyScreen : TBD_DockScreen` — mounts the three panels into the dock shell, wires faction → roster → kit inspector, owns the Lock / Ready toggles; keeps `OpenFromPause` and the `PauseMenuUI` "Change slot" hook.
- `UI/TBD_LobbyFactionPanel.c`: FACTIONS column controller + `TBD_LobbyFactionRowComponent`.
- `UI/TBD_LobbyRosterPanel.c`: ROLES column controller + `TBD_LobbySquadCardComponent` (collapsible) + `TBD_LobbySlotRowComponent` (select / claim / release).
- `UI/TBD_KitInspectorPanel.c`: KIT INSPECTOR column — header chips, preview frame, GEAR / WEAPONS / GRENADES / GADGETS / TOOLS / MEDICAL / MISC cards from `TBD_KitInfo`. The visual preview (soldier) is its own later pass; the old `TBD_LoadoutPreview` was deleted with the monolith.

### Call Flow & Contracts
Player joins server -> `TBD_PreSlotComponent` activates `TBD_PreSlotCamera` -> `UI/TBD_LobbyScreen` opens -> Player picks slot -> `TBD_LobbyClient` requests via `TBD_LobbyController` RPC -> `TBD_LobbyService` validates on server -> broadcasts confirmation -> upon deployment, `TBD_PreSlotCamera` lowers and controls transfer to the physical character body.
