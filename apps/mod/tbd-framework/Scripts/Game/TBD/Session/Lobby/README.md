# Session/Lobby

Pre-game ORBAT slotting, faction selection, player reservations, pre-slot camera, and kit inspection screens.

### Roles & Responsibilities
- `TBD_LobbyData.c`: Serializable wire models representing slot occupancy, locks, and squad groupings.
- `TBD_LobbyService.c`: Server authority validating slot claim/unclaim requests and enforcing roster limits.
- `TBD_LobbyController.c`: Player controller RPC interface routing slotting commands to the server.
- `TBD_LobbyClient.c`: Client-side static cache tracking ORBAT state; raises `ScriptInvoker`s for UI components.
- `TBD_LobbyComponent.c` & `TBD_LobbyStage.c`: Lifecycle managers handling lobby session states and automatic screen display.
- `TBD_PreSlotCamera.c`: Client-side camera providing a smooth scenic orbit overlooking the terrain while a player is in the lobby before body deployment.
- `TBD_PreSlotComponent.c`: `SCR_BaseGameModeComponent` hosting the pre-slot camera lifecycle on `TBD_GameMode.et`.
- `UI/TBD_LobbyScreen.c`: Three-column tactical workstation UI (factions, squad cards, slot inspector).
- `UI/TBD_LoadoutPreview.c`: Visual gear grid inspecting weapons and apparel for selected slots.

### Call Flow & Contracts
Player joins server -> `TBD_PreSlotComponent` activates `TBD_PreSlotCamera` -> `UI/TBD_LobbyScreen` opens -> Player picks slot -> `TBD_LobbyClient` requests via `TBD_LobbyController` RPC -> `TBD_LobbyService` validates on server -> broadcasts confirmation -> upon deployment, `TBD_PreSlotCamera` lowers and controls transfer to the physical character body.
