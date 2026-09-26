**Status:** live

# Known limitations of the two-client playtest

What is known to be missing, unproven or environment-specific before the session starts. None of
it is a surprise, and each item says what it changes in the session and what to do. Read it
before booking the second player.

## Prerequisites

- The [index](/documentation_v2/runbooks/two_client_playtest/README.md) and the PASS lists in
  [Pass criteria and evidence](/documentation_v2/runbooks/two_client_playtest/pass_criteria_and_evidence.md).

## Steps

1. Check that the checkout's addon is what your own client can load. `tbd-framework` is published
   to the Workshop, unlisted, under the same id as the addon GUID `B2C3D4E5F6A78901`, at version
   1.0.1. The playtest server refuses to report success unless the checkout won
   ([Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md)), but a
   joining client resolves `game.mods[]` from the Workshop. Your own client can load the checkout
   instead:

   ```bash
   cargo xtask setup client-addons
   ```

   Expected: `apps/mod/tbd-framework/` linked into `~/.local/share/tbd-server-addons/` and the
   Steam launch options to paste (`-addonsDir "<that folder>" -addons B2C3D4E5F6A78901`); ignore
   the host address its last line prints.

2. Plan the second player's client. It gets the Workshop copy; no second machine has yet joined a
   server running the checkout, so it may refuse on a content mismatch or join and run old
   script. At S5 they type `#tbd`: a current build answers (`TBD: admin only.` for a non-admin),
   the old build has no such command. On silence, publish `tbd-framework` from
   [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) so the Workshop copy matches the checkout
   ([getting a change to the players](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md#getting-a-script-change-to-the-server-and-the-players)),
   then restart the server.

   Expected: both clients answer `#tbd` with a current build's reply.

## Verify

```bash
grep -c '\[TBD\]\[' "$LOG"
```

Expected: a count well above zero on the server's log. The old Workshop build prints no
`[TBD][<channel>]` line at all, so zero means it ran; the count itself varies with the mission and
is never a threshold.

## The limitations

- **The pre-game screens render mock data.** The lobby, briefing, mission selector and players
  screens read catalogs built from `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`, because no
  script calls their `Set()`; the roster wire in `Session/Lobby/TBD_LobbyClient.c` is complete on
  both ends, but nothing calls its `Request`, `Claim`, `Release` or `Deploy`. A lobby click
  changes only the local copy, so the server-side claim, contention and release checks and the
  side-specific briefing check cannot pass. The deploy is live: the briefing's "Ready & Continue"
  asks the server for a body, and the server seats the player by the event roster or round-robin.
- **Nothing spawns in LOBBY.** Bodies wait for BRIEFING: claimed holders deploy once on the
  LOBBY → BRIEFING change, and everyone else from the briefing's "Ready & Continue".
  `m_bAutoDeploy` is 1 in `TBD_SpawnManager.c` and 0 on `Prefabs/Systems/TBD_GameMode.et`; when on,
  it also seats unclaimed players at BRIEFING.
- **Radio tuning on the dev world.** `worlds/TBD_Dev_POC.ent` is a sub-scene of vanilla Everon
  whose layer places four entities: the TBD game mode, a faction manager, a loadout manager and
  the AI world. It places no `RadioManagerEntity`, so the mod logs `backbone: MISSING …` and tunes
  through a script-side channel table (`Systems/Radio/TBD_RadioTuner.c`); the net hint says when
  no tune was verified, and then the frequencies are dialled by hand. Adding the entity is a
  Workbench world edit.
- **Many `modded class SCR_PlayerController` blocks, never observed together at runtime.** Thirteen
  scripts extend the player controller (the lobby, briefing, spectator, markers, radio, spawn,
  mission browser, HUD and others), and fifteen blocks extend `ChimeraMenuPreset`. They compile
  together, but every one of them acts only with a client connected, and duplicate enum values
  compile clean. If one screen works and another silently does nothing, or two screens open the
  same menu, suspect this first and say so in the finding.
- **Menu presets and keybinds need a current resource database.** The spectator and admin action
  configs in `Configs/System/Actions/` carry `.meta` files; if `F`, the arrows, `TAB`, `V` or `F8`
  do nothing, `grep -i "Setting null GUID" "$LOG"`. The admin screen (`F8`, `#tbd menu`) opens only
  once Workbench has registered its preset in `resourceDatabase.rdb`; a screen that cannot open
  logs `GUI (E): Menu preset '<name>' not found!`. The `#tbd` chat commands depend on none of this.
- **A reconnect can delete the player's body.** Vanilla `SCR_BaseGameMode.OnPlayerDisconnected`
  deletes the disconnecting player's controlled entity, which is the slot body; the vanilla
  reconnect component that would keep it hangs off a join path the framework swallows. A hole in
  the world at S14 is this.
- **Player ids are recycled.** A dedicated server reuses numeric player ids, and
  `ScriptCallQueue.Remove` cancels by function, not by arguments, so deferred per-player callbacks
  carry a connection epoch. A joiner landing in a departed player's seat means that epoch check
  failed.
- **The headless rehearsal ends in `WORLD BOOT: FAIL`.** The warning budgets of the golden
  missions are 0 while the validator warns on their `environment` block and role radio nets, and
  `--compiled=<id>` also asserts the seeded fixture's four weapons
  ([Headless preflight](/documentation_v2/runbooks/two_client_playtest/headless_preflight.md)).
- **The dedicated server's Steam app is undecided.** The install hints of `mod compile`,
  `mod world-boot`, `mod playtest` and `mod bootstrap-staging` name app `1890870`, while
  `cargo xtask debug direct-join` reads the server's build from app `1874900`'s manifest. Every
  gate runs whatever is installed at
  `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`. The client is app `1874880`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the second player gets no answer to `#tbd` | their client runs the Workshop's 1.0.1 build | publish from Workbench; restart the server |
| the server log has zero `[TBD][` lines | the server ran the old Workshop build | the playtest gate refuses this; check the run's `FAILED:` output |
| a lobby click changes nothing on the server | the lobby renders a mock catalog | expected; seating happens at deploy |
| the radio hint says no tune was verified | no verified tune through the fallback table | dial the listed frequencies by hand |
| `F8` opens nothing | the admin screen's preset is not registered | use the `#tbd` chat commands |

## Open work

- [T-1085 — Feed the pre-game screens live catalogs instead of mocks](/.ai/tickets/T-1085.toml)
  (idea, no plan): the lobby and briefing read the server's roster, so claims, releases and the
  per-side briefing reach the server.
- [T-1096 — Decide the dedicated server Steam app id xtask relies on](/.ai/tickets/T-1096.toml)
  (idea, no plan): one server app for the workstation, CI and staging, and aligned install hints.
- [T-181.16 — Two-client dedicated-server event loop E2E](/.ai/tickets/T-181.16.toml) (queued, no
  plan) and [T-068.14 — Phase 2 E2E gate editor to player](/.ai/tickets/T-068.14.toml) (queued,
  no plan): this playtest.

## Related

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) and
  [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the mock catalogs, screen by screen.
- [Client join and mod updates](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — the Workshop copy, the client addon link and the publish.
- [Radio](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/README.md) — the tuner and its
  fallback channel table.
- [Event mod program](/documentation_v2/tickets/specs/t181_event_mod_program.md) — the frozen
  specification that records the reconnect, id-recycling and controller-coexistence findings.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
