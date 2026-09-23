# README Contents Check

The parts of the readme-coverage gate that judge one README against its folder: the Contents block
parser, the entry name and glob matcher, and the pairing of entries with the folder's tracked
children.

## Contents

```text
tools_v2/xtask/src/verifications/documentation/readme_coverage/
├── contents_block.rs   finds the Contents block, checks its root line and reads its entry lines
├── entry_pattern.rs    compiles an entry's name or glob into a whole-name matcher
├── folder_matching.rs  pairs the entries with the folder's tracked children, kind by kind
└── tests/              unit tests for the three modules
```

## How it works

`contents_block.rs` returns either one violation (no heading, no block, a block that never closes)
or the block: its root line, its well-formed entries, and a violation for every malformed line.
`folder_matching.rs` then reports each tracked child that matches no entry or more than one, and each
entry that matches no child. The gate in `readme_coverage.rs` sorts both lists by line and prints
each as `path:line: message`. The grammar these modules implement is written out in the
[Documentation Gates README](/tools_v2/xtask/src/verifications/documentation/README.md).

## Boundaries

- Depends on: `markdown_fences.rs` for fences, the tracked tree's folder children, and the `regex`
  crate for globs.
- Used by: the readme-coverage gate alone.
- Rules: pure functions over text and names; nothing here reads the disk or runs git.
