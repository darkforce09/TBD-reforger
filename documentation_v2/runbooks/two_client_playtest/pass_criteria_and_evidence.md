**Status:** live

# Judge and capture the playtest

What counts as a PASS for the two playtest tickets — the event loop and the Arsenal's end-to-end
gate — which session step produces each piece of evidence, how to capture everything once
before a restart deletes it, the greps that classify a failure, and the mod's log lines in one
table. Capture during S16 of
[Session: live round](/documentation_v2/runbooks/two_client_playtest/session_live_round.md), while
the server still runs; the capture takes about ten minutes.

## Prerequisites

- The session's server still running, and `$LOG` pointing at its newest `console.log`.
- The second player able to send their client log.

## The PASS lists

Both tickets close only when every item of both lists passes; one FAIL closes neither. The
"Reachable" column records what the current code can produce: the lobby and briefing screens
render mock catalogs and no lobby click reaches the server
([Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)), so
the items marked "no" stay open until the screens read the server's roster.

**The event loop (T-181.16)**

| # | Item | Evidence | Step | Reachable |
|---|---|---|---|---|
| 1 | two real clients on the dedicated server at once | two authentication lines; both in the roster | S2, S5 | yes |
| 2 | the lobby opened on both clients with readable rows | two screenshots | S7 | yes (mock rows) |
| 3 | claiming a seat worked and did not flicker | `[TBD][Spawn] claim player=… slot=…` | S8 | no |
| 4 | a contended claim was refused and named the holder | `claim rejected … (held by another player)` and a screenshot | S8 | no |
| 5 | releasing a seat worked in the lobby | `[TBD][Spawn] release player=…` | S8 | no |
| 6 | the briefing opened on both clients, side-specific | two screenshots | S9 | opens: yes; side-specific: no (mock pages) |
| 7 | deploy put both players at the authored X, Z and heading | `assigned slot`, `possess request accepted`, `[TBD][Spawn] slot=… heading=…` | S10 | yes |
| 8 | objectives, markers and the play-area warning observed | screenshots and the play-area chat line | S11 | yes |
| 9 | one terminal death; the dead player could not come back | `KILLED — one life spent` and `deploy DENIED … one life spent` | S12 | yes |
| 10 | spectator worked for the dead player | a screenshot | S12 | yes |
| 11 | an admin respawned them and they came back dressed | `back in the world, life restored`, `rematerialized body` and a screenshot | S13 | yes |
| 12 | the round reached END | `[TBD][Stage] … -> END` | S15 | yes |

**The Arsenal end-to-end gate (T-068.14)**

| # | Item | Evidence | Step | Reachable |
|---|---|---|---|---|
| P1 | a loadout authored on a character slot in the [Mission Creator](/documentation_v2/glossary.md#mission-creator), version saved | app screenshot and the version number | [stack](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md) step 11 | yes |
| P2 | `GET /api/v1/missions/<id>/artifacts/<artifact>/document` answers 200 with `slot.loadout.gear` and `slot.loadout.cargo` | the `curl` of stack step 14 | stack step 14 | yes |
| P3 | the server loaded the deployed artifact, verified | `[TBD][Mission] loaded … source=platform` (or `source=cache`) | S1 | yes |
| P4 | the slot was claimed through the lobby picker, not an automatic deploy | a `claim player=…` line | S8 | no; S10's `ready player=… → path=slot result=DEPLOYED` shows the player's own deploy |
| P5 | the player spawned on the slot with the right kit alias | `bound player … to slot … body (kit …)` | S10 | yes |
| **P6** | **a screenshot of the human player wearing the authored loadout**: primary, uniform, vest, helmet, and the cargo in the authored containers | **the screenshot; nothing else counts** | S10 | yes |
| P7 | `[TBD][Loadout][Slot] … loadout pass complete gear=x/x cargo=y/y` for that slot, and the player's seat on the same slot id | the log excerpt; the `assigned slot <id>` line stands in for a claim | S10 | yes |
| P8 | no `[TBD][Loadout][TestNPC]` line: P6 shows a player | `grep -c TestNPC "$LOG"` prints `0` | S10 | yes |
| P9 | the no-garment degrade path ran and behaved as described | the `DEGRADED`, `SHORTFALL` and audit lines | S10 | yes, with `bridgehead-at-levie` |

The sign-off form is the template in
[the Arsenal gate's specification](/documentation_v2/tickets/specs/t068_14_phase2_e2e_gate.md#sign-off-template).
On a full PASS:

```bash
cargo xtask ticket advance-slice T-068
```

```bash
cargo xtask ticket ship T-068
```

```bash
cargo xtask ticket ship T-181
```

```bash
cargo xtask ticket sync
```

## Capture the evidence

A restart of `cargo xtask mod playtest` deletes `<run dir>/profile/logs`, so capture before
anything restarts, and capture once: a bug should never need reproducing.

1. Make the evidence folder.

   ```bash
   EVIDENCE="$HOME/tbd-playtest/evidence/$(date +%F-%H%M)" && mkdir -p "$EVIDENCE"
   ```

   Expected: no output; the folder exists.

2. Copy the server log and its error log.

   ```bash
   cp "$LOG" "$(dirname "$LOG")/error.log" "$EVIDENCE/"
   ```

   Expected: `console.log` and `error.log` in the folder.

3. Copy your client log. The client runs under Proton, app 1874880; compare client and server by
   the game version string both logs print, never by Steam build ids, which differ between the
   client and server apps.

   ```bash
   cp "$(ls -td "$HOME"/.local/share/Steam/steamapps/compatdata/1874880/pfx/drive_c/users/steamuser/Documents/My\ Games/ArmaReforger/logs/logs_* | head -1)/console.log" "$EVIDENCE/client-1.log"
   ```

   Expected: `client-1.log` in the folder. The second player sends theirs from the same path.

4. Copy the mission the server ran and its config.

   ```bash
   cp "$HOME/tbd-playtest/profile/profile/TBD_MissionArtifactCache/document.json" "$HOME/tbd-playtest/server.json" "$EVIDENCE/"
   ```

   Expected: the verified artifact bytes and the rendered `server.json` in the folder.

5. Add the screenshots: every screen that was wrong, one that was right for contrast, and the
   `#tbd audit` output (it is chat, not a file).

To file a finding, add a ticket with the evidence folder in its summary; `ticket add` creates it
with status `idea`. Copy the folder under `.ai/artifacts/` when the ticket should cite it by
repository path.

```bash
cargo xtask ticket add "<what failed>" --summary "<evidence folder and the failing line>"
```

Expected: the new ticket id; then `cargo xtask ticket sync`.

## Verify

```bash
ls "$EVIDENCE"
```

Expected: `console.log`, `error.log`, `client-1.log`, `document.json`, `server.json` and the
screenshots.

## Greps that classify a failure

| Question | Command |
|---|---|
| did the game mode wire up | `grep -E '\[TBD\] roll-call' "$LOG"` |
| a class that does not resolve (the engine's own diagnostic) | `grep -E "WORLD \(E\): Unknown class" "$LOG"` |
| any mod error | `grep -E '\(E\):.*TBD\|\[TBD\].*(REFUSED\|FAILED\|NAKED\|HALF-DRESSED\|MISSING)' "$LOG"` |
| a screen that cannot open | `grep -E "Menu preset '.*' not found" "$LOG" "$(dirname "$LOG")/error.log"` |
| the loadout chain | `grep -E '\[TBD\]\[Loadout\]\|\[TBD\]\[Slots\]' "$LOG"` |
| the spawn chain | `grep -E '\[TBD\]\[Spawn\]\|SpawnManager:' "$LOG"` |
| the stages | `grep -E '\[TBD\]\[Stage\]\|\[TBD\] Stage' "$LOG"` |
| the old Workshop build ran | `grep -c '\[TBD\]\[' "$LOG"` prints `0` |

(In the table the pipes are escaped; type `|` in the shell.) Do not sort errors by message text
and let the rest pass: mod failures can carry neither a `[TBD]` tag nor a path, such as
`Instance of class TBD_SpawnManager is null` or a `Virtual Machine Exception` with no `(E)`
marker. When in doubt, keep the whole log. The staging log verdict also reads a local log:
`cargo xtask mod remote-logs --file "$LOG"`
([boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)).

## The log lines

Follow them all in one terminal:

```bash
tail -f "$LOG" | grep --line-buffered -E '\[TBD\] roll-call|\[TBD\]\[Mission\]|\[TBD\]\[Validate\]|\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Spawn\]|\[TBD\]\[Stage\]|\[TBD\] Stage|\[TBD\]\[Admin\]|\[TBD\]\[JIP\]|\[TBD\]\[Lobby\]|\[TBD\]\[Deployment\]|\[TBD\]\[Roster\]|Unknown class|Menu preset'
```

| Line | Printed by | Means |
|---|---|---|
| `[TBD] roll-call: …=ok` nine times | `TBD_FrameworkManager.PrintComponentRollCall` | every component on `TBD_GameMode.et` instantiated |
| `[TBD][Mission] loaded id=… source=platform` | `TBD_Log.MissionLoaded` | the deployed artifact, fetched and SHA-256 verified; `cache` is the same artifact from the profile cache, `last-verified-cache` the fallback when the deployment could not be read |
| `[TBD][Validate] mission result=PASS` | `TBD_Log.ValidationResult` | the mission is loadable |
| `[TBD][Slots] loadout settle complete … 0 unplayable, M with a shortfall — spawn open` | `TBD_SpawnManager.TickLoadoutSettle` | the lineup is playable; `M` > 0 means some of it is not as authored |
| `[TBD][Slots] loadout SHORTFALL on M of N slot(s)` | `TBD_SpawnManager.TickLoadoutSettle` | not a stop; names the slots carrying less or elsewhere |
| `[TBD][Slots] loadout delivery REFUSED at spawn boundary … UNPLAYABLE` | `TBD_SpawnManager.TickLoadoutSettle` | stop: nobody leaves LOADING; names the slot and the item |
| `[TBD] Stage → LOBBY` | `TBD_FrameworkManager.SetStage` | the lobby is open |
| `[TBD][Spawn] LOBBY: no bodies this phase — …` | `TBD_SpawnManager.OnStageChanged` | nobody spawns until BRIEFING |
| `[TBD][Spawn] claim player=N slot=K` | `TBD_SpawnManager.ClaimSlot` | a seat was taken on the server |
| `[TBD][Spawn] claim rejected … (held by another player)` | `TBD_SpawnManager.ClaimSlot` | a contended claim refused |
| `[TBD] SpawnManager: assigned slot … to player …` | `TBD_SpawnManager.AssignSlotForPlayer` | seated by roster or round-robin |
| `[TBD][Spawn] slot=… Y=… jsonY=… heading=…` | `TBD_SpawnManager.SpawnSlotBody` | the authored transform applied |
| `[TBD][Loadout][Slot] … loadout pass complete gear=x/x cargo=y/y` | `TBD_LoadoutEquipHelper` | the Arsenal gate's line |
| `[TBD][Loadout][Slot] … worn-audit jacket=1 pants=1 boots=1` | `TBD_LoadoutEquipHelper.ReportWornAudit` | actually dressed, not only "equip OK" |
| `[TBD][Loadout][Slot] … NAKED …` or `HALF-DRESSED …` | `TBD_LoadoutEquipHelper.ReportWornAudit` | the nakedness guard fired |
| `[TBD] SpawnManager: bound player … to slot … body (kit …)` | `TBD_SpawnManager.DeployPlayerInternal` | the body handed over |
| `[TBD][Spawn] player=N possess request accepted` | `TBD_SpawnManager.PossessSlotBody` | in the world |
| `[TBD][Spawn] ready player=N … → path=slot result=DEPLOYED` | `TBD_SpawnManager.DeployOnReady` | the briefing's "Ready & Continue" deployed the player |
| `[TBD][Spawn] player=N KILLED — one life spent` | `TBD_SpawnManager.OnPlayerKilled` | a terminal death |
| `[TBD][Spawn] deploy DENIED player=N … one life spent` | `TBD_SpawnManager.DeployPlayerInternal` | one life enforced |
| `[TBD][Admin] respawn player=N by=… — back in the world, life restored` | `TBD_SpawnManager.FinishAdminRespawn` | the admin respawn worked |
| `[TBD][Slots] rematerialized body for slot … — freshly dressed from mission JSON` | `TBD_SpawnManager.DeployPlayerInternal` | the respawn applied the loadout again |
| `WORLD (E): Unknown class '<name>'` | the engine | a prefab component's class does not resolve |
| `GUI (E): Menu preset '<name>' not found!` | the engine | a screen cannot open |

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `cp: cannot stat` on the client log | no client log under that Proton prefix, or another Steam library | find the `ArmaReforger/logs` folder under the client's compatdata |
| the evidence has no `console.log` from the session | the server was restarted before the capture | nothing to recover; capture before any restart next time |
| `grep -c TestNPC` is not `0` | the test-NPC loadout harness ran | the P6 screenshot may show an NPC; take it again on a player |

## Related

- [Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md)
  and [Session: live round](/documentation_v2/runbooks/two_client_playtest/session_live_round.md)
  — the steps that produce the evidence.
- [Known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md) — why
  items 3 to 5 and P4 stay open.
- [Event mod program](/documentation_v2/tickets/specs/t181_event_mod_program.md) — the frozen
  specification behind the event loop list.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
