# Rust path rewriting

The `rust_path` row pass of the relocation: every Rust path in one source file that starts with
a retired prefix, rewritten to the new prefix in `use` trees, code, attributes, doc links,
comments and string literals.

## Contents

```text
tools_v2/xtask/src/commands/refactor/relocate/rust_paths/
├── mod.rs          the pass over one file: use trees, code paths, comments and literals
├── module_tree.rs  the module path of every file of a crate and of every byte inside a file
├── path_rules.rs   the rows as segment rules, and how one path is rewritten by them
└── use_trees.rs    `use` declarations flattened into leaves and rendered back as one tree
```

## How it works

`path_rules.rs` holds the rows longest prefix first and matches whole segments only. Before a
`self::` or `super::` path is matched, it is turned into an absolute `crate::` path when it crosses
the edge of a moved module, inward or outward, because the move changes the depth it was counted
against; `module_tree.rs` supplies the file's module by walking the crate from its roots through
its `mod` declarations, honouring `#[path = "…"]` and both the `mod.rs` and `<module>.rs` layouts,
and adds the inline modules around the byte in question.

`use_trees.rs` flattens each `use` declaration into one leaf per imported name with the straight
chain each segment sits in. `mod.rs` rewrites the leaves in place when the rewritten segments form
one chain, and regroups and re-renders the whole tree when the retired prefix spans a `{`; a leaf
that imported the moved module under its own name keeps that name with `as`. Paths in code and the
same prefixes inside comments and string literals are rewritten through the same rules. A tree that
would need regrouping but holds a comment or a leading `::` is reported unresolved, never rewritten
with the comment lost.

## Boundaries

- Depends on: `super::rust_lexer`, `super::text_edits`, `super::path_mapping`,
  `super::repository_files` and `super::path_references::path_tokens`.
- Used by: `super::relocation_plan` (the pass, with a module tree when a row moves a `crate::`
  module) and `super::retired_spellings` (the same pass without a module, to find surviving
  prefixes).
- Rules: a rule matches whole segments only and the longest `from` wins; edits never overlap
  (`relocate_rust_path_rewrites_use_trees_doc_links_and_super_chains`,
  `relocate_use_trees_flatten_and_render`).
