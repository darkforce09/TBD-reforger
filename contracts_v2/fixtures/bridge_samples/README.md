# Voice bridge samples

Six messages across a voice session's lifecycle, as the in-game TBD-Radio bridge mod sends them
to the external TBD Voice client. Both sides are built outside this repository; these samples are
the shared reference each implements against, so either can change without a coordinated release.

## Contents

```text
contracts_v2/fixtures/bridge_samples/
├── death.json         a player dies and moves to the dead channel
├── hello.json         the bridge's handshake: mission id and supported schema versions
├── net_change.json    a player retunes, replacing their net membership
├── ptt.json           a push-to-talk transition, by radio or direct speech
├── spawn.json         a player spawns into a role, with faction, radio class and nets
└── stage_change.json  the game-mode stage changes, so the client can mute radio outside it
```

## Format

- Encoding: UTF-8 JSON, one message per file, named after the message `type` it carries.
- Schema: `contracts_v2/definitions/bridge-messages.schema.json`. Every message shares the
  envelope `v` (protocol version, 1), `type`, `ts` (epoch milliseconds) and `session`; the schema
  also defines an `ack` message, which has no sample here.
- Adding a file: add one message named after its type and run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers: `cargo xtask schema validate`
  (`tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs`), which
  validates every `.json` file here against the bridge schema; nothing else in the repository
  reads them.

## Boundaries

- Depends on: `contracts_v2/definitions/bridge-messages.schema.json`.
- Used by: the xtask schema gate.
- Rules: every sample validates against the bridge schema (`cargo xtask schema validate`); a
  schema change updates the samples of every message type it touches in the same change.

## Related documentation

- [TBD Voice game bridge contract](/documentation_v2/contracts_v2/definitions/bridge_messages.md)
  — the transport, the envelope and the message lifecycle.
