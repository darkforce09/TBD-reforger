# Session/Spectator

Post-elimination and referee spectator mode, free/follow cameras, and unit tracking HUD.

### Roles & Responsibilities
- `TBD_SpectatorCamera.c`: Client camera controller driving free-cam movement, orbit, and first-person follow modes.
- `TBD_SpectatorTargets.c`: Query manager indexing living players, vehicles, and spectator targets.
- `TBD_SpectatorHost.c` & `TBD_SpectatorHostEntity.c`: Dedicated non-character entity socket hosting spectator cameras without game presence.
- `TBD_SpectatorComponent.c` & `TBD_SpectatorController.c`: Host component and RPC endpoints managing spectator state transitions.
- `UI/TBD_SpectatorScreen.c`: HUD overlay providing alive player lists, faction rosters, and camera controls.

### Call Flow & Contracts
Player death in `Gamemode/Spawning` triggers spectator transition -> client attaches `TBD_SpectatorCamera` to host entity -> `UI/TBD_SpectatorScreen` displays living unit list for targeted observation.
