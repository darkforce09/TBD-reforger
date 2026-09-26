**Status:** live

# Mission selection screen

The Mission Selector, the "Scenario Browser" tab of the pre-game screens: a player browses
terrains, the [missions](/documentation_v2/glossary/g_to_m.md#mission) built for each, and one mission's
versions, modset, summary, [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and objectives, and picks
the mission and version the lobby and briefing are titled with. The screen renders mock data and
requests no deployment.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/README.md)
  (the screen and its panels) and
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/README.md)
  (the models, `TBD_MissionCatalog`, `TBD_SessionSelection`, and the admin deployment path).
- Layouts: [`apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/`](/apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/README.md),
  whose README gives the dock geometry, the inspector hero and every widget the handlers bind.
- Entry: `TBD_MissionSelectorScreen` on the `TBD_UIMissionSelector` menu preset
  (`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/TBD_MissionSelectorScreen.c`).
- Related features: the [lobby](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md)
  and the [briefing](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md),
  which read the pick.

## Behaviour

### Opening

1. On entering the `LOBBY` stage, `TBD_LobbyStage` raises the Mission Selector, and raises it
   again while the stage stays in `LOBBY`, no pre-game screen is open, the player has no body and
   no deploy was accepted.
2. F9 (`TBD_MissionSelector` in the `TBD_BrowserContext` input context) raises or drops it
   (`Toggle`); the top bar's "Scenario Browser" tab reaches it from the lobby and briefing.
3. The top bar shows "Scenario Browser" as the title, the player's identity chip and the connected
   count; the bottom bar holds one primary action, "Select Scenario".

### Browsing

1. The "Terrains" column lists each terrain with an icon, its mission count and a chevron, under a
   "Search Maps..." field; the first terrain is selected on open. A search with no hit shows "No
   terrain matches.".
2. The centre column, titled "<terrain> Missions" with an `N AVAILABLE` chip, lists that terrain's
   mission cards: a mode tag, the terrain, `N SLOTS` and the title. "Search Scenario..." filters by
   title as the player types, and the Modes dropdown ("Filter modes") narrows by mode, with a count
   per mode. No hit shows "No scenario matches.".
3. Choosing a card fills the inspector and enables "Select Scenario": a hero band with the terrain
   art, the title, a mode tag, "by" and the author with an `AUTHOR` chip, and a version dropdown
   ("Select mission version"); then four cards: "Required Modset & Mods" (each mod with a synced
   or missing mark and its version, `N SYNCED & ACTIVE`), "Mission Summary" (`SITREP`), "ORBAT
   Overview" (per faction, slots and assets, `N SLOTS`) and "Objectives" (per faction,
   `N ACTIVE`). With no card chosen it reads "Pick a scenario to inspect it.".

### Selecting

1. "Select Scenario" stores the mission and version label in `TBD_SessionSelection`, which titles
   the lobby and briefing, and logs `[TBD][selector] SELECT SCENARIO <title> (<id>) on <terrain>,
   version <label>`. With nothing selected it logs a warning and does nothing.
2. Changing the version logs the label; nothing else follows.

### Known discrepancies

- The screen lists the platform's missions (`TBD_MissionCatalog` in `TBD_MissionSelectorData.c`)
  — but the catalog is built from `TBD_MissionSelectorMock`
  (`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`), because no script calls `Set()`.
- "Select Scenario" reads as choosing what the server runs (`TBD_MissionSelectorScreen.c`) — but
  it only records the pick for the next screens. Deploying a mission from inside the game is the
  admin path of the MissionSelector folder: F6 and F7, or `#tbd missions` and
  `#tbd mission <n>`, relay the pick to the platform (see Data).
- Comments in `TBD_MissionSelectorScreen.c` and `TBD_MissionBrowser.c` name F6 for this screen —
  but F6 is `TBD_MissionCycle`, which steps through the deployable list; F9 opens the screen.

## Data

The screen makes no HTTP call and sends no RPC. The admin deployment path beside it, listed in the
MissionSelector README's [How it works](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/README.md#how-it-works):

- `GET /api/v1/game-runtime/missions` (`TBD_DeployableMissionList.Refresh`, on entering `LOBBY`
  or `#tbd refresh`): the missions the platform lets this server deploy, by title; the browser
  numbers them from 1 and marks the running one.
- `POST /api/v1/game-runtime/deployments` (`TBD_MissionDeploymentRelay.RequestByNumber`): the
  admin's pick as a [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment)
  request with the admin's `arma_id`; the platform decides whether that identity is an
  administrator and validates the pick as it validates a website deployment. The relay restarts
  nothing; a refusal reaches the admin's chat in words for its `details.code`.

## Design

- As built: a dock shell with a 320 px Terrains column, a 440 px mission column and the inspector
  filling the rest, between the 56 px top bar and the 64 px bottom bar; the inspector's 176 px
  photo hero fades into the card stack. The
  [layouts README](/apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/README.md) gives the
  geometry and the corner masks.
- Design target: the four Stitch sets in
  [visual_references](/documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/README.md)
  (top bar, terrain selector, mission browser, mission inspector), design-phase references. The
  built panels follow them; their mock data carries the mockups' sample missions and modsets.
- Starting point: the Arma 3 "CREATE GAME" screen in
  [reference_screenshots](/documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/reference_screenshots/README.md).
  TBD keeps its terrain-first drill-down, the mission search and a summary of the chosen mission;
  it drops the mission settings and difficulty panel, the chat tab, the in-game editor shortcut,
  "SHOW ALL MISSIONS", `GAME OPTIONS`, the Steam Workshop button and `BACK`, and adds versions,
  the required modset, the ORBAT and the objectives, since a TBD mission comes from the platform.

## Open work

- [T-1085 — Feed the pre-game screens live catalogs instead of mocks](/.ai/tickets/T-1085.toml)
  (idea, no plan): the catalog reads the platform's missions.
- [T-1084 — Arm the mission browser input context every frame](/.ai/tickets/T-1084.toml) (idea, no
  plan): the F6 to F9 keys stay live, and the F6 comments are corrected.

## Decisions

- The screen records a pick and deploys nothing: deployment is an administrator's act the platform
  validates, so it stays on the admin keys and chat commands.
- Terrain first: a mission belongs to one terrain, so choosing the terrain scopes the list and the
  counts.
