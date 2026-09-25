**Status:** live

# End screen

The END banner: the moment a round is decided, every player with a screen sees "MISSION ENDED",
the winning faction and why the round ended, over a dimmed world. It stays up until the next
stage closes it, which only an admin moves; the DEBRIEF scoreboard follows.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/README.md)
  (`TBD_EndScreen.c`) and the stage machine in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/README.md)
  (`TBD_FrameworkManager.c`), which records the winner and the reason.
- Layout: [`apps/mod/tbd-framework/UI/layouts/Session/PostGame/`](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/README.md),
  `TBD_EndScreen.layout`, whose README lists every widget the handler binds.
- Entry: `TBD_FrameworkManager.ApplyEndScreens`, which opens `TBD_EndScreen` on the `END` stage
  and closes it on every other stage.
- Related features: the [debrief](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md),
  the scoreboard the `DEBRIEF` stage opens; the
  [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md),
  whose admin screen forces the next stage.

## Behaviour

### How a round ends

The server ends a round by moving the stage machine from `LIVE` to `END`. Five paths lead there:

1. The objective end triggers, checked every 2 s while `LIVE` by `TickWinConditions`:
   `TBD_ObjectiveRegistry.EvaluateEndTriggers` returns `all_objectives_captured`,
   `objective_destroyed` or `hold_expired` with the winning faction.
2. Attrition, in the same tick: when at least two factions claimed a
   [slot](/documentation_v2/glossary.md#slot) and only one still has a living player, the reason is
   `faction_eliminated` and that faction wins.
3. The round clock, armed at `LIVE` from the [mission](/documentation_v2/glossary.md#mission)'s
   time limit: at zero it broadcasts "[TBD] TIME. The round is over." and ends the round with
   `time_limit`; the winner is the one faction still alive, else none.
4. The authored win rule's `extraction` and `vip` modes (`TBD_WinConditionEvaluator`) and a
   trigger's `end_mission` effect (`TBD_TriggerRuntime`).
5. An admin forcing the stage: `#tbd stage END` or `#tbd stage next` in chat, or the admin
   screen's "Force stage".

Paths 1 to 3 record their reason and winner before the stage changes. Paths 4 and 5 record none,
so on entering `END` the server infers them (`InferEndBanner`): an objective trigger that holds,
else `faction_eliminated` when exactly one of two or more contesting factions is alive, else
`admin` with the survivor arithmetic's winner.

### What the player sees

1. On `END`, `TBD_EndScreen.Open` creates the layout on the workspace of every machine that has
   one; a dedicated server has none and only logs `[TBD] END - winner=<key> reason=<reason>` to
   chat and console.
2. The panel reads "MISSION ENDED", the subtitle "The round is over.", the winner line and the
   reason line, and the footer "The next stage closes this screen.".
3. The winner line is the winning faction's key as the mission names it, or "No winner".
4. The reason line: "Faction eliminated." for `faction_eliminated`, "Time limit expired." for
   `time_limit`, "An admin ended the round." for `admin`, "The round ended." when no reason
   arrived, and the raw key for anything else (`all_objectives_captured`, `objective_destroyed`,
   `hold_expired`).
5. The layout's "BACK" button closes the banner on that machine only.
6. The banner stays until the stage changes. Nothing moves `END` to `DEBRIEF` on a timer: an admin
   does, with `#tbd stage next` or "Force stage". Any stage other than `END` closes it, and
   `LOADING` or `LOBBY` clears the stored winner, reason and board.

The overlay is a workspace widget, never a menu, so no key it swallows and no failure inside it
can refuse a stage change. On `END` the server also ends every life it opened on the platform
(`TBD_SpawnManagerDeploymentAuthorization.OnStageChanged`), the dynamic spawner deletes the AI
groups it spawned (`TBD_DynamicSpawner`), and a mission's authored `mission_end` audio cues fire
once (`TBD_AudioEmitter`).

### Known discrepancies

- The banner names the reason for every ending (`TBD_EndScreen.c`) — but the extraction, VIP and
  trigger endings call `SetStage(END)` without a reason (`TBD_WinConditionEvaluator.c`
  `EndRound`, `TBD_TriggerRuntime.c` `EffectEndMission`), so the inference labels them
  `faction_eliminated` or "An admin ended the round.".
- The objective triggers reach the player as raw keys such as `all_objectives_captured`
  (`TBD_EndScreen.DescribeReason`), since only three keys have a sentence.

## Data

The screen makes no HTTP call and sends no RPC. It reads three `TBD_FrameworkManager` properties,
replicated to every client:

- `m_sEndWinner` (`GetEndWinner`) and `m_sEndReason` (`GetEndReason`): written on the authority
  inside `SetStage(END)` by `SnapshotEndBanner`, from the pending pair the ending path recorded or
  from `InferEndBanner`.
- `m_sDebriefBoard`: the packed scoreboard, snapshotted at the same moment for the debrief.

## Design

- As built: a full-screen `ScreenRoot` with a `Backdrop` painted `TBD_UITheme.SCRIM`, a centred
  `PanelFrame` whose `Panel` is `SURFACE`, the `Title`, `Subtitle`, `Winner`, `Reason` and
  `Status` texts, two `OUTLINE_VARIANT` rules, and a "BACK" button. The layout shares the list
  shell's shape; the handler hides its `List` and `PrimaryAction`. Texts draw in the engine's
  default font, and faction colours are not applied.
- Design target: the [end screen banner mockup](/documentation_v2/mod/tbd-framework/UI/end_screen/visual_references/ultra_clear_2_second_end_screen_banner_mockup/README.md),
  a design-phase reference: a faction pill ("BLUFOR // US MARINE CORPS"), a large "VICTORY", a
  reason headline and a sentence, in a glowing glass card.
- The design target set out, and the built banner lacks:
  - a faction-coloured winner badge (blue, red or green fill; slate for a draw) and a full faction
    name in place of the key;
  - a reason headline and sentence per trigger: "ENEMY FORCES ELIMINATED", "OBJECTIVE SECURED",
    "MISSION GOAL ACCOMPLISHED", "TIME LIMIT EXPIRED" and "ADMINISTRATIVE TERMINATION";
  - the round's length (`MM:SS`, or `HH:MM:SS` past an hour) and a 1 Hz countdown to the debrief
    ("Debrief in: 10s"), with the server moving to `DEBRIEF` after 10 to 15 s;
  - "DISMISS (ESC)" in place of "BACK";
  - a combat freeze on `END`: weapons safed, movement locked, damage off;
  - a win fanfare, a loss tone and ambience ducked by 6 dB, beyond the authored cues.
- No open ticket covers these differences.

## Open work

- [T-1082 — Record end reason and winner for extraction, VIP, trigger endings](/.ai/tickets/T-1082.toml)
  (idea, no plan): the extraction, VIP and trigger endings record their own reason and winner, so
  the banner stops calling them admin endings.

## Decisions

- The END and DEBRIEF screens are workspace overlays, not menus: a menu can swallow Esc and hold
  input, and nothing on a player's screen may refuse or delay the server's stage change.
- Every ending passes through `SetStage(END)` and one banner snapshot: a new way to end a round
  adds a reason, never a second end path, so every guard and log line applies to it.
- The winner and reason replicate as two strings on `TBD_FrameworkManager`: a client holds no
  mission document, so it paints only what the server decided.
