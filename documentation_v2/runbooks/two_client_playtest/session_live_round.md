**Status:** live

# Run the playtest session: the live round

Session steps S11 to S16 of the two-client playtest: objectives, markers, the play area and radio,
the move to LIVE, one terminal death, the admin respawn, a reconnect, the end of the round, and
the evidence capture before anything restarts. It takes about 20 minutes and follows
[Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md);
the one-life rule it tests is set out in
[mod design](/documentation_v2/mod/tbd-framework/mod_design.md#2-non-negotiables).

## Prerequisites

- Both players deployed at the end of S10, and you listed in `game.admins[]` (S3).
- `$LOG` pointing at the running server's newest `console.log`.

## Steps

1. **S11 — objectives, markers, play area and radio.** Check each item in game.

   | Item | Do | Expect | If not |
   |---|---|---|---|
   | Objectives | open the map (`M`) | the mission's objectives, per side | no `[TBD][Obj]` line in the log: the registry never armed |
   | Markers | open the map | the authored markers on the vanilla placed-marker system | read the `[TBD][Markers]` lines; labels are cut to a byte budget (`CapLabel` in `TBD_MarkerData.c`), so a multi-byte label can lose its tail |
   | Play area | walk out of the boundary zone | private chat `TBD: you are outside the play area (<zone>) -- return within <n>s.`, plus ` ONE LIFE: you will be killed and cannot respawn.` on a kill zone (`TBD_PlayAreaComponent.c`) | no message: the mission authored no boundary zone (legal), or the enforcer is not ticking |
   | Radio | read the net hint and check the radio's frequency | the side's nets listed, and the carried radios tuned to them | the dev world has no `RadioManagerEntity`, so the log says `backbone: MISSING — … using script-side channel table …` (`TBD_RadioComponent.c`) and tuning runs through that table; when the hint says no tune was verified, dial by hand ([known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)) |

2. **S12 — go LIVE and die once.** As the admin, in chat, one line at a time:

   ```text
   #tbd safestart status
   #tbd stage next
   #tbd safestart go
   #tbd stage next
   ```

   Expected: damage is reported off, the stage moves BRIEFING → SAFE_START, the warmup ends early,
   and the stage moves SAFE_START → LIVE. A refusal
   `SAFE_START has no enforcement on this world (TBD_SafestartManager is missing from the game mode prefab) — …`
   means the safe start component did not resolve: use `#tbd stage LIVE` and tell the other
   player weapons are hot. A refusal naming identity sends you back to S4.

   Then the second player dies once, for good. Expected:

   ```text
   [TBD][Spawn] player=<n> KILLED — one life spent (key=<key>), slot retained, awaiting admin
   ```

   (`TBD_SpawnManager.OnPlayerKilled`), and their client enters spectator: `F` free camera, `←`
   and `→` next and previous, `TAB` roster, `V` view (`apps/mod/tbd-framework/Configs/System/Actions/`).
   `[TBD][Spawn] player=<n> killed — re-armed for respawn (slot retained)` instead means
   `m_bOneLife` is off on the prefab, and the session is not testing the event configuration.

3. **S12, continued — prove one life holds.** Ask the server to deploy the dead player:

   ```text
   #tbd deploy <playerId>
   ```

   Expected: the server refuses, logging
   `[TBD][Spawn] deploy DENIED player=<n> key=<key> — one life spent (admin respawn is the only way back)`.
   A body for the dead player is a headline finding: one life is broken. A fresh seat cannot be
   claimed from the lobby either, but the lobby's clicks never reach the server
   ([Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md),
   S8), so this admin deploy is the server-side check.

4. **S13 — admin respawn.** As the admin:

   ```text
   #tbd dead
   ```

   Expected: `TBD dead: <playerId>`.

   ```text
   #tbd respawn <playerId>
   ```

   Expected: `[TBD][Admin] respawn player=<n> by=<you> result=<…>` and
   `[TBD][Admin] respawn player=<n> by=<you> — back in the world, life restored`
   (`TBD_SpawnManager.AdminRespawn`, `FinishAdminRespawn`). The player leaves spectator in a
   freshly dressed body: `[TBD][Slots] rematerialized body for slot <key> (<reason>) — freshly dressed from mission JSON`.
   Check the loadout again, as in S10. On an event seat the respawn first waits for the platform:
   `[TBD][Admin] respawn player=<n> by=<you> - awaiting the platform's deployment decision, player stays DEAD until it allows the new life`.
   Other replies:
   `respawn REFUSED … — not dead` or `— disconnected` (wrong id; `#tbd dead` lists the dead),
   `— RETRY queued, player stays DEAD until a body lands` (wait, then run it again), and
   `did NOT deploy (…) — player REMAINS dead, run '#tbd respawn <n>' again` (run it again;
   capture and file it if it never lands). Then `#tbd audit` must list your respawn
   (`<you> respawn <playerId> -> …`); the trail records admin actions, not deaths.

5. **S14 — reconnect.** The second player disconnects and rejoins while alive, then again after
   dying.

   Expected, alive: `[TBD][JIP] player=<n> left ALIVE — seat <key> released, reclaim recorded under key <k> keyMode=<mode>`.
   Dead: `[TBD][JIP] player=<n> left DEAD — seat <key> retained under key <k> reclaimable=<0|1> …`,
   and on rejoin `[TBD][Spawn] player=<n> rejoined on a spent life — slot <key> handed back (still dead)`
   (`TBD_SpawnManager.OnPlayerDisconnected`, `ReclaimDepartedSeat`). A dead player who rejoins
   alive means one life did not survive the reconnect, expected only after the S4 waiver. A
   hole in the world where the body stood is a known engine behaviour
   ([known limitations](/documentation_v2/runbooks/two_client_playtest/known_limitations.md)).

6. **S15 — end the round.** As the admin, or let an authored win condition fire:

   ```text
   #tbd stage END
   ```

   Expected: `[TBD][Stage] LIVE -> END`; when a win condition fires instead, a `[TBD][Win]` line
   names the reason and `winner=`. The mission's `winConditions` decide which conditions can
   fire; a 5400 s time limit does not fire inside a session, so force the end.

7. **S16 — capture before tearing down.** While the server still runs, capture the evidence
   ([pass criteria and evidence](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md#capture-the-evidence)).
   Every start of `cargo xtask mod playtest` deletes `<run dir>/profile/logs`, so a restart loses
   this session's log.

   Expected: the evidence folder holds the server `console.log` and `error.log`, both client logs,
   the artifact document, `server.json`, and the screenshots.

## Verify

```bash
grep -E 'KILLED — one life spent|deploy DENIED .* one life spent|back in the world, life restored|rematerialized body|\[TBD\]\[Stage\] .*-> END' "$LOG"
```

Expected: one line of each kind: the terminal death, the refused deploy, the respawn, the fresh
body and the end of the round.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `SAFE_START has no enforcement on this world …` | `TBD_SafestartManager` did not resolve on the game mode | `#tbd stage LIVE`; warn the players |
| SAFE_START or LIVE refused for identity | a connected player has no durable identity | S4: fix the registration, or waive with the phrase |
| `re-armed for respawn (slot retained)` after a death | `m_bOneLife` is off on the prefab | run with the committed `Prefabs/Systems/TBD_GameMode.et` |
| `respawn REFUSED … — not dead` | the id names a living player | `#tbd dead`, then respawn that id |
| spectator keys do nothing | the action configs did not register | `grep -i "Setting null GUID" "$LOG"`; chat commands keep working |
| `#tbd audit` lacks the respawn | `TBD_AdminService` did not record the action | capture the chat and the log; file it |

## Related

- [Pass criteria and evidence](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md)
  — the next runbook: judging and capturing the session.
- [Spawning](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/README.md) — one life,
  the admin respawn and the reconnect reclaim in the mod.
- [Admin session](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md) — the `#tbd`
  commands and the audit trail.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
