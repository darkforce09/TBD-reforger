# Mission document

The `mission_document` crate: `MissionDocCore`, the live, mergeable `yrs` document of one
[mission](/documentation/glossary/g_to_m.md#mission) that the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) edits. Its root maps, every
field-level write, the JSON views the payload compiler and the docks read, the
[slot](/documentation/glossary/n_to_z.md#slot) columns, hydrate, merge and paste, peer updates, the
selection policy and local undo.

## Contents

```text
crates/mission/mission_document/
├── Cargo.toml  the package: `mission_crdt`, `mission_model`, `mission_validation`, `orbat_slot_ids`, `yrs`, layout tier 5
└── src/        the document's rows, selection policy, ids, error, test clock and whole-document tests
```

## How it works

Every mutator opens one transaction under the `local-user` origin, or under `init` while
`set_origin_init(true)` is on; the undo manager tracks only `local-user`, so a hydrate or a
restore is never an undo step and a batch write is exactly one. The undo timestamps come from a
host `time_source::Clock` through `mission_crdt`'s grouping clock: the platform wall clock by
default (`Date.now()` in the browser), any clock passed to `MissionDocCore::with_undo_clock`
otherwise. `apply_update` merges a peer's update under `init` and refuses one that claims more
edits under the document's own client id than it has made. The [rows README](src/rows/README.md)
describes the maps, the exports, hydrate and merge.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_document   # rows, undo, peers, hydrate and compile round trips
```

## Configuration

One feature, `test_fixtures`, off by default: it exposes `test_fixtures::SteppingClock` to the
tests of crates that drive a document and is enabled only from their `[dev-dependencies]` (the map
engine's). No environment variables.

## Public surface

- `MissionDocCore` and its inherent commands and reads; `EntityTransformPatch`,
  `SquadMembership`, `MergeOpts`, `MergeReport`.
- `ConnectionKind`, `ConnectionRow`, `ConnectionFinding`, `validate_connection_rows`;
  `formation_offsets`.
- `ids`: `FactionId`, `SquadId`, `LayerId`, `EntityId`, `VehicleId`, `CommentId`,
  `CompositionId`, `ConnectionId`, `CrewSeatId`, `ClientId`.
- `Error`, `Result`; `prelude`; `test_fixtures` (feature-gated).

## Boundaries

- Depends on: `mission_crdt`, `mission_model`, `mission_validation`, `newtype_ids`,
  `orbat_slot_ids`, `time_source`, `deterministic_random` (the seeded slot positions), `yrs`,
  `serde_json`, `thiserror`; dev: `mission_payload`,
  `mission_compiler`.
- Used by: `mission_operations`, the map engine's editing layer, its integration tests, and the
  Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/`.
- Rules: mission tier 5 (`cargo xtask verify crate-tiers`); the Yjs maps and the exported editor
  payload stay byte-identical (the hydrate and compile round-trip tests); a local write is one
  undo step and a batch is one.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the mission domain's crates and their tiers.
- [Mission CRDT](/crates/mission/mission_crdt/README.md) — the id arrays, slot columns and undo clocks.
