# Capture objectives

The `objective_capture` kind: a zone a side takes by standing on it for its capture length, which
can change hands all round, and which ends the round on `all_objectives_captured` when one side
owns every usable capture.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/Capture/
├── TBD_ObjectiveCaptureBehaviour.c  the capture hooks: vocabulary, load log, starting owner, end condition, text
├── TBD_ObjectiveCaptureProgress.c   one tick of capture: contest, teardown, build, decay, the CAPTURED line
└── TBD_ObjectiveCaptureRules.c      validates the capture and neutralize lengths, contestable, onEmpty, decayRate
```

## How it works

`TBD_ObjectiveCaptureBehaviour` answers the capture hooks of `TBD_ObjectiveKindBehaviour`. Its
task types are `capture` and `defend` (the far side of a capture). `ResolveRules` hands the zone's
rules to `TBD_ObjectiveCaptureRules`: an unreadable `captureSeconds` falls back to the default with a
warning and never makes the objective inert, so `all_objectives_captured` stays winnable;
`neutralizeSeconds` defaults to the capture length. `ApplyStartingOwner` starts the objective full
and owned by a declared side the zone's `faction` allows, else logs and leaves it neutral.

Each tick `Advance` hands the objective to `TBD_ObjectiveCaptureProgress`:

```text
TBD_ZoneVolume.ResolveActingFaction ──▶ acting side, contested flag
  contested ──▶ frozen
  nobody    ──▶ onEmpty: hold keeps progress, decay drains a neutral objective's
  enemy progress or owner ──▶ teardown at neutralizeSeconds ──▶ neutral
  own side  ──▶ build to captureSeconds ──▶ owner ──▶ "TBD: <name> has been CAPTURED by <side>."
```

The CAPTURED line goes back to the engine, which puts it in chat; the progress lines go to the log
only, since the capture bar is the player channel. `HasEnded` needs at least one usable capture
and every one owned by the same side. The board status reads `neutral`, `OURS` or `held by
<side>`, then ` -- CONTESTED` or the partial percentage; the HUD glyph is `o`, `+` or `-`, and a
player inside a capture objective is shown its capture bar.

## Authority

- Server: everything; the hooks run from the objectives engine's server build and tick.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ObjectiveKindBehaviour` in the parent folder; `TBD_Objective` and
  `TBD_EObjectiveOnEmpty` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Model/`;
  `TBD_ObjectiveRegistry.CH`, `TBD_ObjectiveRuleResolver` limits and `TBD_ObjectiveRulesStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Registry/`;
  `TBD_ObjectivesComponent.TICK_SECONDS`; `TBD_ZoneVolume.ResolveActingFaction` and its log channel;
  `TBD_DeclaredFactions` and `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: the lookup in `TBD_ObjectiveKindBehaviour`, through which the objectives engine,
  `TBD_ZoneVolume` and the win-condition checks reach it.
- Rules: rates never scale with headcount; an owned objective is lost only to a side standing on
  it; a contest freezes progress both ways; restoring the owner's own bar is never announced as a
  capture; the log and chat strings stay byte-identical.
