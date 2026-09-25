**Status:** live

# Run the playtest session: join to deploy

Session steps S1 to S10 of the two-client playtest: the server settles, both players join, the
first becomes an admin, the identity gate is checked, the optional
[event](/documentation_v2/glossary.md#event) seating is exercised, the lobby and briefing screens
open, and both players deploy into a [slot](/documentation_v2/glossary.md#slot) wearing the
authored loadout. It takes about 25 minutes with both players present; S11 to S16 follow in
[Session: live round](/documentation_v2/runbooks/two_client_playtest/session_live_round.md).

## Prerequisites

- A server from [Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md),
  and `$LOG` pointing at its newest `console.log` (see the
  [index](/documentation_v2/runbooks/two_client_playtest/README.md)).
- Two Arma Reforger clients with `tbd-framework` loaded; the second player's client resolves the
  mod from the Workshop ([Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)).
- The mod's log lines followed in a spare terminal for the whole session:

  ```bash
  tail -f "$LOG" | grep --line-buffered -E '\[TBD\]'
  ```

The lobby and briefing screens render mock data: a seat clicked in the lobby changes only that
client's copy and never reaches the server, so the server seats players itself when they deploy
from the briefing. Steps S7 to S10 say which checks this leaves open.

## Steps

1. **S1 — the server settles.** Start the server (step 2 of
   [Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md)) and watch
   the log.

   Expected, in order:

   ```text
   [TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok
   [TBD][Mission] loaded id=<id> name='<name>' slots=<N> source=platform
   [TBD][Validate] mission result=PASS errors=0 warnings=<W>
   [TBD][Slots] Slot-<n> <slot id> (<faction>) kit <kit> at <position>                 (one per slot)
   [TBD][Spawn] slot=<id> Y=… jsonY=… surfaceY=… delta=… heading=…                     (one per slot)
   [TBD][Loadout][Slot] slot=<id> loadout pass complete gear=<a>/<b> cargo=<c>/<d>     (each dressed slot)
   [TBD][Slots] materialized <N>/<N> bodies — <A> with a JSON loadout, <B> kit-only, 0 failed
   [TBD][Slots] loadout settle complete — <N> application(s), 0 unplayable, <M> with a shortfall — spawn open
   [TBD][Stage] LOADING -> LOBBY
   [TBD] Stage → LOBBY
   [TBD][Spawn] LOBBY: no bodies this phase — claimed holders deploy on BRIEFING …
   ```

   Otherwise: a `=MISSING` roll-call entry is a component that will never run, and nothing else
   reports it; `source=last-verified-cache` means the deployment could not be read and the last
   verified artifact runs instead; `NO MISSION` or `NO MISSION YET` and `mission result=FAIL`
   keep the server in LOADING (see the playtest server's troubleshooting);
   `loadout delivery REFUSED at spawn boundary … UNPLAYABLE` is a stop and
   `loadout SHORTFALL on M of N slot(s)` is not
   ([reading the loadout lines](/documentation_v2/runbooks/two_client_playtest/headless_preflight.md#reading-the-loadout-lines)).

2. **S2 — the first player joins.** In the client: Multiplayer, Direct Join, then
   `<lan address>:2001` or the Direct Join Code from the banner.

   Expected: the world loads, the player has no body, and the lobby screen opens. The server log
   shows the engine's authentication line for the player; the client log shows
   `[TBD][Lobby] Tick ARMED after <n> attempt(s)` (`TBD_LobbyStage.c`). "No server found" means no
   room registered in this boot (see the playtest server's troubleshooting); with `ping` answering,
   it is not the firewall.

3. **S3 — make yourself an admin.** Read your identity id, stop the server and restart it with
   `--admin` (steps 3 to 5 of
   [Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md)); then join
   again with the new Direct Join Code and type in chat:

   ```text
   #tbd help
   ```

   Expected: `TBD: #tbd missions | mission <n> | backend <url> [token] | refresh | validate | dead | respawn <playerId> | deploy <playerId> | stage [next|<NAME>] | safestart [status|go|<seconds>] | identity [status|override <phrase>|enforce] | audit | menu`,
   the reply to any subcommand the handler does not know (`TBD_AdminCommands.c`); `#tbd` alone
   lists the [missions](/documentation_v2/glossary.md#mission) the platform lets this server
   deploy. `TBD: admin only.` means you are not in `game.admins[]`: the only admin source
   `TBD_AdminService.IsAdmin` reads is vanilla's `SCR_PlayerListedAdminManagerComponent`, filled
   from `game.admins[]` at connect, which exists only in `-config` mode; `#login` with
   `passwordAdmin` does not add you. Without an admin, S13 cannot run. The boot's
   `BACKEND (W): !!! JsonApi Array name="admins" found in JSON …` warning comes from a scripted
   mirror of the config and does not concern the native admin list.

4. **S4 — the identity gate.** In chat:

   ```text
   #tbd identity status
   ```

   Expected: `TBD identity: oneLife=ON gate=OPEN connected=<n> …`. When the log instead carries
   `[TBD][Spawn] player=<n> has NO durable identity (keyMode=NUMERIC) — …`, SAFE_START and LIVE
   will be refused. A dedicated server without backend identities is misconfigured: check that
   the room registered. To waive it for this session, type the phrase verbatim and record the
   waiver in the sign-off, because deaths then do not survive a reconnect:

   ```text
   #tbd identity override I-ACCEPT-NO-ONE-LIFE
   ```

   `#tbd identity enforce` re-arms the gate.

5. **S5 — the second player joins** with the same Direct Join, and types `#tbd` in chat.

   Expected: the lobby opens for them too. A current build answers `TBD: admin only.` (they are
   not an admin); no answer at all means their client runs the old Workshop build of the mod
   ([Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)).
   A blank or collapsed lobby is a layout failure: run `cargo xtask verify ui-layouts`, and if it
   passes, capture a screenshot and the client log.

6. **S6 — event seating and deployment authorization (optional).** Needs the event bound to the
   server and the server started with `--event-mission`. In the app, open `/events/<EID>` and
   register one account for a slot; on the website, that account generates a link code from the
   top bar's "Link Arma Identity"; in game, that player types:

   ```text
   #tbd link <code>
   ```

   Expected: the reply arrives only in that player's private chat; the code is posted to
   `POST /api/v1/ingest/link-confirm` from the server and never echoed. On the next mission load
   the log shows `[TBD][Runtime] session-started session=… generation=…` (a heartbeat every 15 s
   from then on) and `[TBD][Roster] loaded event=<EID> version=2 assignments=<n> slots=<m>`, and
   the linked player is seated into their reserved slot at deploy. Each deploy into an event seat
   logs `[TBD][Deployment] authorization-requested player=… slot=…`, then `deployment-allowed`
   (the spawn continues) or `deployment-denied … reason=…` (the player reads the reason in
   private chat and stays out; an unlinked player on a reserved seat shows one). When that life
   ends: `[TBD][Deployment] life-ended player=…`, then `life-end-reported occupancy=…`.
   `[TBD][Roster] roster of event … not loaded (…)` is a refusal: 401 the credential, 403 the
   event is not bound to this server, 404 an unknown event. Every event-seat deploy is then
   refused with "the event roster has not loaded on this server yet, so event seats cannot be
   authorized - try again shortly." and a `DEPLOYMENT: …` entry in `#tbd audit`; the mod fetches
   again every 60 s and needs no restart. `assignments=0` is legal: nobody is linked, and
   everyone is seated round-robin.

7. **S7 — the lobby screen.** Both players look at the lobby.

   Expected: the top bar with the "Lobby" tab, the "Factions" column with per-faction counts and
   a Spectators row, the "Roles" column of squad cards, the "KIT INSPECTOR" with a 3D kit preview,
   and "Lock Lobby" and "Ready & Continue" in the bottom bar
   ([lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md)).
   The rows come from the mock catalog, not the mission; record that the screen rendered readable
   rows. No screen at all: look for `GUI (E): Menu preset 'TBD_UILobby' not found!` in the log
   (`MenuConfigs` in `apps/mod/tbd-framework/addon.gproj` must list both `chimeraMenus.conf`
   entries), and for `[TBD][Lobby] Start GAVE UP` in the client log, which means
   `TBD_FrameworkManager` and `TBD_LobbyComponent` are not on the same prefab.

8. **S8 — claim, contend and release.** You click a seat twice, the second player clicks the same
   seat twice, then you click your own seat again.

   Expected: each client shows the change on its own copy only; the server log shows no
   `[TBD][Spawn] claim player=` or `release player=` line, because no script sends a lobby click
   to the server. The server-side contention and release checks stay open
   ([pass criteria](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md)).

9. **S9 — the briefing.** Advance the stage as the admin:

   ```text
   #tbd stage next
   ```

   Expected: `[TBD][Stage] LOBBY -> BRIEFING`, `[TBD] Stage → BRIEFING` and
   `[TBD][Spawn] BRIEFING: deploying each claimed slot holder once …` in the log, and the briefing
   screen opening on both clients (`TBD_BriefingController` opens it on the stage change and
   closes it on any other stage). Its pages render mock content, so side-specific orders cannot be
   judged from them. Nothing leaves BRIEFING on its own: `flow.briefingSeconds` is announced, not
   enforced. A player who joins during BRIEFING logs `jip catch-up — local controller up,
   stage=BRIEFING` on their client.

10. **S10 — deploy, and the loadout.** Each player presses "Ready & Continue" on the briefing.
    The server seats a player with no claimed seat through the event roster, or round-robin in the
    mission document's slot order, so the first player to deploy takes the first free slot.

    Expected, per player, in the server log:

    ```text
    [TBD] SpawnManager: assigned slot <id> to player <n> at (<x>,<z>)
    [TBD] SpawnManager: bound player <n> to slot <key> body (kit <kit>)
    [TBD][Spawn] player=<n> possess request accepted
    [TBD][Spawn] ready player=<n> mode=<mode> stage=BRIEFING → path=slot result=DEPLOYED
    ```

    A refusal shows on the button and in `[TBD][Spawn] ready player=<n> … → … (<reason>)`;
    "bodies wait for BRIEFING — an admin advances the stage" means S9 did not run.
    `#tbd deploy <playerId>` puts a player with no body into the world as an admin.

    Then check the slot each player got against the authored one:

    ```bash
    grep -E '\[TBD\]\[Spawn\] slot=|\[TBD\]\[Loadout\]\[Slot\]' "$LOG"
    ```

    Expected: for that slot, `[TBD][Spawn] slot=<id> Y=… jsonY=… surfaceY=… delta=<small> heading=<deg>`
    matching the authored X, Z and heading (a `jsonY=… deviates … m from surfaceY=…` warning
    means a stale terrain height or a mis-authored slot), and per dressed slot lines such as
    `[TBD][Loadout][Slot] slot=<id> primary equip OK <prefab> [ … ]`,
    `[TBD][Loadout][Slot] slot=<id> cargo <prefab> x<n>/<n> -> vest`,
    `[TBD][Loadout][Slot] slot=<id> worn-audit jacket=1 pants=1 boots=1 kit=<kit> (settled on attempt <k> of <n>, <t> ms)`
    and `[TBD][Loadout][Slot] slot=<id> loadout pass complete gear=<a>/<a> cargo=<c>/<c>`. The
    tag is `[TBD][Loadout][Slot]`; nothing prints `[TBD][Loadout][Player]`.

    Then look, because no log proves it: open the inventory (`I`) and third person. The player
    entity must wear the authored uniform, vest, helmet, trousers and boots, carry the authored
    rifle with its optic and magazine, and hold the cargo in the named containers. Screenshot it;
    it is the Arsenal ticket's decisive evidence. The test-NPC harness is off by default
    (`m_bRunLoadoutTest` defaults to 0 and `TBD_GameMode.et` does not set it), so any
    `[TBD][Loadout][TestNPC]` line means something enabled it and the screenshot may show an NPC.

    With `bridgehead-at-levie` the degrade case runs on every boot: `blufor:Alpha:RFL:0` wears
    `kit:us_rifleman`, which has no backpack, and carries a cargo row aimed at `backpack`. The log
    shows `[TBD][Loadout][Slot] slot=blufor:Alpha:RFL:0 cargo:backpack DEGRADED item=… — …`, that
    slot's `loadout SHORTFALL …` line, the `[TBD][Slots] loadout SHORTFALL on <M> of 18 slot(s)`
    roll-up and a `LOADOUT: <M> slot(s) did not get the authored loadout (session opened anyway)`
    entry in `#tbd audit`. In game, the player on that slot carries the item in whatever storage
    took it, and the round runs. A refusal instead, or a `DEGRADED` line with no roll-up and no
    audit entry, is a finding.

## Verify

After S10 both players stand in the world and the server log shows two seated players:

```bash
cargo xtask mod remote-logs --file "$LOG"
```

Expected: `VERDICT: PASS — boot healthy and at least one player was seated.`, exit 0; without a
seated player it reports PARTIAL (exit 2).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| a `=MISSING` roll-call entry | a component class on `TBD_GameMode.et` did not resolve | stop; read `WORLD (E): Unknown class` in the log |
| `#tbd` gets no answer from the second player's client | their client runs the old Workshop build | [known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md) |
| `TBD: admin only.` for you | your id is not in `game.admins[]` | restart with `--admin=<id>` (S3) |
| the lobby never opens | a menu preset did not resolve, or the lobby stage gave up | the checks in S7 |
| "bodies wait for BRIEFING — an admin advances the stage" | nothing spawns during LOBBY | `#tbd stage next` (S9) |
| `[TBD][Deployment] deployment-refused … the event roster has not loaded on this server yet` | the roster fetch was refused or unanswered | read the `[TBD][Roster]` line's status; fix the binding or credential |
| no `[TBD][Loadout][Slot]` line for the player's slot | the slot is kit-only, or the check greps the wrong tag | compare with the mission's slot; grep `[Slot]` |

## Related

- [Session: live round](/documentation_v2/runbooks/two_client_playtest/session_live_round.md) —
  S11 to S16.
- [Pass criteria and evidence](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md)
  — which step proves which item, and what to capture.
- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) and
  [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the two screens as built.
- [Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md) — seating,
  deploy and one life in the mod.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
