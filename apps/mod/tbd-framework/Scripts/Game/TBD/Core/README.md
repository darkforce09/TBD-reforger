# Framework core utilities

The small utilities every other part of the TBD framework [mod](/documentation_v2/glossary/g_to_m.md#mod)
leans on: the structured log, the alias-to-prefab resolver, server-to-player chat, the replication
side test, SHA-256 in script, and the small shared helpers in the subfolders (character, faction and
player lookups, once-per-key logging, rounding, clock text, the RPC record codec and world queries).

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/
├── Characters/                  character body predicates: is this body dead
├── Factions/                    membership of a key in the loaded mission's declared factions
├── Hashing/                     SHA-256 in script: the hasher, a frame-spread job and the self-test
├── Logging/                     once-per-key warnings and announcements over `TBD_Log`
├── Math/                        nearest-integer rounding for both signs
├── Players/                     the faction of a player's assigned slot
├── Time/                        countdown clock text and the milestones players are told at
├── Wire/                        the tab-and-newline record codec of the session RPC payloads
├── World/                       world box queries: nearest vehicle, every entity of a prefab
├── TBD_Authority.c              which side of the replication session this machine is on
├── TBD_Log.c                    the structured log: one greppable `[TBD][<channel>]` line per call
├── TBD_PlayerChat.c             server-to-player chat: one player, everyone, or a logged broadcast
├── TBD_Registry.c               resolves registry aliases (`kit:*`, `veh:*`, ...) to prefabs
└── TBD_RegistryPocComponent.c   dev harness: spawns every registry alias in a row, off by default
```

## How it works

`TBD_Log` is static and stateless. `Event`, `Warn` and `Error` print `[TBD][<channel>] <message>` at
an explicit `LogLevel`; `Kv` appends a pre-built `key=value` string; `MissionLoaded`,
`ValidationResult` and `Stage` print the three lines operators grep for most
(`[TBD][Mission] loaded …`, `[TBD][Validate] mission result=PASS|FAIL …`, `[TBD][Stage] A -> B`), and
`Banner` frames a load-blocking failure between two rules. The fixed channels are `CH_MISSION`,
`CH_VALIDATE`, `CH_STAGE` and `CH_SAFESTART`; other callers pass their own channel constant. No call
sits on a per-frame path.

`TBD_Registry` loads the alias registry once, on the first `Load`, `Resolve` or `GetAllAliases`,
from `$TBD_Framework:Data/registry.json`, or from `$profile:TBD_Registry.json` only when the mod file
is missing. It keeps each entry's `alias` and `guid` and logs `[TBD] Registry loaded (<n> aliases).`;
an unknown alias logs an ERROR and resolves to an empty name with `ok` false.
`TBD_RegistryPocComponent`, a game mode component, spawns every alias along a line when its
`m_bRunPoc` attribute is on, starting at `m_vSpawnOrigin` and `m_fSpacing` metres apart.

`TBD_PlayerChat.Tell` sends a private chat message through the player controller's
`SCR_ChatComponent`, and `TellEveryone` returns how many players it reached; `Broadcast(tag, text)`
does the same from the authority only, after logging `[TBD][<tag>] broadcast: <text>` when `tag`
is not empty. `TBD_Authority.IsClient()` and `IsServer()` wrap `RplSession.Mode()`: a client is
`RplMode.Client`, and everything else (dedicated, listen host, single-player) is the server. Chat is the one
channel that reaches a player on a dedicated server without a menu preset, so replies, refusals and
announcements go through it. `Hashing/` holds the SHA-256 that gates every mission
[artifact](/documentation_v2/glossary/a_to_f.md#artifact).

## Authority

- Server: `TBD_PlayerChat` (`@authority server`) and the SHA-256 job and self-test. The registry
  proof-of-concept spawn runs only when `TBD_Authority.IsClient()` is false. `TBD_EntityQuery`
  runs where the world is authoritative.
- Client: nothing of its own; `TBD_Log` and `TBD_Registry` carry no tag and run wherever they are
  called.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_EGameStage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`
  (for `TBD_Log.Stage`); `apps/mod/tbd-framework/Data/registry.json`; the engine's
  `SCR_BaseGameModeComponent`, `SCR_ChatComponent`, `PlayerManager`, `RplSession` and
  `JsonLoadContext`; the subfolders' own READMEs list theirs.
- Used by: `TBD_Log` by nearly every framework script; `TBD_Authority` by every script that gates
  work on the replication side; `TBD_Registry` by the mission world applier, slot checks and
  vehicle roster, `TBD_SlotBodyMaterializer`, `TBD_ObjectiveDestroyTargets`,
  `TBD_TriggerWorldEffects`, the lobby's `TBD_LoadingGate` and the briefing's `TBD_UniformInfo`;
  `TBD_PlayerChat` by the round orchestration (`TBD_MissionFlowReport`, `TBD_EndBanner`,
  `TBD_RoundClock`), safe start (`TBD_SafestartManager`, `TBD_SafestartWatchdog`), the objectives,
  play area and trigger effects, `TBD_DeploymentAuthorization`, `TBD_MissionDeploymentRelay`, the
  identity link confirmation and the fleet actions in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/`. No prefab attaches
  `TBD_RegistryPocComponent`.
- Rules: every log line starts `[TBD][<channel>]`, and log scrapers such as
  `cargo xtask mod remote-logs` pin that prefix, never the sentence; the registry's shape follows
  `contracts_v2/definitions/registry.schema.json`; the proof-of-concept spawn stays off in a shipped
  prefab; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the framework's thesis, its
  non-negotiables and the event loop it serves
