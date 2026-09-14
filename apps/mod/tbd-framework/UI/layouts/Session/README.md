# UI/layouts/Session

Screen layouts and dock sub-layouts for the player & referee match journey.

## Subdirectories

- **`Lobby/`:** ORBAT slotting — dock shell + 9 sub-layouts (factions, squad cards / slot rows, kit inspector). Second Dock & Sub-Layout screen (2026-09-13).
- **`Briefing/`:** the Briefing dock shell over the live map + its page sub-layouts (FreqRow, OrbatPage, AssetPreview, UniformCard, MarkersPanel); shipped 2026-09-14.
- **`Spectator/`:** One-life elimination camera UI, broadcast top/bottom bars, and anatomical trauma forensics.
- **`Admin/`:** Game master mission control panel and administrative tooling.
- **`MissionSelector/`:** Scenario browser — dock shell + 7 sub-layouts (terrain list, mission cards, inspector). First Dock & Sub-Layout screen (2026-09-12).
- **`PostGame/`:** End match victory outcome banner and after-action review (AAR) scoreboard.
- **`Shared/`:** Chrome every pre-game screen wears: `TBD_SessionTopBar`, `TBD_SessionBottomBar`, the PLAYERS panel (`TBD_PlayersPanel` + lanes, shipped 2026-09-14); voice panel pending.
- **`Pause/`:** In-game pause menu and player readiness voting.
