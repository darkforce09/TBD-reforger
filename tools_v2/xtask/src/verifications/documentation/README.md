# Documentation Gates

The repository checks that keep documentation where it belongs, shaped the same way everywhere and
linked correctly: `readme-coverage` (every folder carries a README.md whose Contents block matches
the folder), `markdown-placement` (Markdown lives in the documentation tree, and live documents stay
short) and `link-check` (every link in the documentation reaches what it names, and every path and
xtask command a live document writes as code exists).

## Contents

```text
tools_v2/xtask/src/verifications/documentation/
├── gate_scope.rs          the --path scope: normalises each value, refuses one naming no tracked folder
├── link_check/            the link check's scan, anchors, and link, backticked-path and command rules
├── link_check.rs          the link-check gate: the rule pipeline, one verdict per document, the totals
├── markdown_fences.rs     recognises the lines that open and close a fenced code block
├── markdown_placement.rs  the markdown-placement gate: code-tree Markdown, the retired root, the size limit
├── mod.rs                 registers the gates and holds what they share: preparation, reading, printing
├── path_regions.rs        where a path sits: code trees, the README span, exempt folders, size-exempt areas
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
`documentation_v2`), the roots included, minus the exempt folders and everything below them: a
folder named `tests` or `generated`, a folder whose name begins with `.`, and the pending-merge area
(`PENDING_MERGE_DIR`). The repository root's README.md lies outside the span. Both rules judge the
span alone, so a README.md inside an exempt folder is neither required nor checked.

1. Coverage: each folder in the span carries a tracked README.md.
2. Contents: every tracked README.md in the span passes the Contents grammar below.

### The Contents grammar

The Contents block is the first fenced code block whose info string is exactly `text` and that opens
after the `## Contents` heading and before the next `## ` heading. A heading or fence inside another
fenced block does not count. A README without the heading, a section without such a block, and a block
that never closes each fail.

- Root line: line 1 of the block is exactly the folder's repository-relative path followed by `/`;
  trailing whitespace is ignored.
- Spacer lines: a line after the root line made only of whitespace and the tree-drawing characters
  `├`, `└`, `│` and `─` lists nothing and is ignored, whether it is blank or a spacer such as `│` or
  `│   │`.
- Entry lines: every other line is one direct-child entry, made of an optional tree-drawing prefix,
  the entry token, two or more spaces, and a non-empty role.
  - The prefix is the leading run of `├── `, `└── `, `│   ` and single spaces. A direct child's
    prefix is empty, one `├── ` or `└── `, or at most four spaces. A prefix that holds `│   `, a second
    branch or deeper indentation marks a nested line, which fails.
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

### link-check

The judged documents are every tracked Markdown file under the documentation root except the
program records (`PROGRAM_RECORDS_PREFIX`), every tracked README.md anywhere (the repository root's
included), the project instructions (`PROJECT_INSTRUCTIONS`), the Markdown files directly in the
ticket folder (`TICKETS_DIR`), and the Markdown and `.mdc` files under the Cursor rule folders
(`CURSOR_RULE_DIRS`). Nothing in the agent artifact tree (`ARTIFACTS_DIR`: `.ai/artifacts`) is
judged, its README.md included. The ticket records and the archive are frozen: only the link rules
(1 to 7 below) judge them. Every other judged document is live, and rules 8 and 9 judge the live
documents alone.

Each document is scanned the way a renderer reads it. Inline links and images, angle destinations,
autolinks, and reference definitions with their full, collapsed and shortcut uses count as links;
front matter, fenced and indented code blocks, HTML comments and inline code spans do not. Each
break names its rule:

1. missing target: a destination starting with `/` resolves from the repository root, any other
   from the document's folder, after percent-decoding and `.`/`..` normalisation; it must name a
   tracked file or a folder that holds one. An untracked file is missing.
2. escapes repository: the path climbs above the repository root.
3. undefined reference: a full (`[text][label]`) or collapsed (`[text][]`) reference names a label
   the document never defines; a shortcut (`[text]`) without a definition is plain text.
4. missing anchor: a fragment on a rendered Markdown target (or `#fragment` alone, on the document
   itself) matches none of its anchors. A heading's anchor is its rendered text lowercased, with
   every character other than a letter, digit, space, hyphen or underscore dropped and each space
   made a hyphen; a repeat gets `-1`, `-2` in document order; setext headings count, headings in
   code do not, and every `<a id>` or `<a name>` adds an anchor. A fragment on a folder (a tree
   permalink included), or any fragment but a line anchor on a file GitHub does not render as
   Markdown, is missing too.
5. line anchor out of range: `#L<n>` or `#L<n>-L<m>` on a file shown as text (any file but
   Markdown, or Markdown with `?plain=1`) must lie within its line count.
6. non-permalink repository URL: a blob or tree view of this repository that names a branch, a
   tag or an abbreviated commit instead of the full commit id.
7. unknown permalink object: the local history holds nothing at a permalink's `<commit>:<path>`,
   or, for a tree view, something other than a folder. A permalink is a blob view
   `PERMALINK_BASE<commit>/<path>` or a tree view (`tree/` in place of `blob/`; the commit alone
   names the repository root) with the full 40- or 64-character commit id; a blob view also holds
   on a folder, which GitHub opens as its tree view. All permalinks of a run, of both views, are
   looked up in one `git cat-file --batch-check`, and the blobs a fragment needs are read in one
   `git cat-file --batch`, so a shallow clone reports older commits as unknown.
8. backticked path names nothing: an inline code span (fenced blocks are not read for paths) whose
   first `/`-separated segment is a tracked top-level folder of the checkout, or the retired
   documentation root (`RETIRED_DOCS_ROOT`) whether or not it still holds files, is a repository
   path. A span that holds whitespace (a command), `://` (a URL), `*`, `?` or `[` (a glob), `<` or
   `>` (a placeholder), `{` or `}` (a set), `$` (a variable), or `...` or `…` (an elision) is a
   pattern, counted and skipped. Any other such span loses a `#` fragment and a trailing `:N`,
   `:N-M` or `:N:M` line suffix, has its `.`, `..` and empty segments resolved, and must then name a
   tracked file or a folder that holds one; a trailing `/` asks for a folder. A path the tracked
   tree does not hold passes when git ignores it (runtime output such as a build tree, export
   scratch or a local secrets file) or when the layout module lists it as a historical spelling a
   live document names on purpose (`HISTORICAL_PATH_SPELLINGS`, each entry with its reason). Every
   path of a run the tracked tree does not hold is asked in one `git check-ignore --stdin -z`, a
   span without a trailing `/` both as written and as a folder, so a folder-only ignore pattern
   answers the same whether or not the folder exists on disk. A `..` that climbs above the
   repository root names nothing.
9. cited command does not exist: every `cargo xtask` in an inline code span, and in each line of a
   fenced code block whatever its info string, is a citation; `cargo` must stand as a word of its
   own, so `hcargo xtask` is none, and a fenced line ending in `\` continues on the next. The words
   after `cargo xtask` run up to a `#` comment, a pipe, `&&`, `;`, `&`, a redirection, the `)` of a
   command substitution, the closing quote of a string the citation sits in, or a word ending in
   `,`, `:` or `.` (the word counts without it), since a citation inside a sentence carries the
   sentence's punctuation and no subcommand name ends in one; quotes around a word are removed.
   The words are walked down xtask's own clap command tree (`crate::cli::Cli`, built as clap builds
   it before parsing): while the command reached has subcommands, a flag (`-x`, `--name`,
   `--name=value`) is skipped together with the value an option of that command takes, a
   placeholder (`<…>`, `[…]`, `{…}`, `…`, `...`) ends the walk without a break, and any other word
   must name a subcommand or one of its aliases. The words after a leaf command are its arguments
   and are never judged, so `cargo xtask`, `cargo xtask --help` and every target of `mk` and `ci`
   pass. The break names the path up to and including the first word that names no subcommand.

External destinations are counted and never fetched: any other scheme or host, and every page of
this repository other than a blob or tree view, such as its home page (with or without a trailing
`/`), issues, pull requests, actions, releases, wiki, commits and comparisons. Every break prints
as `path:line: rule: message`. Without `--report` the gate prints every failing document with its
break count, the first 20 breaks in full, and the totals; with `--report` it prints every break.
The totals count documents, links by kind, backticked paths by outcome, command citations by
outcome, breaks by rule, and breaks by area: the documentation root's live documents and frozen
records, the ticket folder, the Cursor rules, the project instructions, and the other READMEs. A
failed ignore batch is one "did not run" verdict for the paths it held, never a pass or a break.

A rule is a `DocumentRule` in `link_check.rs`: it says which areas it judges, judges one scanned
document at a time, settles any batched work when the run ends, and adds its own totals lines. A
new rule joins the list in `verify_link_check` and reads the scan it is given.

## Public surface

- `cargo xtask verify readme-coverage [--path <dir>]...`
- `cargo xtask verify markdown-placement [--path <dir>]...`
- `cargo xtask verify link-check [--report] [--path <dir>]...`

`--path` is repeatable and repository-relative: a gate judges only the folders (readme-coverage) or
files (markdown-placement, link-check) at or under the named folders. `.` or the checkout root means
the whole repository, which is also the default. A value that climbs out with `..`, lies outside the
checkout, names a file or names no tracked folder is refused with exit 2.

## Boundaries

- Depends on: `verification_core` (verdicts, the shared report, the process runner), `git ls-files`,
  `git cat-file` and `git check-ignore`, the `regex` crate for globs, xtask's own clap command tree
  (`crate::cli::Cli`, read through `clap::CommandFactory`), and the layout constants in
  `repository_layout.rs`.
- Used by: the verify command group, through `dispatch.rs`.
- Rules: every path comes from the layout module; a check that could not examine its input reports
  "did not run", never a pass; production files stay under 500 lines, and tests live in the sibling
  `tests/` folders.
