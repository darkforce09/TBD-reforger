**Status:** live

# Contracts documentation

The documents on the cross-boundary contracts in `contracts_v2/`: how a schema may change, and the
contracts whose partner lives outside this repository. Developers and AI agents read them below
the [contracts README](/contracts_v2/README.md), which says what each folder of the tree holds.

## Contents

```text
documentation_v2/contracts_v2/
├── definitions/                the contracts of individual schemas in contracts_v2/definitions/
└── schema_evolution_policy.md  how a schema may change: additive rules, versions, breaking changes
```

## How it works

Read the [schema evolution policy](/documentation_v2/contracts_v2/schema_evolution_policy.md)
before editing any schema; it holds the boundary invariants, the additive and breaking-change
rules and the steps a change takes. `definitions/` holds a document for a schema whose meaning a
reader cannot take from the schema file alone, such as a wire shared with a partner. Each document
follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Schema evolution policy](/documentation_v2/contracts_v2/schema_evolution_policy.md) | invariants, codegen, additive and breaking changes, the change steps | [`contracts_v2/definitions/`](/contracts_v2/definitions/README.md) |
| [TBD Voice game bridge contract](/documentation_v2/contracts_v2/definitions/bridge_messages.md) | transport, envelope, message lifecycle, radio nets, framework hooks | `contracts_v2/definitions/bridge-messages.schema.json` |

## Code

- [Contracts](/contracts_v2/) — the schemas, rules, catalogues and fixtures these documents
  govern.
- [Contract definitions](/contracts_v2/definitions/) — the schema files the policy applies to.
- [Voice bridge samples](/contracts_v2/fixtures/bridge_samples/) — the messages the bridge
  contract describes.

## Boundaries

- Depends on: the schemas and fixtures in `contracts_v2/`, the codegen in
  `tools_v2/xtask/src/commands/generate/` and the schema gates in
  `tools_v2/xtask/src/verifications/schemas/`, which every claim is checked against; the feature
  doc template; the ticket registry in `.ai/tickets/` for open work.
- Used by: the READMEs of `contracts_v2/`, `contracts_v2/definitions/` and
  `contracts_v2/fixtures/bridge_samples/`, which link these documents; the [mod](/documentation_v2/glossary/g_to_m.md#mod)'s radio hook class
  `TBD_RadioBridgeStub`, whose header cites the bridge contract.
- Rules: a document describes the committed schemas and code, and a disagreement goes under Known
  discrepancies with both places; the archived relocation plan stays frozen, and the live policy
  is this folder's.

## Related documentation

- [Contract pipeline and evolution policy, archived](/documentation_v2/archive/contracts_v2_relocation/architecture_plan.md)
  — the frozen plan the live policy carries forward.
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the framework's
  no-workshop-dependency rule behind the bridge hooks.
