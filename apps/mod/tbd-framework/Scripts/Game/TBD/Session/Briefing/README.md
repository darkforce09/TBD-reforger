# Session/Briefing

Tactical map briefing screen, operational orders, and mission parameter review.

### Roles & Responsibilities
- `TBD_BriefingData.c`: Serializable models holding mission orders, situation reports, and rules of engagement.
- `TBD_BriefingService.c`: Server service assembling faction-specific briefing payloads from the loaded mission.
- `TBD_BriefingController.c`: Player controller RPC conduit synchronizing briefing payloads to connected clients.
- `TBD_BriefingClient.c`: Client cache holding briefing text and firing invokers when data arrives.
- `UI/TBD_BriefingScreen.c`: Interactive map screen combining native `SCR_MapEntity` drawing with a collapsible tactical drawer.

### Call Flow & Contracts
Server `TBD_BriefingService` compiles mission text -> `TBD_BriefingController` RPC sends payload to owner -> `TBD_BriefingClient` caches data -> `UI/TBD_BriefingScreen` binds and renders orders drawer.
