**Status:** live

# Product roadmap

The product items of the TBD Reforger platform that are planned and not yet built, grouped by area:
the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator),
[missions](/documentation/glossary/g_to_m.md#mission) and [operations](/documentation/glossary/n_to_z.md#operations), the [mod](/documentation/glossary/g_to_m.md#mod)
and game runtime, the map and terrain, and the platform and its tooling. The operator curates the
list: an item without a [ticket](/documentation/glossary/n_to_z.md#ticket) stays only while the
operator keeps it, and the open product questions close with a ticket or a decision.

## How to read it

Each row gives the item, what it adds, its ticket's slug and its status, checked against the
ticket's status in the central ticket manager (`ttm --project reforger show <ticket>`), which also
holds the ticket's spec and plan:

- **open (idea, queued or ready)**: a ticket plans the item.
- **deferred**: a ticket holds the item, parked by the operator.
- **no ticket (candidate for strike)**: a product plan names the item and no ticket covers it; the
  operator keeps it, and a ticket follows, or strikes it.

Shipped and cancelled items leave the live list and are named under [Recently landed](#recently-landed).
The Source column names where the item comes from:

- **build plan §**: a section of the master build plan, the platform's first product plan, kept
  in the commit history.
- **milestones**: the mod milestones and the first milestone's announcement, kept in the commit
  history.
- **backlog**: the north-star backlog, the spec of the ticket `route-planner-tool` in `ttm`, which
  covers the backlog tickets from `route-planner-tool` to `mc-shell-layout-polish`.
- **feature docs**: the Open work of a live feature document, such as the
  [Mission Creator roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md),
  which also lists the editor's usability and defect tickets that this roadmap leaves out.

## Mission Creator

| Item | What it adds | Ticket | Status | Source |
|---|---|---|---|---|
| Realtime collaborative editing | several authors edit one mission live, over a websocket that relays document updates and presence | `realtime-collaborative-editing` | open (ready) | backlog, feature docs |
| Visual mission diff | an entity-level diff between two versions, for review before publishing | `multiplayer-mc-visual-git` | open (ready) | backlog |
| Route planner tool | waypoints snapped to the road graph, with the route's length and elevation profile | `route-planner-tool` | open (ready) | backlog |
| Command palette | one searchable palette over every editor command | `command-palette-over-editor` | open (ready) | feature docs |
| Typed per-side objectives | objectives become typed, placed, per-side entities with attributes | `typed-side-objectives-attributes` | open (ready) | feature docs |
| Shell layout polish | toolbelt placement, grouping in the Attributes dialog, and hidden stub tools | `mc-shell-layout-polish` | open (ready) | backlog |
| Procedural slot naming | generated [slot](/documentation/glossary/n_to_z.md#slot) display names from word lists, with a manual override | `procedural-slot-naming` | open (ready) | backlog |
| Building floor selector | placement on a chosen floor of a building | `building-floor-selector` | open (ready) | backlog, feature docs |
| Virtual Arsenal close-out | the [arsenal](/documentation/glossary/a_to_f.md#arsenal) program closes with a human two-client sign-off from an editor loadout to a dressed player | `virtual-arsenal` | deferred | feature docs |
| Guided mission wizard | a linear flow, one decision per screen, that builds a mission from a template, a terrain, zones, faction presets and player counts, with the [ORBAT](/documentation/glossary/n_to_z.md#orbat) generated | none | no ticket (candidate for strike) | build plan §B2 |
| Developer mode | a raw JSON editor with schema completion beside the map | none | no ticket (candidate for strike) | build plan §B2 |
| Mission cloning | a copy of a mission as the start of a new one | none | no ticket (candidate for strike) | build plan §B2 |
| Second mission template | a meeting-engagement template beside attack and defend | none | no ticket (candidate for strike) | build plan §9 |
| Decoration layers | a layer library with footprint previews, and a step that attaches reusable layers of hand-placed props to a mission | none | no ticket (candidate for strike) | build plan §12 |

## Missions and operations

| Item | What it adds | Ticket | Status | Source |
|---|---|---|---|---|
| [After-action review](/documentation/glossary/a_to_f.md#after-action-review) replay | a match's timeline served by the [API](/documentation/glossary/a_to_f.md#api) and a map scrubber with play, pause and speed, linked from the deployments page | `3d-aar-ocap-style` | open (ready) | build plan §B6, backlog |
| Combat, medical and vehicle telemetry events | a telemetry-events schema and an ingest that stores the events the replay plays | `website-platform-events-telemetry.combat-medical-vehicle-telemetry` | open (ready) | build plan §A10 |
| Live telemetry bridge | live game-server events in the telemetry ingest, the feed a ticker of captures, kills and score during an [event](/documentation/glossary/a_to_f.md#event) would read | `live-game-server-telemetry` | deferred | build plan §B5 |
| Mission modset manager | a mod set per mission, alias checks against it at export, and a join gate on a mismatch | `mission-modset-manager` | open (ready) | build plan §7, backlog |
| Combat figures on the [service record](/documentation/glossary/n_to_z.md#service-record) | kills, deaths and K/D on the deployments page, which the API already sends | `add-kills-deaths-k` | open (idea) | feature docs |
| Discord platform rework | a bot for slot confirmations, reminders before an event and the replay link after it, and a channel and role layout | `discord-platform-rework` | deferred | build plan §B3, backlog |
| Mission planner workspace | a whiteboard where each planner draws over a published mission, beside the unchanged mission | none | no ticket (candidate for strike) | feature docs |
| Sign-up windows and check-in | sign-up open and close times per event, and a check-in window that releases the claims of no-shows | none | no ticket (candidate for strike) | build plan §B3 |
| Mission library ratings | members rate missions in the library | none | no ticket (candidate for strike) | build plan §9 |
| Custom mission mods on event pages | a mission's authoring tier in the library, and a "download required" badge with the workshop link and a publish countdown on the event page | none | no ticket (candidate for strike) | build plan §12 |

## Mod and game runtime

| Item | What it adds | Ticket | Status | Source |
|---|---|---|---|---|
| Staging soak and golden-mission smoke | a pinned game and mod version soaked on the staging server, with a golden mission that exercises the spawner, zones, loadouts and results | `staging-soak-golden-mission` | open (queued) | build plan §8, milestones |
| Two-client event loop on a dedicated server | a human playtest of connect, slot, briefing, deploy, death and admin respawn with two real clients | `tbd-framework-arma-3.two-client-dedicated-server` | open (queued) | feature docs |
| Editor-to-player loadout check | a human sign-off from a Mission Creator loadout to a dressed player in the game | `virtual-arsenal.phase-2-e2e-gate` | open (queued) | feature docs |
| Slot identity reaches the game | each seat's identity fields reach the compiled mission and the mod | `slot-identity-reaches-wire` | open (queued) | feature docs |
| Vehicle roster reaches the game | placed vehicles and their crews reach the compiled mission | `vehicle-roster-reaches-game` | open (queued) | feature docs |
| Mission client payload budget | a console-safe size budget for the compiled mission, reported as a compile diagnostic | `mission-client-payload-budget` | open (ready) | backlog |
| Live data on the pre-game screens | the mission selector, lobby, briefing and players panel read the server's data instead of mock catalogs | `feed-pre-game-screens` | open (idea) | feature docs |
| Mission runtime gaps | readers for mission parameters that have none, group AI defaults, audio for late joiners | `close-mission-runtime-gaps` | open (idea) | feature docs |
| Debrief kill credit and auto-advance | the team-kill rule for the debrief scoreboard, and a DEBRIEF countdown that advances on its own | `decide-debrief-team-kill` | open (idea) | feature docs |
| Player help requests to admins | a help request a player sends to the admins in game | `decide-whether-build-admin` | open (idea) | feature docs |
| Voice integration | the TBD Voice client and the TBD-Radio bridge mod on the [bridge contract](/documentation/contracts/definitions/bridge_messages.md): push-to-talk per net, the dead kept apart from the living, casters listening in | none | no ticket (candidate for strike) | build plan §5, milestones |
| Manual radio tuning | an in-game tuning control, gamepad first, for re-tasking nets during a mission | none | no ticket (candidate for strike) | build plan §5 |
| More objective types | `intel_grab` and `escort` objectives beside capture, destroy and hold | none | no ticket (candidate for strike) | build plan §A6 |
| Elimination that waits for the wounded | faction elimination that counts unconscious players as alive while they can be revived | none | no ticket (candidate for strike) | build plan §A6 |
| Walk-ins and no-show release | open slots a player without a claim may take, and claimed slots that unlock a set time after the briefing starts | none | no ticket (candidate for strike) | build plan §A4 |
| Spectator slots for casters | a spectator slot type for casters and admins, apart from the dead | none | no ticket (candidate for strike) | build plan §A4 |
| Locked supply boxes | arsenals and equipment boxes locked unless the mission spawns one on purpose | none | no ticket (candidate for strike) | build plan §A5 |
| More admin powers | teleport a player, and end the round with a named winner | none | no ticket (candidate for strike) | build plan §A11 |
| Schema version handshake | the mod advertises the mission schema versions it runs, and the platform refuses to deploy a mission it cannot | none | no ticket (candidate for strike) | build plan §2.2 |
| Console release gate | every screen navigable by gamepad, and a console checklist per release tested on Xbox and PlayStation | none | no ticket (candidate for strike) | build plan §A8, §A12 |
| Workshop release channels | development, staging and release copies of each addon on the Workshop | none | no ticket (candidate for strike) | build plan §A12 |
| In-game build mode | a build session on the staging server whose placed props export as a decoration layer, and the same layer format from a [Workbench](/documentation/glossary/n_to_z.md#workbench) plugin | none | no ticket (candidate for strike) | build plan §12 |
| Mission maker SDK | custom-objective, stage-hook, telemetry and marker interfaces for a custom mission mod built on the framework, with a sample mission | none | no ticket (candidate for strike) | build plan §12 |

## Map and terrain

| Item | What it adds | Ticket | Status | Source |
|---|---|---|---|---|
| Map visualization program | the placement audits, the world-object field contract, and hover, inspect, filter and legend for world objects | `map-visualization-program` | open (ready) | feature docs |
| Arland world objects | Arland's object data, so a second terrain carries a full map | `arland-has-manifest-no` | open (ready) | build plan §9, feature docs |
| Terrain-sized map layers | the grid, basemap, peaks and forest sized from each terrain instead of Everon's | `derive-map-grid-basemap` | open (idea) | feature docs |
| Chunk-free map storage | one object container with a spatial index read by range fetches in place of the chunk files (its child slices queued), and the finished compressed-JSON cutover (its child slice `map-binary-storage-hybrid.finish-gz-cutover-left` ready) | `map-binary-storage-hybrid` | open (queued) | feature docs |
| Terrain base and sparse deltas | a binary terrain base with sparse prop deltas for a million or more map objects | `terrain-base-sparse-deltas` | deferred | feature docs |
| Terrain elevation export automation | an automated elevation export for each terrain | `terrain-dem-export-automation` | deferred | feature docs |
| Map assets pinned to terrain versions | map assets versioned per terrain version, with a calibration mission that catches coordinate drift | none | no ticket (candidate for strike) | build plan §10 |

## Platform and tooling

| Item | What it adds | Ticket | Status | Source |
|---|---|---|---|---|
| One-command self-host setup | one xtask command that checks the dependencies, writes the environment file, migrates and seeds the database and pulls the map assets | `one-command-self-host` | open (ready) | backlog |
| Wiki Markdown renderer | doctrine wiki pages rendered from Markdown | `wiki-markdown-renderer` | deferred | feature docs |
| Rich text in the content manager | a rich text editor for announcements and other CMS content | `cms-rich-text-editor` | deferred | feature docs |
| Server control and [RCON](/documentation/glossary/n_to_z.md#rcon) | a live server control panel wired to an RCON backend | `server-control-rcon-api` | deferred | feature docs |
| Multi-server picker | a choice among several game servers in the server intel views | `multi-server-picker` | deferred | build plan §9, feature docs |
| Scenario to mission rename | code, data and mod identifiers that say scenario say mission | `rename-scenario-mission-across` | deferred | feature docs |
| Supporter payments | a supporter subscription and paid supporter-only event nights through Stripe, with an entitlement check when a player joins | none | no ticket (candidate for strike) | build plan §B4 |
| License matrix | a reviewed record of every mod's license and commercial-use status, and a modset policy for monetised servers | none | no ticket (candidate for strike) | build plan §7 |
| Several game identities per account | one website account linked to game identities on several platforms | none | no ticket (candidate for strike) | build plan §B1 |
| Game server environments | separate development, staging and production game-server instances, with the mission, backend and credential injected per instance | none | no ticket (candidate for strike) | build plan §8 |
| Uptime monitoring | uptime checks that alert the admin Discord channel | none | no ticket (candidate for strike) | build plan §8 |
| Telemetry storage lifecycle | retention rules for stored telemetry and replays | none | no ticket (candidate for strike) | build plan §8 |
| Preview deploys | a preview deployment of the website per change | none | no ticket (candidate for strike) | build plan §8 |
| White-label platform | the platform run by other communities under their own name | none | no ticket (candidate for strike) | build plan §9 |

## Open product questions

The feature writers raised these and no ticket settles them; each needs a decision, then a ticket
or a strike.

1. **Mission armory and slot loadouts.** The [armory](/documentation/glossary/a_to_f.md#armory)
   (`GET` and `PUT /api/v1/missions/{id}/armory`, `mission_armory.rs` in
   `crates/api/api_missions/src/handlers/`) is a separate list that a write replaces whole;
   nothing derives it from the slots' loadouts. Does an Arsenal edit update the armory, or do the
   two stay apart? No ticket.
2. **Mission planner.** No ticket covers the planned workspace, which has no code yet
   ([design notes](/documentation/crates/frontend/workspaces/planner/mission_planner.md)). File one,
   or strike the planner row above.
3. **Where the replay lives.** `3d-aar-ocap-style`'s plan puts the replay among the routed pages and
   names code paths that no longer exist, while the after-action review is planned as a workspace
   crate under `crates/frontend/workspaces/` ([after-action review
   notes](/documentation/crates/frontend/workspaces/aar/after_action_review.md)). The [mod
   design](/documentation/mod/tbd-framework/mod_design.md) also records full after-action recording
   as deferred by the operator while `3d-aar-ocap-style` is ready: which one stands?
4. **Team kills.** The debrief scoreboard counts team kills
   (`mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:908-919`),
   while the comment above it (`:897`) says they are ignored. Should they count?
   `decide-debrief-team-kill` (idea).
5. **DEBRIEF auto-advance.** The DEBRIEF stage never counts down or advances on its own. Should
   it? `decide-debrief-team-kill` (idea).
6. **AI skill.** Seats with waypoints run AI (`TBD_WaypointRuntime.ShouldEnableAIAtSpawn`, in
   `mod/tbd-framework/Scripts/Game/TBD/Systems/AI/`), but neither the mission schema nor the mod has
   a skill value; the Eden gap row is ATTR-FIELD-OBJ-SKILL in the [Eden gap
   analysis](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md).
   Is AI skill authorable? No ticket.
7. **Steam app id.** xtask reads the dedicated server's build from app id 1874900
   (`tools/commands/remote_debugging/src/debug/direct_join.rs:98`) and tells the operator to install
   app id 1890870 (`tools/commands/mod_operations/src/compile/execution.rs:132`). Which is right?
   `decide-dedicated-server-steam` (idea).
8. **Realtime collaboration.** `realtime-collaborative-editing` is ready, yet the editor keeps one
   writer tab per mission and the backlog planned a solo first version; `multiplayer-mc-visual-git`
   builds on `realtime-collaborative-editing`, and its notes place the diff page in a
   `pages/public/` folder the frontend does not have. Is co-editing wanted now?

## Recently landed

The items of the product plans that have shipped, or that the operator cancelled, leave this
list; the ticket manager keeps each ticket's record. From the plans above:

- Shipped: the game server's fetch of the compiled mission (`spawn-transform-parity-mod`), with its
  verified local cache; slot roster enforcement and the production slot picker
  (`slot-roster-enforcement-production`), the capture win condition (`capture-win-condition`),
  results posted to the platform (`results-post-backend`), mission upload and validation
  (`mission-upload-validation-ui`), the event ORBAT and identity linking
  (`event-orbat-identity-linking`, `identity-linking-last-mile`), the framework's loadouts, safe
  start, boundary and admin commands (`framework-mvp-remainder`), timed objectives
  (`ofcr-timed-objectives`), in-game markers (`game-marker-system`), the lobby loadout preview
  (`lobby-loadout-visual-preview`), and radio nets tuned from the mission's radio plan
  (`emit-radioplan-unlocks-seven`).
- Cancelled: custom compositions (`custom-compositions`), the connection and sync UI
  (`connection-sync-ui`) and the latest-everything dependency upgrade
  (`dependency-toolchain-upgrade`).

## Related documentation

- [Mission Creator roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md)
  — the editor's tracks, defects and deferred work in full.
- [Mod design](/documentation/mod/tbd-framework/mod_design.md) — the mod's spine, deferrals and
  open work.
