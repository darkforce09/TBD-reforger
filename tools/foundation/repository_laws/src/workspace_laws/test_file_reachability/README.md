# Test-file reachability, module declarations

The module declaration reader of the test-file reachability law in
`tools/foundation/repository_laws/src/workspace_laws/test_file_reachability.rs`: the files one
Rust source file's out-of-line `mod` declarations load.

## Contents

```text
tools/foundation/repository_laws/src/workspace_laws/test_file_reachability/
└── module_declarations.rs   every `mod <name>;` of a source, its `#[path]`, and the files the compiler would load for it
```

## How it works

`declared_module_files` blanks the comments and string literals of a source, so a declaration
quoted in a string or a comment never counts, and walks what is left: attributes (`#[…]`) are
collected until the next item, and a `path = "…"` among them (plain or inside `cfg_attr`) is read
from the original text at the same position. Each `mod <name>;` then names its candidate files by
the Rust reference's rules:

- in a file that owns its folder (a crate root, a `mod.rs` or a file loaded through `#[path]`),
  `<name>.rs` or `<name>/mod.rs` beside it; in any other file `<folder>/<stem>.rs`, the same under
  `<folder>/<stem>/`;
- a top-level `#[path]` resolves beside the declaring file, and a `#[path]` inside an inline
  module under the inline folders;
- an inline `mod <name> { … }` adds its name, or its own `path` value, as a folder for the
  declarations inside it.

A `cfg` guarding a declaration does not matter: the law asks whether any build reaches a file.
Every candidate path is folded lexically (`.` dropped, `..` folded), so a `#[path = "../tests/…"]`
compares equal to the file it names.

## Boundaries

- Depends on: `super::super::rust_module_references` (the comment and string blanking) and
  `regex`.
- Used by: `super::reachable_files`, which walks each member's module tree from its targets'
  root files.
- Rules: the reader knows no crate, member or folder name; a file a declaration names but the
  checkout lacks is simply not loaded.
