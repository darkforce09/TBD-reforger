# Character predicates

Questions about a character body in the world that several systems ask the same way.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/Characters/
└── TBD_CharacterUtil.c  whether a body is a dead character; the caller chooses the unreadable answer
```

## How it works

`TBD_CharacterUtil.IsDead(body, missingCountsAsDead)` casts to `ChimeraCharacter` and asks its
`CharacterControllerComponent`. A null body, a non-character or a character without a controller
returns `missingCountsAsDead`: false for presence checks (objectives, play area, triggers), true
for the spawn path, which rebuilds a body it cannot read rather than hand it on.

## Authority

- Server: nothing of its own; it runs wherever the caller runs, and its callers are server-side.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `ChimeraCharacter` and `CharacterControllerComponent`.
- Used by: `TBD_ObjectivesComponent` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`, `TBD_PlayAreaComponent` and
  `TBD_TriggerPlayerSnapshot` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`, and
  `TBD_DeployExecutor` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Deploy/`.
- Rules: never throws on null; lines added stay ASCII; `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Framework core utilities](/apps/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
