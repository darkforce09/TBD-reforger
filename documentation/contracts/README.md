**Status:** live

# Contracts documentation

The documents on the cross-boundary contracts in `contracts/`: how a schema may change, and the
contracts whose partner lives outside this repository. Developers and AI agents read them below
the [contracts README](/contracts/README.md), which says what each folder of the tree holds.

## Contents

```text
documentation/contracts/
├── definitions/                the contracts of individual schemas in contracts/definitions/
└── schema_evolution_policy.md  how a schema may change: additive rules, versions, breaking changes
```

## How it works

Read the [schema evolution policy](/documentation/contracts/schema_evolution_policy.md)
before editing any schema; it holds the boundary invariants, the additive and breaking-change
rules and the steps a change takes. `definitions/` holds a document for a schema whose meaning a
reader cannot take from the schema file alone, such as a wire shared with a partner. Each document
follows the [feature doc template](/documentation/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Schema evolution policy](/documentation/contracts/schema_evolution_policy.md) | invariants, codegen, additive and breaking changes, the change steps | [`contracts/definitions/`](/contracts/definitions/README.md) |
| [TBD Voice game bridge contract](/documentation/contracts/definitions/bridge_messages.md) | transport, envelope, message lifecycle, radio nets, framework hooks | `contracts/definitions/bridge-messages.schema.json` |

## Code

- [Contracts](/contracts/) — the schemas, rules, catalogues and fixtures these documents
  govern.
- [Contract definitions](/contracts/definitions/) — the schema files the policy applies to.
- [Voice bridge samples](/contracts/fixtures/bridge_samples/) — the messages the bridge
  contract describes.

## Boundaries

- Depends on: the schemas and fixtures in `contracts/`, the codegen in
  `tools/commands/schema_tooling/src/generate/` and the schema gates in
  `tools/commands/schema_tooling/src/schema_checks/`, which every claim is checked against; the feature
  doc template; the ticket registry in `.ai/tickets/` for open work.
- Used by: the READMEs of `contracts/`, `contracts/definitions/` and
  `contracts/fixtures/bridge_samples/`, which link these documents; the [mod](/documentation/glossary/g_to_m.md#mod)'s radio hook class
  `TBD_RadioBridgeStub`, whose header cites the bridge contract.
- Rules: a document describes the committed schemas and code, and a disagreement goes under Known
  discrepancies with both places; the archived relocation plan stays frozen, and the live policy
  is this folder's.

## Related documentation

- [Contract pipeline and evolution policy, archived](/documentation/archive/contracts_v2_relocation/architecture_plan.md)
  — the frozen plan the live policy carries forward.
- [Mod design](/documentation/apps/mod/tbd-framework/mod_design.md) — the framework's
  no-workshop-dependency rule behind the bridge hooks.
