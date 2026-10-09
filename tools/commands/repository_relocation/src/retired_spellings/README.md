# Retired spelling verification parts

The two parts of the relocation verification
(`tools/commands/repository_relocation/src/retired_spellings.rs`) that make it one pass over the
tree, however many manifests it judges.

## Contents

```text
tools/commands/repository_relocation/src/retired_spellings/
├── spelling_matcher.rs  every manifest's retired spellings and prefix segments, and the one automaton over them
└── tree_text.rs         the tree's text files read once, their treatments per area set, allowed spans per treatment
```

## How it works

```text
TrackedTree ─▶ tree_text.rs (each text file read once)
manifests  ─▶ spelling_matcher.rs (distinct `path` spellings + `rust_path` segments ─▶ one automaton)
                    │
     per file: one scan ─▶ path starts, segments present ─▶ judged per retiring row
                                                            (its manifest's treatment, its scope)
```

`tree_text.rs` reads every text file of the judged tree once, in path order, computes each file's
treatment once per distinct set of frozen areas (a manifest that moves a frozen area judges with
its own set) and the allowed spans of a file once per treatment, on first use. A file that cannot
be read stops the run as a did-not-run for every manifest.

`spelling_matcher.rs` indexes the distinct `path` row spellings and the distinct `rust_path`
rewrites (one per `from` and `to`) with the rows that retire each, and builds one Aho-Corasick
automaton over the path spellings and the prefix segments. A scan of one file yields every start of
every path spelling, overlapping ones included, and the set of prefix segments the file spells;
the Rust path pass runs on the file only for a prefix all of whose segments it spells, since that
pass matches whole segments, each a slice of the file.

## Boundaries

- Depends on: `aho-corasick`; the crate's `file_treatment.rs`, `path_references::allowed_spans`,
  `repository_files.rs` and `rust_paths::path_rules`.
- Used by: `tools/commands/repository_relocation/src/retired_spellings.rs` only.
- Rules: the single pass gives every manifest the verdicts, notes and offence order the per-row
  judge gives it.
