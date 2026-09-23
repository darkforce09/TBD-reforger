# Documentation Gates

The repository checks that keep documentation where it belongs and shaped the same way everywhere:
`readme-coverage` (every folder carries a README.md whose Contents block matches the folder) and
`markdown-placement` (Markdown lives in the documentation tree, and live documents stay short).

## Contents

```text
tools_v2/xtask/src/verifications/documentation/
├── gate_scope.rs          the --path scope: normalises each value, refuses one naming no tracked folder
├── markdown_fences.rs     recognises the lines that open and close a fenced code block
├── markdown_placement.rs  the markdown-placement gate: code-tree Markdown, the retired root, the size limit
├── mod.rs                 registers the gates and holds what they share: preparation, reading, printing
├── path_regions.rs        where a path sits: code trees, the README span, skipped folders, exempt areas
├── readme_coverage/       the Contents block parser, entry names and globs, and child matching
├── readme_coverage.rs     the readme-coverage gate: README coverage and the Contents check
├── tests/                 unit tests, and the fixture checkout they share
└── tracked_tree.rs        the tracked files and folders that git ls-files lists
```

## How it works

Each run lists the index once with `git ls-files -z` (`tracked_tree.rs`) and judges only what that
listing holds: a file on disk that git does not track is invisible, and a folder exists when it holds
a tracked file. `mod.rs` refuses to judge anything when git is missing, fails, is killed or lists no
file, and when a `--path` value names no tracked folder (`gate_scope.rs`); each gate then turns every
judged item into one `verification_core` verdict and prints them through the shared report.

Exit status: 0 when every judged item held, 1 when at least one broke a rule, 2 when a check did not
run: the listing failed or was empty, a judged file could not be read, the scope was refused, or the
scope selected nothing to judge.

### readme-coverage

The README span is every tracked folder at or under the code trees (`CODE_TREES`: `apps`,
`tools_v2`, `contracts_v2`, `assets_v2`) and the documentation root (`DOCUMENTATION_ROOT`:
`documentation_v2`), the roots included, minus the pending-merge area (`PENDING_MERGE_DIR`) and
everything below it. The repository root's README.md lies outside the span.

1. Coverage: each folder in the span carries a tracked README.md, unless a component of its path is
   `tests`, `generated`, or begins with `.`.
2. Contents: every tracked README.md in the span, including one inside a skipped folder, passes the
   Contents grammar below.

### The Contents grammar

The Contents block is the first fenced code block whose info string is exactly `text` and that opens
after the `## Contents` heading and before the next `## ` heading. A heading or fence inside another
fenced block does not count. A README without the heading, a section without such a block, and a block
that never closes each fail.

- Root line: line 1 of the block is exactly the folder's repository-relative path followed by `/`;
  trailing whitespace is ignored.
- Entry lines: every other non-blank line is one direct-child entry, made of an optional tree-drawing
  prefix, the entry token, two or more spaces, and a non-empty role.
  - The prefix is the leading run of `├── `, `└── `, `│   ` and single spaces. A direct child's
    prefix is empty, one `├── ` or `└── `, or at most four spaces. A prefix that holds `│   `, a second
    branch or deeper indentation marks a nested line, which fails; so does a line that holds only
    tree-drawing characters.
  - The token is a name or a glob, and it runs up to the first two consecutive spaces. A folder entry
    ends in `/` and a file entry does not; any other `/` in the token fails, and `/` alone names
    nothing and fails. On a line without two consecutive spaces the token is the first
    space-separated word and the line has no role; a line whose two spaces are followed by nothing has
    no role either. A line without a role fails, though its token still lists its child.
  - Globs: `*` matches any run of characters, a leading dot included; `?` matches one character;
    `[…]` matches one character of a set, where `!` or `^` first negates it, `a-z` is a range and a
    `]` in first place is a member; `{a,b}` matches either alternative, and alternatives may nest and
    hold globs. A glob matches whole names only; an unclosed `[` or `{`, or a reversed range, fails.
- Matching: every tracked direct child except `README.md` matches exactly one entry of its own kind,
  a file against file entries and a folder against folder entries, and every entry matches at least one
  tracked child. Untracked and ignored files are invisible.

Every violation prints as `path:line: message`: a child that no entry matches is reported at the root
line, a missing heading at line 1, and every other violation at the line it concerns.

### markdown-placement

1. The code trees hold no tracked `.md` file, in any letter case, other than README.md, except below a
   `tests`, `generated` or `.`-prefixed folder.
2. The retired documentation root (`RETIRED_DOCS_ROOT`: `docs`) holds no tracked file.
3. Every tracked `.md` file, in any letter case, under the documentation root is at most 500 lines,
   except under the ticket records (`TICKET_DOCUMENTS_DIR`), the archive (`ARCHIVE_DIR`) and the
   pending-merge area, the program records whose path begins with `PROGRAM_RECORDS_PREFIX`, and the
   two documents that `cargo xtask ticket sync` rewrites between markers (`ROADMAP`,
   `GAP_ANALYSIS`), whose sync-managed tables stay in one file.

## Public surface

- `cargo xtask verify readme-coverage [--path <dir>]...`
- `cargo xtask verify markdown-placement [--path <dir>]...`

`--path` is repeatable and repository-relative: a gate judges only the folders (readme-coverage) or
files (markdown-placement) at or under the named folders. `.` or the checkout root means the whole
repository, which is also the default. A value that climbs out with `..`, lies outside the checkout,
names a file or names no tracked folder is refused with exit 2.

## Boundaries

- Depends on: `verification_core` (verdicts, the shared report, the process runner), `git ls-files`,
  the `regex` crate for globs, and the layout constants in `repository_layout.rs`.
- Used by: the verify command group, through `dispatch.rs`.
- Rules: every path comes from the layout module; a check that could not examine its input reports
  "did not run", never a pass; production files stay under 500 lines, and tests live in the sibling
  `tests/` folders.
