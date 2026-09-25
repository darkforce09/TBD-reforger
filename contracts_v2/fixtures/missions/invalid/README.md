# Invalid mission fixtures

Negative goldens of the [mission](/documentation_v2/glossary.md#mission) contract: well-formed
documents that the gates must reject, each for one stated reason at one JSON pointer. A gate that
stops rejecting one of them names the rule that regressed.

## Contents

```text
contracts_v2/fixtures/missions/invalid/
└── *.json  one wrapper each: why it exists, the gate and pointer that must reject it, the mission
```

## How it works

`cargo xtask schema validate` reads every `.json` file here in name order
(`tools_v2/xtask/src/verifications/schemas/checks/mission_validation.rs`). For each wrapper it takes
`mustFail.gate`, `mustFail.at` and `document`, and a wrapper missing any of them fails the run. A
`schema` fixture must be rejected by `contracts_v2/definitions/mission.schema.json`, and every
finding must sit at or below `mustFail.at`. A `registry` fixture must pass the schema and fail only
the kit-alias cross-reference against the [mod](/documentation_v2/glossary.md#mod)'s spawn registry
`apps/mod/tbd-framework/Data/registry.json`, again at or below its pointer. A fixture rejected for
any other reason fails the run, so each file stays pinned to its reason.

| Fixture | Gate | Pointer | Defect |
|---|---|---|---|
| `cargo-container-typo.json` | schema | `/slots/0/loadout/cargo/0/container` | the cargo container reads `vets`, outside the container enum |
| `kit-alias-not-in-registry.json` | registry | `/slots/0/kit` | a well-formed `kit:` alias that no spawn registry entry defines |
| `net-range-any.json` | schema | `/radioPlan/nets/1/range` | a net range of `any`, outside the `short` and `long` radio classes |
| `slot-callsign-separator.json` | schema | `/slots/0/groupCallsign` | a TAB inside a group callsign, which the wire-safe string refuses |
| `zone-label-newline.json` | schema | `/zones/0/label` | a newline inside a zone label, which the wire-safe string refuses |
| `zone-rules-key-typo.json` | schema | `/zones/4/rules` | `graceSecconds` in a zone's closed `rules` object |

## Format

- Encoding: UTF-8 JSON, one wrapper per file, named in lowercase hyphenated words after the defect.
- Schema: a wrapper object with `$comment` (why the fixture exists), `mustFail` (`gate`: `schema`
  or `registry`; `at`: the JSON pointer the rejection must sit at or below; `because`: the harm
  the rule prevents) and `document`, a mission following
  `contracts_v2/definitions/mission.schema.json` except for its one defect. Each document is a
  valid mission with a single change; none is malformed JSON.
- Adding a file: copy a valid mission from `contracts_v2/fixtures/missions/valid/`, break exactly
  one rule, wrap it with its `mustFail` block, and run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers: `cargo xtask schema validate`, which is also the first step of the `schema-validate`
  CI task and of the `schema.yml` workflow; `mission_validation.rs` builds this folder's path
  itself and uses `dangling_kits` from
  `tools_v2/xtask/src/verifications/schemas/checks/kit_registry_references.rs` for the
  `registry` gate.

## Boundaries

- Depends on: `contracts_v2/definitions/mission.schema.json` and the `entries[].alias` values of
  `apps/mod/tbd-framework/Data/registry.json`.
- Used by: the xtask schema gate only.
- Rules: every fixture is rejected, by its named gate, at or below its pointer, and for no other
  reason (`cargo xtask schema validate`); a `registry` fixture passes the schema; a fixture is
  never repaired to make the gate pass, since a fixture that starts passing means a rule was lost.
