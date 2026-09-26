# Prefab text tokenizer

The low-level half of `prefab_catalog.rs` in `tools_v2/developer-tools/src/blueprint/bvh/`: the
tokenizer and block parser for [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) `.et` entity
templates, and the readers that pull one file's own facts out of the parsed blocks before the
parent resolves the inheritance chain.

## Contents

```text
tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/
└── tokenize.rs  `parse_et`, `strip_guid`, and the readers of one file's mesh, doors, pivot and children
```

## How it works

`tokenize` splits `.et` text into words, quoted strings and braces, and `parse_et` builds the
parent's `Block` tree from them: a class line with an optional base prefab, props, typed and named
blocks (`components`, `MeshObject "{guid}" { … }`), and the anonymous block that holds the child
entities. Unknown constructs are kept rather than refused. `strip_guid` turns `{GUID}path` into
`path`.

`own_facts` reads what one file states, before any inheritance: `MeshObject.Object` (the default
`Common/Models/Default.xob` counts as no mesh), the `DoorComponent` and `SlidingDoorComponent`
parameters with their `Enabled` flag (`enabled`), `Hierarchy.PivotID`, `SlotBoneMappings` and the
child entities. A `$grp` child line expands into one child per instance block, and `read_child`
gives each child its class, prefab, `ID`, pivot, `coords`, `angles` and `scale`. The parent's
`PrefabResolver` then merges these facts along the base chain, nearest first.

## Boundaries

- Depends on: the parent's `Block`, `Tok`, `OwnFacts`, `ChildRef`, `DoorParams` and
  `SlidingParams`.
- Used by: `prefab_catalog.rs`, which re-exports `parse_et` and `strip_guid`; `strip_guid` is also
  read by the prefab library in `tools_v2/developer-tools/src/blueprint/archive_emission/`.
- Rules: the parser is tolerant, since the `.et` grammar has no published specification; the
  tokenizer and block shapes are pinned against an inline `.et` text (`tokenizer_and_block_shapes`),
  and a resolver walk over inheritance, sockets and children against the `.et` fixtures in
  `tools_v2/developer-tools/test_fixtures/blueprint/prefab/`
  (`resolver_walks_inheritance_sockets_and_children`), both in
  `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs`.

## Related documentation

- [Prefab text fixtures](/tools_v2/developer-tools/test_fixtures/blueprint/prefab/README.md) — the
  `.et` files the resolver test reads.
