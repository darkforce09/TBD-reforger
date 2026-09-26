**Status:** live

# Briefing screen

The Briefing tab of the pre-game screens: during the `BRIEFING` stage every player reads their
side's orders, radio nets, assets, uniforms and [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) over
the live map, sees who is slotted, and presses "Ready & Continue" to be deployed into their
[slot](/documentation_v2/glossary/n_to_z.md#slot). The pages render mock content; the deploy is live.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/README.md)
  (the screen, its two navigation panels and ten pages) and
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/README.md)
  (the catalog, the per-side briefing payload and the ready tally).
- Layouts: [`apps/mod/tbd-framework/UI/layouts/Session/Briefing/`](/apps/mod/tbd-framework/UI/layouts/Session/Briefing/README.md),
  whose README gives the dock geometry, the page widths and every widget the handlers bind.
- Entry: `TBD_BriefingScreen` on the `TBD_UIBriefing` menu preset
  (`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/TBD_BriefingScreen.c`).
- Related features: the [lobby](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md),
  whose roster and kit inspector the ORBAT page reuses; the
  [players panel](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/README.md), a mode of
  this screen.

## Behaviour

### Opening and closing

1. When the server enters `BRIEFING`, `TBD_FrameworkManager` pushes `TBD_OnStageChanged` to every
   controller; the client resets `TBD_BriefingClient` and opens the screen. Any other stage closes
   it, and a joining client reads the replicated stage once instead. The top bar's "Briefing" tab
   also opens it.
2. The screen opens the map full screen under every panel, zoomed out and panned to the player's
   body or, with none, the game mode's origin. While the cursor is over a panel or the page
   column, the map's input context is withheld, so the wheel scrolls the panel instead of zooming.
3. The top bar shows the mission id in a mono face, the "Briefing" tab active, the player's
   identity chip and the connected-of-capacity count; the bottom bar holds "Lock Lobby" and the
   primary "Ready & Continue".

### Modes and pages

The primary navigation, on the left, picks one of four modes; the screen opens in Briefing:

| Mode | What shows |
|---|---|
| Map | the navigation only; the map is clear |
| Briefing | the topic navigation and the chosen page beside it |
| Players | the players panel: "TOTAL: N PLAYERS", then BLUFOR and OPFOR lanes ("Slotted:"), Spectators ("Count:") and Unslotted ("Pending:"); the Players item carries slotted-count chips per side |
| Markers | a "Tactical Plan" panel (`N Available`), a plan dropdown ("No plans" when empty) and "Load Plan" |

The topic navigation lists ten pages in three groups, and opens on Frequencies:

| Group | Page | Content |
|---|---|---|
| Comms and force | Frequencies | "Radio Frequencies Net": the "Long Range (LR) Command" net, then "Short Range (SR) Squad Nets" (`N nets`), each with its frequency chip and auxiliary channels |
| | ORBAT | the lobby's roster read only, with a Locate button per squad, beside its kit inspector |
| Assets and identification | Friendly Assets, Enemy Assets | per vehicle type a "Vehicle Info" section (3D preview, "Vehicle Weapons", "Mobility", "Crew & Capacity"), then a row per vehicle with "Locate Vehicle", "Vehicle Ammunition" and an "Inventory" of ammunition, weapons, grenades, medical and miscellaneous items |
| | Friendly Uniforms, Enemy Uniforms | one card per faction with a 3D rifleman preview, gear chips and the camouflage |
| Orders | Objectives | "Mission Objectives": "Directives" with the capture summary, "Time Limit", then each objective with its type, capture time, retake rule and Locate |
| | Rules | the rule groups, one section each |
| | Background | the mission's background text |
| | Parameters | the mission parameters as key-value rows |

Enemy pages reuse the friendly builders in the OPFOR tint. Every Locate pans the map smoothly to
the position without leaving the page.

### Ready and deploy

1. "Ready & Continue" reports the player ready (`TBD_BriefingClient.ReportReady`) and asks the
   server for a body (`TBD_SpawnClient.Request`); the button reads "Deploying…" and is disabled.
2. The server's `TBD_SpawnManager.DeployOnReady` deploys the player into their claimed slot. On a
   deployed answer the screen closes every menu (`TBD_MenuStack.CloseAll`) one tick later; on a
   refusal the button shows the reason in the warning tint and can be pressed again.
3. The server records the ready report and answers with the player's own side's tally.
4. "Lock Lobby" flips to "Unlock Lobby" and back and logs; it reaches no server.

### Known discrepancies

- The pages show the caller's side briefing (`TBD_BriefingCatalog.c`) — but the catalog is built
  from `TBD_BriefingMock` (`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`), because no script
  calls `Set()`; the real per-side payload reaches `TBD_BriefingClient` and no page reads it. The
  players panel and the lobby roster behind the ORBAT page are mock catalogs too.
- The Markers mode offers "Load Plan" (`TBD_BriefingMarkersPanel.c`) — but it only logs the chosen
  plan id; no plan store exists.
- "Lock Lobby" reads as a session control (`TBD_BriefingScreen.c`) — but it only toggles its label.

## Data

The screen reads catalogs; the calls behind it travel through `TBD_BriefingController` and
`TBD_SpawnClient`, listed in the Briefing README's
[How it works](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/README.md#how-it-works):

- `TBD_RpcAsk_Briefing` → `TBD_RpcDo_Briefing(wire, situation, mission, execution)`: the server
  resolves the caller's side from its own state and builds that side's briefing from
  `TBD_SpawnManager` and `TBD_MissionLoader`: tab-separated lines, at most 400, with the orders'
  situation, mission and execution texts as separate string arrays. A player never receives the
  other side's briefing.
- `TBD_RpcAsk_Ready` → `TBD_RpcDo_ReadyTally`: the server records the player as ready and answers
  with their own side's tally.
- The deploy request of `TBD_SpawnClient` (`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`):
  `TBD_SpawnManager.DeployOnReady` puts the player in their slot's body, and the answer, deployed
  or refused with a reason, returns to the screen.

## Design

- As built: a dock shell laid over the vanilla full-screen map, which never resizes: a 288 px
  primary navigation, a 340 px topic navigation, and the page at its own width (ORBAT 1200, the
  uniforms 768, the assets 576, rules 480, frequencies 448, the other pages 440); the players
  panel takes an 800 px `WideDock`. The
  [layouts README](/apps/mod/tbd-framework/UI/layouts/Session/Briefing/README.md) draws the shell.
- Design target: the fourteen Stitch sets in
  [visual_references](/documentation_v2/mod/tbd-framework/UI/briefing/visual_references/README.md),
  design-phase references. Differences: the voice panel is not built; the page the "Lore" set
  draws is titled "Background"; the set names do not follow their sides
  (`friendly_vehicles_panel_2_mockup` draws Enemy Assets, `visual_pid_uniforms_panel_opfor_1_mockup`
  draws Friendly Uniforms).
- Starting point: the sixteen Arma 3 captures in
  [reference_screenshots](/documentation_v2/mod/tbd-framework/UI/briefing/visual_references/reference_screenshots/README.md).
  TBD keeps their map-first overlay, the nested navigation, clickable positions, the radio plan with
  auxiliary channels, vehicle manifests with cargo, uniform identification, the side's ORBAT with
  trait chips and the mission parameters; it replaces the six system tabs with four modes and the
  sixteen topics with ten pages, and has no personal squad page, marker audit log or network
  diagnostics.

## Open work

- [T-1085 — Feed the pre-game screens live catalogs instead of mocks](/.ai/tickets/T-1085.toml)
  (idea, no plan): the pages read the per-side payload `TBD_BriefingClient` already receives, and
  the players panel and ORBAT page read live rosters.
- [T-181.16 — Two-client dedicated-server event loop E2E](/.ai/tickets/T-181.16.toml) (queued, no
  plan): a human playtest of brief and deploy with two real clients.

## Decisions

- The briefing is a dock shell over the live map, never a menu stack: a menu pushed on top would
  close the map, so Players and Markers are modes inside the screen.
- The server builds each player's briefing for their own side: an enemy plan never reaches a
  client, whatever the client asks for.
- "Ready & Continue" deploys: reading the orders is the last step before a body, so the ready
  report and the deploy request leave together.
- The stage handler is the one opener and closer on the stage change, so every client leaves the
  briefing when the server does.
