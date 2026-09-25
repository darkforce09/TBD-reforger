**Status:** live

# Lobby screen

The Lobby tab of the pre-game screens: a player picks a faction, reads its squads and
[slots](/documentation_v2/glossary.md#slot), claims a seat in the
[ORBAT](/documentation_v2/glossary.md#orbat) and inspects the seat's kit on a 3D doll before the
[event](/documentation_v2/glossary.md#event) goes to the briefing. The screen renders mock data;
the server-side roster wire it will use exists beside it.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/README.md)
  (the screen and its panels) and
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/README.md)
  (the catalog, the stage watcher and the roster wire).
- Layouts: [`apps/mod/tbd-framework/UI/layouts/Session/Lobby/`](/apps/mod/tbd-framework/UI/layouts/Session/Lobby/README.md),
  whose README gives the dock geometry and every widget the handlers bind.
- Entry: `TBD_LobbyScreen` on the `TBD_UILobby` menu preset
  (`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/TBD_LobbyScreen.c`).
- Related features: the [Mission Selector](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md),
  whose pick titles the lobby, and the [briefing](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md),
  whose ORBAT page reuses the lobby's roster and kit inspector read only.

## Behaviour

### Opening

1. On the `LOBBY` stage, `TBD_LobbyStage` raises the Mission Selector, not the lobby; the player
   reaches the lobby through the top bar's "Lobby" tab. The Lobby README's
   [stage watcher](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/README.md#the-stage-watcher)
   gives the timings.
2. During `BRIEFING`, `SAFE_START` and `LIVE`, the game's pause menu shows "Change slot" in place
   of its leave-faction button; it closes the pause menu and opens the lobby
   (`TBD_LobbyScreen.OpenFromPause`).
3. The top bar shows the mission id in a mono face as the title, the "Lobby" tab active, and the
   player's identity chip; the bottom bar holds "Lock Lobby" and the primary "Ready & Continue".

### Picking a seat

1. The "Factions" column lists each faction with its role chip (for example `DEFENDING`) and a
   claimed-of-total count (`0 / 92`), then a Spectators row.
2. Choosing a faction fills the "Roles" column with its squad cards: callsign and vehicle chips, a
   filled count (`3/8`) and a fold chevron. Each seat row shows the role in capitals, weapon chips,
   trait chips (`MED` in the success tint), the holder, and a status chip: `Unslotted` for an open
   seat, `DEAD` for a spent life. A dead seat takes no clicks.
3. The first click on a seat selects it and shows its kit. A second click on the selected open seat
   claims it (`TBD_LobbyCatalog.Claim`); a second click on the player's own seat releases it
   (`Release`). Focus opens on the player's own seat, else the first seat.
4. The "KIT INSPECTOR" column shows the seat line (`8: RIFLEMAN (AT)`), a 3D preview of the kit on
   a character, and cards for gear, weapons (mounted attachments and ammunition), grenades,
   gadgets, tools, medical and miscellaneous items. With no seat chosen it reads "Pick a slot to
   inspect its kit."; when the preview cannot be built it captions "PREVIEW UNAVAILABLE".

### Bottom bar

1. "Lock Lobby" flips to "Unlock Lobby" in the warning tint and back.
2. "Ready & Continue" flips to "Ready (Waiting for Admin)" in the success tint and back.
3. Each press logs one line (`[TBD][lobby] LOCK LOBBY -> …`, `[TBD][lobby] READY -> …`); neither
   reaches the server. The player deploys from the briefing's "Ready & Continue".

### Known discrepancies

- The screen shows a server-owned roster (`TBD_LobbyCatalog.c`) — but the catalog is built from
  `TBD_LobbyMock` (`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`), because no script calls
  `TBD_LobbyCatalog.Set()`, so claims and releases change only this client's copy.
- The roster wire (`TBD_LobbyClient.Request`, `Claim`, `Release` and `Deploy` in
  `TBD_LobbyClient.c`) is complete on both sides — but no script calls it.
- "Lock Lobby" and "Ready & Continue" read as session controls (`TBD_LobbyScreen.c`) — but they
  only toggle their labels; the server holds no lobby lock and no lobby ready state.

## Data

The screen makes no HTTP call and sends no RPC. The wire it is meant to use, listed in the Lobby
README's [roster wire](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/README.md#the-roster-wire):

- `TBD_RpcAsk_LobbyRoster`, `TBD_RpcAsk_ClaimSlot(string)`, `TBD_RpcAsk_ReleaseSlot` and
  `TBD_RpcAsk_Deploy` (`TBD_LobbyController.c`): the server takes the caller from
  `GetPlayerId()`, applies the action through `TBD_SpawnManager` (`ClaimSlot`, `ReleaseSlot`,
  `DeployPlayerEx`), and answers with `TBD_RpcDo_LobbyRoster`: the whole roster as it stands after
  the action, parsed from `TBD_SpawnManager.BuildSlotRoster`, with a `V` verdict record naming
  the action and why it failed. The client replaces its roster with each reply, so a refused claim
  reverts in the message that explains it.
- A deploy the platform is still deciding answers `AUTHORIZING`, one it cannot authorize now
  `UNAUTHORIZED` (`TBD_LobbyServiceDeploymentAuthorization.c`).
- `TBD_SessionSelection` (`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionSelectorData.c`)
  carries the Mission Selector's pick to the lobby's title.

## Design

- As built: a dock shell with a 320 px Factions column, a 500 px Roles column and the kit inspector
  filling the rest, between the 56 px top bar and the 64 px bottom bar; rounded glass panels
  painted from `TBD_UITheme` tokens. The
  [layouts README](/apps/mod/tbd-framework/UI/layouts/Session/Lobby/README.md) gives the geometry.
- Design target: the four Stitch sets in
  [visual_references](/documentation_v2/mod/tbd-framework/UI/lobby/visual_references/README.md)
  (sidebar, ORBAT panel, slot kit inspector, bottom bar), design-phase references. Differences:
  the faction column's `VoiceDock` stays empty, since no voice panel is built; the bottom bar's
  two actions only toggle.
- Starting point: the Arma 3 "ROLE ASSIGNMENT" screen in
  [reference_screenshots](/documentation_v2/mod/tbd-framework/UI/lobby/visual_references/reference_screenshots/README.md).
  TBD keeps its side → squad → seat drill-down, per-side counts, the player's own name on a claimed
  seat and a host lock, and drops its amber header rail, the per-seat AI toggle and `DISABLE AI`,
  leader-first seat locking, the ping-sorted player table and `BACK` / `OK`; the kit inspector has
  no counterpart there.

## Open work

- [T-1085 — Feed the pre-game screens live catalogs instead of mocks](/.ai/tickets/T-1085.toml)
  (idea, no plan): the catalog reads the server's roster through `TBD_LobbyClient`, so claims,
  releases and deploys reach `TBD_SpawnManager`.
- [T-946.58 — T-139 kit preview is empty on a dedicated server](/.ai/tickets/T-946.58.toml) (idea,
  no plan): the kit preview on a client that is not also the server; its summary names an older
  code path (`TBD_MissionLoader.GetSlotById`) that the screen no longer reads.
- [T-181.16 — Two-client dedicated-server event loop E2E](/.ai/tickets/T-181.16.toml) (queued, no
  plan): a human playtest of connect, slot, brief and deploy.

## Decisions

- The lobby is a dock shell whose panels mount into named docks: each panel owns its widgets and
  the screen owns only the wiring, so the briefing reuses the roster and kit inspector.
- A seat is claimed by a second click, not a separate button: selecting first shows the kit, so a
  player sees what they carry before committing.
- A dead seat stays visible with `DEAD` and cannot be clicked: one life means a spent seat is part
  of the ORBAT's truth, not a free slot.
- The server answers every roster request with the whole roster and a verdict, never a delta: the
  client cannot drift from `TBD_SpawnManager`.
