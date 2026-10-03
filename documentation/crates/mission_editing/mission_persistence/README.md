**Status:** live

# Draft persistence documentation

The feature documentation of the `mission_persistence` crate: the decisions behind the Mission
Creator's local drafts of a [mission](/documentation/glossary/g_to_m.md#mission). Developers and AI
agents read it before changing how drafts are keyed, merged, classified or adopted.

## Contents

```text
documentation/crates/mission_editing/mission_persistence/
└── draft_persistence.md  draft keys, merge, classify, adopt and the snapshot pair
```

## Code

- [mission_persistence](/crates/mission_editing/mission_persistence/README.md) — the crate the
  feature doc describes; its source README holds the exact rules and the tests that pin them.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  code of `mission_persistence` and of the Mission Creator's shell that stores and fetches the
  drafts.
- Used by: the `mission_persistence` README, the
  [editing layer](/documentation/crates/mission_editing/editing_layer.md) document and the map
  engine documentation.
- Rules: the feature doc keeps its name, which those links use, and stays within 500 lines.

## Related documentation

- [Editing layer](/documentation/crates/mission_editing/editing_layer.md) — the editing host the
  draft decisions sit beside.
