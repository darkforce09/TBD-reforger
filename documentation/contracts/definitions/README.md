**Status:** live

# Contract definition documents

Documents for individual schemas in `contracts/definitions/` whose meaning goes beyond the
schema file: a wire shared with a partner outside the repository, its lifecycle and the hooks
that feed it. The [definitions README](/contracts/definitions/README.md) lists every schema and
who reads it.

## Contents

```text
documentation/contracts/definitions/
└── bridge_messages.md  the TBD Voice game bridge: transport, envelope, messages, radio nets, hooks
```

## Code

- [Contract definitions](/contracts/definitions/) — `bridge-messages.schema.json` and the
  other schemas.
- [Radio systems](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/) — the framework's
  bridge hooks and its own radio tuning.

## Boundaries

- Depends on: the schema files and their samples in `contracts/`, and the mod code that feeds
  the wire.
- Used by: the `contracts/`, `contracts/definitions/` and
  `contracts/fixtures/bridge_samples/` READMEs, and the header of
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioBridgeStub.c`.
- Rules: one document per schema, named after the schema's subject in snake_case; the schema file
  stays the authority, and the document follows the
  [schema evolution policy](/documentation/contracts/schema_evolution_policy.md).
