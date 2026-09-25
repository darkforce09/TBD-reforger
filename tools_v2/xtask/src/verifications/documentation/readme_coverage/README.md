# README Contents check

The parts of the readme-coverage gate that judge one README against its folder: the Contents
block parser, the entry name and glob matcher, and the pairing of entries with the folder's
listed children.

## Contents

```text
tools_v2/xtask/src/verifications/documentation/readme_coverage/
├── contents_block.rs   finds the Contents block, checks its root line and reads its entry lines
├── entry_pattern.rs    compiles an entry's name or glob into a whole-name matcher
├── folder_matching.rs  pairs the entries with the folder's listed children, kind by kind
└── tests/              unit tests for the three modules
```

## How it works

`contents_block.rs` returns either one violation (no `## Contents` heading, no `text` block in
the section, a block that never closes) or the block: its root line, its well-formed entries,
and a violation for every malformed line. Each well-formed entry's token becomes an
`entry_pattern.rs` matcher: the exact name, or a glob compiled to an anchored regular expression.
`folder_matching.rs` then reports each listed child that matches no entry (at the root line) or
more than one, and each entry that matches no child. The gate in
`tools_v2/xtask/src/verifications/documentation/readme_coverage.rs` sorts the violations by line
and prints each as `path:line: message`. The grammar these modules implement is written out in
the [documentation gates README](/tools_v2/xtask/src/verifications/documentation/README.md#the-contents-grammar).

## Boundaries

- Depends on: `markdown_fences.rs` of the parent for fences, the folder children of the parent's
  tracked tree, and the `regex` crate for globs.
- Used by: the readme-coverage gate in the parent's `readme_coverage.rs` alone.
- Rules:
  - pure functions over text and names; nothing here reads the disk or runs git;
  - a nested line and a `/` inside a token are violations, never entries
    (`a_nested_line_is_a_violation_and_never_an_entry`, `a_slash_inside_an_entry_is_a_violation`);
    a line without a role is a violation whose token still lists its child
    (`an_entry_needs_two_spaces_before_a_non_empty_role`);
  - a malformed glob is refused with its reason, never compiled looser
    (`a_malformed_glob_is_refused_with_its_reason`);
  - a file entry matches only files and a folder entry only folders, and README.md is never a
    child (`a_file_entry_never_matches_a_folder_and_says_so`, `the_readme_is_never_a_child`).
