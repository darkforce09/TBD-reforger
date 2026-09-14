# Session/Briefing

Tactical map briefing: the wire (server → owner RPC) and the rebuilt Briefing screen (2026-09-14).

### Roles & Responsibilities
- `TBD_BriefingData.c`: Serializable models holding mission orders, situation reports, and rules of engagement.
- `TBD_BriefingService.c`: Server service assembling faction-specific briefing payloads from the loaded mission.
- `TBD_BriefingController.c`: Player controller RPC conduit synchronizing briefing payloads to connected clients.
- `TBD_BriefingClient.c`: Client cache holding briefing text and firing invokers when data arrives.
- `TBD_BriefingCatalog.c` (2026-09-14): the read surface behind the screen — nets, objectives, rule groups, lore, parameters, assets (both sides), uniforms (both sides), plans, own/enemy faction. Wire-shaped presentation models; `TBD_BriefingMock` builds it today, an adapter over `TBD_BriefingPayload` + `TBD_RadioClient` + `TBD_MarkerClient` + `TBD_LobbyCatalog` calls `Set()` later (vehicles need a wire line first — the roster is server-only).
- `UI/TBD_BriefingScreen.c`: `TBD_BriefingScreen : TBD_DockScreen` over the full-screen `SCR_MapEntity` (map lifecycle kept verbatim, `MapContext` re-armed each tick). `TBD_BriefingPrimaryNav` in LeftDock, `TBD_BriefingTopicNav` (10 topics, 3 groups) in CenterDock, the page in RightDock at its mockup width; `SetMode` / `ShowPage`; `LocateOnMap(x, z)` pans the map; bottom bar = Lock Lobby (mock) + Ready & Continue (reports readiness, then `TBD_SpawnClient.Request()` → `TBD_SpawnManager.DeployOnReady`; deployed → `TBD_MenuStack.CloseAll()`); PLAYERS is a mode: `TBD_PlayersPanel` pops out beside the primary nav in `WideDock`, the map live behind it.
- `UI/TBD_BriefingPrimaryNav.c`: the primary navigation panel (`TBD_BriefingPrimaryNav` + `TBD_PrimaryNavItemComponent : TBD_UIInteractive`): glass panel, icon boxes, active blue fill with the white accent bar, BLUFOR / OPFOR slotted-count chips on Players; `GetOnSelected()(nav, index)` with `TBD_EBriefingMode` indices.
- `UI/TBD_BriefingTopicNav.c`: the topic navigation panel (`TBD_BriefingTopicNav` + `TBD_TopicNavItemComponent`): ten topics from `TBD_BriefingNav.TopicItems`, separators between the three groups, active = solid blue; `GetOnSelected()(nav, index)` with `TBD_EBriefingPage` indices.
- `UI/TBD_BriefingNav.c`: `TBD_EBriefingMode`, `TBD_EBriefingPage`, the two item tables, `PageWidth`, `CreatePage`.
- `UI/TBD_BriefingPage.c`: page base — `TBD_PanelFill` + `TBD_ScrollList`, badge chips, tracked 3D previews (destroyed with the page), Locate buttons, `AddRow` / `AddCellGrid` helpers.
- `UI/TBD_BriefingPageInfo.c`: Objectives (time-limit row, numbered cards with a stat grid + Locate), Rules (two `TBD_Section` groups of numbered cards), Background (inset paragraphs), Parameters (icon rows).
- `UI/TBD_BriefingPageComms.c`: Frequencies (LR net gold, SR nets, own net highlighted), ORBAT (the lobby roster read-only + kit inspector, own side).
- `UI/TBD_BriefingPageAssets.c`: Assets (per type: section → Vehicle Info with a real vehicle render + specs, one section per vehicle with ammunition + inventory grids, Locate on the friendly side) and Uniforms (one card per faction component, doll = that component's rifleman prefab). Enemy pages = the same builders in the enemy tint.
- `UI/TBD_BriefingMarkersPanel.c`: plan dropdown + `Load Plan` (logs the plan id; no plan store yet, by operator word).

### Call Flow & Contracts
Server `TBD_BriefingService` compiles mission text -> `TBD_BriefingController` RPC -> `TBD_BriefingClient` cache (unchanged). The screen reads `TBD_BriefingCatalog.Get()` only; wiring the payload into it is the adapter pass, together with `TBD_BriefingClient.ReportReady` behind the bottom bar's Ready toggle.
