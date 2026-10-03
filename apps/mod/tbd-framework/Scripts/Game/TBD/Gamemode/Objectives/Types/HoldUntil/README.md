# Hold-until objectives

The `objective_hold_until` kind: ground the zone's side must hold until a clock runs out, which
completes the objective and ends the round on `hold_expired` with the holder as the winner.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/HoldUntil/
└── TBD_ObjectiveHoldUntilBehaviour.c  the hold hooks: rules, holder, end condition, ladder, clock, text
```

## How it works

`TBD_ObjectiveHoldUntilBehaviour` answers the hold hooks of `TBD_ObjectiveKindBehaviour`. Its task
type is `hold`, and the side of an untyped hold defends (`SideDefendsByDefault`). `ApplyDefaults`
sets the hold announcement cadence; `ResolveRules` makes the objective inert without a holding
`faction` or without a `holdSeconds` inside the shared limit, then takes `pauseOnEnemy`,
`resetOnEnemy` and `requireHolderPresent`. A typed row's `side` supplies a missing holder
(`SeedFromSide`); a `startingOwner` other than the holder is logged and ignored.

On the LIVE edge `OnEnterLive` skips every rung of the remaining-time ladder (`NextHoldMark`: 600,
300, 120, 60, 30, 10 s) at or above the hold length. Each tick `Advance` runs the clock:

```text
TBD_ZoneVolume.EnemyContestsHold, HolderPresent
  enemy and resetOnEnemy        ──▶ clock reset, paused
  enemy and pauseOnEnemy        ──▶ paused
  requireHolderPresent, nobody  ──▶ paused
  otherwise ──▶ +1 s ──▶ ladder rung log ──▶ holdSeconds reached
            ──▶ complete ──▶ "TBD: <name> has been HELD to the clock by <holder>."
```

The HELD line goes back to the engine, which puts it in chat. `HasEnded` fires on any usable
complete hold objective and names the holder. The board status reads `HELD` or `hold <n>s left`,
with ` (PAUSED)` while the clock stands; the HUD glyph is `H`.

## Authority

- Server: everything; the hooks run from the objectives engine's server build and tick.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ObjectiveKindBehaviour` in the parent folder; `TBD_Objective` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/`;
  `TBD_ObjectiveRegistry.CH`, `TBD_ObjectiveRuleResolver` limits and `TBD_ObjectiveRulesStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/`;
  `TBD_ObjectivesComponent.TICK_SECONDS`; `TBD_ZoneVolume.EnemyContestsHold`, `HolderPresent` and
  its log channel; `TBD_DeclaredFactions` and `TBD_Log` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: the lookup in `TBD_ObjectiveKindBehaviour`, through which the objectives engine,
  `TBD_ZoneVolume` and the win-condition checks reach it.
- Rules: a hold objective with no holder or length goes inert rather than guessing when the round
  ends; `requireHolderPresent` defaults to false, since a one-life hold that needs a manned zone
  becomes unwinnable after casualties; the log and chat strings stay byte-identical.
