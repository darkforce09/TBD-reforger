# Clock text

Countdown text and the second marks at which players are told how long is left.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Core/Time/
└── TBD_ClockText.c  m:ss clock text and the safestart and round clock milestone ladders
```

## How it works

`TBD_ClockText.FormatClock(seconds)` writes `4:30` or `0:09`: minutes unpadded, seconds two
digits, and `0:00` for zero or less. Three ladders name the seconds at which a countdown speaks:
`IsCountdownChatMilestone` (600, 300, 120, 60, 30, 10), `IsCountdownPopupMilestone` (the chat
ladder plus 240, 180, 15 and each of the last five seconds) and `IsRoundClockMilestone` (1800 and
900, then the chat ladder). Chat is durable and turns to noise fastest, so its ladders are sparse;
a pop-up is transient, so its ladder is denser.

## Authority

- Server: nothing of its own; pure functions called by the server's countdowns.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: nothing.
- Used by: `TBD_SafestartManager` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/Safestart/`;
  `TBD_RoundClock` and `TBD_MissionFlowReport` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`.
- Rules: the ladders are the player-facing contract of when time is announced; lines added stay
  ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
