# TBD Gameplay Framework (`mod/tbd_framework/`)

The core shipping gameplay mod for Arma Reforger (`TBD_Framework`), compiled directly into the game runtime.

## Subsystems
- `Gamemode/`: State lifecycle manager, match phase transitions, safe-start warmup zones.
- `Session/`: Player reservation tokens, slot materialization, spawn determinism.
- `Systems/`: Radio frequency networks, gear/loadout distribution, capture objectives, play areas, tactical markers.
- `UI/`: Tactical HUD widgets, compass, slotting lobby, briefing dossier dock, pause menu.
- `API/`: REST API client and backend event streaming bridge.

## Code Mapping
- Source: `apps/mod/tbd-framework/Scripts/Game/TBD/`
- Prefabs & Configs: `apps/mod/tbd-framework/Prefabs/`, `apps/mod/tbd-framework/Configs/`
