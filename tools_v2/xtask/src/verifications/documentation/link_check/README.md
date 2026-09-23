# Link Check Rules

The parts of the link-check gate below the pipeline: the Markdown scan, heading anchors, target
resolution, permalink lookups, the selection of judged documents, and the link rule itself.

## Contents

```text
tools_v2/xtask/src/verifications/documentation/link_check/
├── heading_anchors.rs        GitHub heading anchors: slug rules, numbered repeats, explicit anchors
├── inline_html.rs            HTML tags in a run: where a tag ends, and the anchors an <a> tag declares
├── judged_documents.rs       which tracked files are judged, and the area each belongs to
├── link_destination.rs       inline link tails, reference definition lines and reference labels
├── link_targets.rs           the link rule: checkout targets, fragments, references, link totals
├── markdown_inlines.rs       the inline pass: links, references, code spans, rendered heading text
├── markdown_lines.rs         the shape of one line: list markers, headings, breaks, quotes, tabs
├── markdown_scan.rs          the block pass: code and comments set aside, runs, definitions, headings
├── permalink_targets.rs      the link rule's permalinks: held for the run, settled in two git batches
├── repository_permalinks.rs  blob and tree views of this repository, and the git objects they name
├── target_resolution.rs      destination kinds, checkout path resolution, line anchors, fragment needs
└── tests/                    unit tests for every module here
```

## How it works

`markdown_scan.rs` walks a document's lines once, asking `markdown_lines.rs` what each line opens.
Front matter, fenced code blocks (found at a list item's content column too), indented code blocks
and HTML comments are set aside; every other paragraph, heading and table row becomes a run, and
reference definition lines are collected. `markdown_inlines.rs` then reads each run with the
defined labels: code spans, autolinks, HTML comments and tags (`inline_html.rs`) bind first, then
brackets pair into inline links, images and references. The
scan yields every destination with its line, the full and collapsed references whose label is
undefined, the headings' rendered text, the `<a id|name>` anchors, and the code spans and fenced
blocks.

`link_targets.rs` classifies each destination through `target_resolution.rs` and judges it: a
checkout path must resolve to a tracked file or a folder holding one, and a fragment must match a
heading anchor from `heading_anchors.rs` or fit a line anchor. A blob or tree view of this
repository pinned to a full commit id is a permalink, which `permalink_targets.rs` holds until the
run ends; then one `git cat-file --batch-check` looks every permalink up and one
`git cat-file --batch` reads the blobs a fragment needs, through `repository_permalinks.rs`. A
blob or tree view of a branch, a tag or an abbreviated commit breaks; every other page of this
repository is external. The rules are written out in the
[Documentation Gates README](/tools_v2/xtask/src/verifications/documentation/README.md).

## Boundaries

- Depends on: `markdown_fences.rs`, `path_regions.rs` and the tracked tree of the parent module,
  the layout constants, and `git cat-file` through `verification_core`'s process runner.
- Used by: the link-check gate in `link_check.rs`.
- Rules: the scan and the anchors are pure functions of text; only `link_targets.rs` reads target
  files, and only `repository_permalinks.rs` runs git.
