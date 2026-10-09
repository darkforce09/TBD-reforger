# Game-document compiler

Compiles a saved [mission](/documentation/glossary/g_to_m.md#mission) payload into the document the
[mod](/documentation/glossary/g_to_m.md#mod) loads, as `contracts/definitions/mission.schema.json`
defines it, with the findings and kit substitutions of the same compile. The crate root of
`mission_compiler` re-exports its entry points, which the API and the Mission Creator import
directly.

## Contents

```text
crates/mission/mission_compiler/src/game_document/
├── compile_graph.rs              `flatten_to_mod_document`: the one walk from payload to document
├── diagnostics.rs                the compile's findings: identity checks, drop messages, quoting
├── document.rs                   `ModMissionDocument`, the compiled document
├── environment.rs                the environment axes; `apply_authored_environment`
├── export.rs                     JSON-in, JSON-out entry points: bytes, substitutions, findings
├── flow.rs                       the `flow` defaults and durations; net id and label helpers
├── identity.rs                   the finding rule ids; the schema's rank, stance and seat enums
├── loadouts_briefings.rs         slot gear and cargo, and the per-faction briefings
├── metadata.rs                   `MissionMeta`, schema limits, key slugs, terrain key, document id
├── mod.rs                        the module tree; the stages' shared imports and the crate's entry points
├── radio.rs                      the radio plan: authored verbatim, or derived from the ORBAT
├── roster.rs                     the vehicle roster rows with their crew seats; the settings
├── substitutions.rs              `KitSubstitutionReport`: seats compiled as their side's default
├── tests/                        unit tests for the contract, the goldens, findings, each block
├── type_safety.rs                `scan_editor_payload_types`: can the compiler parse these bytes
├── unsupported_authored_data.rs  `unsupported_authored_data`: gameplay data the document drops
├── vehicles.rs                   entities and vehicles as `entities[]` rows; heading, side, cargo
├── win_conditions.rs             the win rule, authored or derived; its timeout folded into `flow`
└── zones.rs                      authored zones, per-side spawn circles, the terrain boundary
```

## How it works

`flatten_to_mod_document(meta, payload)` parses the payload once into the input types of
`crate::authoring` and fills the document in one walk:

```text
MissionMeta + payload bytes
   │  payload does not parse ────────────────────────────────▶ Error::Parse
   ▼
factions in order ─▶ their squadIds ─▶ each squad's slots, sorted by `index`
   │  per slot: the kit alias, or the side's default kit (recorded as a substitution);
   │  tag, callsign, rank, stance, unitName emitted or dropped (dropped = a finding)
   ▼
no slot emitted ─────────────────────────────────────────────▶ Error::NoSlots
   ▼
entities and vehicles ─▶ vehicle roster ─▶ zones ─▶ authored blocks ─▶ win conditions ─▶ flow
   ▼
meta, environment, radio plan, briefings, settings ─▶ ModMissionDocument
```

A [slot](/documentation/glossary/n_to_z.md#slot)'s `id` is `faction:callsign:role:occurrence` and its
`uid` is the editor's slot id. Editor `position.x`, `.y`, `.z` and `.rotation` become `x`, `z`,
`y` (only when non-zero and finite) and `headingDeg` (normalised to 0–360). `schemaVersion` is
`1.3` when a 1.3 key reaches the document (a slot identity key, a squad leader, a roster vehicle,
an environment axis), `1.2` when a slot carries an elevation, and `1.1` otherwise.

| Document keys | Filled from |
|---|---|
| `meta`, `environment` | `MissionMeta`, whose time and weather `apply_authored_environment` takes from the payload first; wind, fog and view distance when authored in range |
| `radioPlan` | the authored plan verbatim, else one long-range command net per side, then squad nets, 32 nets at most from 30 MHz in 0.5 MHz steps |
| `zones` | authored zones, a 150 m spawn circle at each side's slot centroid, and the terrain rectangle when no zone is a boundary |
| `winConditions`, `flow` | the authored rule, else `attrition` ending on `time_limit`, plus `faction_eliminated` when two sides hold slots; flow defaults of 600, 300 and 5400 s |
| `entities`, `vehicles` | placed entities, and each placed vehicle as an entity row; the authored roster, where a row the schema cannot carry is dropped whole |
| other authored blocks | `tasks`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`, through `mission_model::authored_blocks` |

Findings are `mission_validation::Finding` values under the eight ids of
`COMPILE_DIAGNOSTIC_RULE_IDS`. They and the substitution report are never serialised, so the bytes
stay inside the schema, which closes the document. A finding never refuses the compile: the value
is dropped or replaced by the derivation, and the finding says which. Only an unparsable payload,
a payload whose squads emit no slot, and a placed vehicle whose `resourceName` has no `veh:` alias
fail it. `unsupported_authored_data` lists what changes gameplay (warning findings, substitutions,
authored editor triggers), and the [API](/documentation/glossary/a_to_f.md#api) makes no
[artifact](/documentation/glossary/a_to_f.md#artifact) of a version that has any.

## Boundaries

- Depends on: `crate::authoring` (the input types), `crate::Error`, `mission_model` (the compiled
  rows and the authored blocks), `mission_payload` (`terrain_bounds`, the kit aliases),
  `mission_validation` (`Finding`, `Severity`, `Primitive`, the ids) and `mission_wire_safety`
  (`is_wire_unsafe`, `MAX_REPORTED`); `serde_json`.
- Used by:
  - the API: `crates/api/api_missions/src/services/mission_compile.rs` builds `MissionMeta`
    from the mission row and compiles,
    `crates/api/api_missions/src/services/mission_artifacts/artifact_store.rs` calls
    `unsupported_authored_data`, and `crates/api/api_missions/src/handlers/mission_versions.rs`
    runs `scan_editor_payload_types` on every save;
  - the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): its compiled export
    (`crates/frontend/workspaces/mission_creator_session/src/document_commands/imp/compilation.rs`), the
    flow defaults of `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/env.rs`, and the
    `compiled_meta` builders in `crates/frontend/foundation/frontend_api_dtos/src/`.
- Rules: `contracts/fixtures/missions/valid/compiler-shaped-two-faction.json` is regenerated
  from this module, never hand-edited (`compiler_shaped_golden_is_a_fresh_emitter_output` in
  `tests/cases_3.rs`); `SLOT_RANKS`, `SLOT_STANCES` and `VEHICLE_SEAT_ROLES` equal the schema's
  enums (`the_identity_enums_are_the_schema_s_own`, `the_vehicle_seat_roles_are_the_schema_s_own`);
  findings never move the bytes and are never debug-gated, and substitutions never reach the wire
  (`a_diagnostic_is_not_a_refusal_and_does_not_move_the_bytes`,
  `substitutions_never_reach_the_compiled_wire`).

## Related documentation

- [Mission artifacts](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
  — how a compiled document becomes an immutable artifact.
- [Voice bridge contract](/documentation/contracts/definitions/bridge_messages.md) — how the
  radio plan's nets map to voice channels.
