**Status:** live

# TBD Voice game bridge contract

The wire between the game and an external voice client: the messages an in-game TBD-Radio bridge
[mod](/documentation_v2/glossary/g_to_m.md#mod) sends to the TBD Voice client, so that voice channels
follow each player's slot, radio nets, life state and the game-mode stage. Both ends are built
outside this repository; the framework's side of the contract is its hook points and the
[mission](/documentation_v2/glossary/g_to_m.md#mission) document's `radioPlan`.

## Where it lives

- Code: the schema
  [`contracts_v2/definitions/bridge-messages.schema.json`](/contracts_v2/definitions/bridge-messages.schema.json),
  one sample per message type in
  [`contracts_v2/fixtures/bridge_samples/`](/contracts_v2/fixtures/bridge_samples/README.md), and
  the hook class `TBD_RadioBridgeStub` in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/README.md).
- Entry: the bridge's `hello` message; on the framework side, the stage-change call in
  `TBD_FrameworkManager.SetStage`
  (`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c`).
- Related features: the framework's own radio tuning, which reads the same `radioPlan.nets[]`
  and needs no bridge (the [Radio README](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/README.md));
  the [mod design](/documentation_v2/mod/tbd-framework/mod_design.md), which rules out workshop
  dependencies.

## Behaviour

### Transport

No transport is chosen: local REST, a WebSocket or a named pipe would all carry the contract.
The envelope is transport-agnostic, so the same JSON object is a WebSocket frame or an HTTP body.
Every message validates against the schema, and the samples in
[`/contracts_v2/fixtures/bridge_samples/`](/contracts_v2/fixtures/bridge_samples/README.md) are the
reference each side implements against.

### Envelope

Every message carries the same envelope:

| Field | Type | Meaning |
|---|---|---|
| `v` | integer | protocol version, the constant `1` |
| `type` | string | `hello`, `ack`, `spawn`, `death`, `net_change`, `ptt` or `stage_change` |
| `ts` | integer | Unix epoch milliseconds |
| `session` | string | the scope a voice room binds to: the [event](/documentation_v2/glossary/a_to_f.md#event) or server session |
| `player` | object | `identityId`, `platform` (`pc`, `xbox`, `psn`) and an optional `name`; required on `spawn`, `death`, `net_change` and `ptt` |
| `payload` | object | the type's body |

`player.identityId` is the engine identity the framework already keys slot enforcement on, the
same `arma_id` the website binds to a member through `POST /api/v1/ingest/link-confirm`
(`ingest_link_confirm` in
`apps/website/api_v2/src/identity_and_access/handlers/arma_link_confirmation.rs`). The voice
client matches a connection to an in-game player by this id and never by what the client claims.

### Message lifecycle

```text
TBD-Framework             TBD-Radio bridge                 TBD Voice client
     |                          |── hello (role, missionId, schemaVersions) ──▶|
     |                          |◀── ack (ok) ─────────────────────────────────|
     |── stage LIVE ───────────▶|── stage_change (LIVE) ──────────────────────▶|
     |── player spawned ───────▶|── spawn (faction, slot, radioClass, nets) ──▶|
     |── push-to-talk down ────▶|── ptt (start, radio, netId) ────────────────▶|
     |── player retunes ───────▶|── net_change (nets) ────────────────────────▶|
     |── player died ──────────▶|── death (channel dead) ─────────────────────▶|
```

1. `hello` and `ack`: the handshake. The bridge (`role` `bridge`) announces the mission id and the
   mission schema versions it supports; the client acknowledges with `ok` and an optional
   `message`.
2. `spawn`: the framework spawned a player into a role. It carries the faction, the optional group
   callsign and slot, the `radioClass` of the issued radio (`none`, `handheld`, `manpack`,
   `vehicle`) and the nets the player may use.
3. `net_change`: an in-mission retune, or a radio picked up or dropped. It replaces the player's
   active net membership and may restate the radio class.
4. `ptt`: push-to-talk `start` or `stop`. `mode` is `radio` or `direct` (proximity speech);
   `netId` names the net of a radio transmission.
5. `death`: the player moves to the `dead` or `spectator` channel, so the living no longer hear
   them.
6. `stage_change`: the game-mode stage (`LOADING`, `LOBBY`, `BRIEFING`, `SAFE_START`, `LIVE`,
   `END`, `DEBRIEF`, the values of `TBD_EGameStage`), so the client can, for example, mute radio
   outside `LIVE`.

### From radioPlan to voice nets

The mission's `radioPlan.nets[]` (`contracts_v2/definitions/mission.schema.json`, at most 32 nets)
is the single source of the nets. Each net becomes one `netMembership` entry:

| Mission field | Bridge field | Voice client use |
|---|---|---|
| `net.id` (`net:<name>`) | `id` | stable channel key |
| `net.label` | `label` | display name |
| `net.freqMHz` (30 to 512) | `freqMHz` | tuning and same-frequency grouping |
| `net.range` (`short` or `long`) | informs `radioClass` | handheld or backpack radio gating; the engine has exactly two radio gadget types |
| the role's `radio[]` membership | `transmit` and `receive` | which nets a slot may use |

A squad leader whose role lists `["net:cmd", "net:alpha"]` spawns with both nets set
`transmit: true, receive: true`; a rifleman whose role lists no nets spawns with none and has
direct speech only.

### Framework hook points

`TBD_RadioBridgeStub` declares one static method per message a bridge would send. A bridge mod
subscribes to these without forking the framework:

| Hook | Fires when | Message | Called today |
|---|---|---|---|
| `OnPlayerSpawned(identityId, radioNetIds)` | a player spawns into a slot | `spawn` | no; empty |
| `OnPlayerSpawnedById(playerId)` | the same, keyed by player id | `spawn` | no; builds the player's radio nets |
| `OnPlayerKilled(identityId)` | a player dies | `death` | no; empty |
| `OnRadioRetune(identityId, netId)` | a player changes frequency or radio | `net_change` | no; empty |
| `OnPTT(identityId, netId, pressed)` | push-to-talk changes | `ptt` | no; empty |
| `OnStageChanged(stage)` | the stage machine moves | `stage_change` | yes, from `TBD_FrameworkManager.SetStage` |

`OnStageChanged` is the one live call: it starts the framework's own radio sweep, which serves and
tunes every connected player's nets at `SAFE_START` and `LIVE`. The empty hooks are the documented
subscription points; the hook names are the framework's own and not engine symbols.

### Versioning

`v` changes only with a breaking change to the envelope; an added optional payload field keeps
it. Mission schema versions are negotiated separately: the bridge lists the ones it supports in
`hello`, and neither side runs a mission whose schema version the other cannot read. The
[schema evolution policy](/documentation_v2/contracts_v2/schema_evolution_policy.md) covers the
schema file itself.

### Known discrepancies

- The hook header says `TBD_FrameworkManager.c:250` calls `OnStageChanged`
  (`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioBridgeStub.c:21`) — the call
  is at `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:1200`.
- The schema's description says the transport is "decided in Phase 0.2"
  (`contracts_v2/definitions/bridge-messages.schema.json:5`) — no transport is chosen and no
  phase plan exists.
- The bridge's `radioClass` has four values, `vehicle` among them (`bridge-messages.schema.json`)
  — the framework's tuner knows two radio gadgets, `RADIO` and `RADIO_BACKPACK`, and a net's
  `range` two values, so nothing in the framework yields `vehicle`.

## Data

- `contracts_v2/definitions/bridge-messages.schema.json`: a `oneOf` over the seven message types,
  each a closed object (`additionalProperties: false`) with `v` fixed at 1; net ids match
  `^net:[a-z0-9_]+$`.
- `contracts_v2/fixtures/bridge_samples/*.json`: one sample per type except `ack`; `cargo xtask
  schema validate` checks every file against the schema.
- `radioPlan` and each role's `radio[]` in `contracts_v2/definitions/mission.schema.json`: the
  source of every net a `spawn` or `net_change` carries.

## Design

- The framework and the voice stack meet only here: the framework keeps its hooks and the
  mission's `radioPlan` stable, and the message format is the whole of the coupling.
- The framework takes no workshop dependencies, so no bridge mod lives in this repository and the
  hooks without a caller stay empty. Radio nets do not wait on a bridge: the framework reads,
  serves and tunes `radioPlan.nets[]` into the engine's own radios, with frequencies as integer kHz
  on its own wire.

## Open work

- [T-1080 — Clean up contract schema descriptions, $id hosts and workflow name](/.ai/tickets/T-1080.toml)
  (idea, no plan): the bridge schema's description stops naming a phase for the transport
  choice.

## Decisions

- The envelope is transport-agnostic: the transport can be chosen late without changing a
  message.
- Players are matched by the engine identity the website already links: a voice client is never
  trusted to say who it is.
- `radioPlan.nets[]` is the single source of nets for both the framework's tuner and the bridge,
  so voice channels and in-game frequencies cannot disagree.
- The framework takes no workshop dependency for voice: the hooks stay as subscription points,
  and the framework tunes the engine's radios itself.
