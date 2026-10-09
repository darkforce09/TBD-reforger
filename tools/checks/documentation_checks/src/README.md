# Documentation link check

The check that keeps documentation linked correctly: `link-check` (every link reaches what it
names, and every path and xtask command a live document writes as code exists).

## Contents

```text
tools/checks/documentation_checks/src/
├── gate_run.rs            what every gate shares: the request, preparation, the run and its printing
├── gate_scope.rs          the --path scope: normalises each value, refuses one naming no listed folder
├── lib.rs                 the crate root: the link-check module and its entry point
├── link_check/            the link check's scan, anchors, and link, backticked-path and command rules
├── link_check.rs          the link-check gate: the rule pipeline, one verdict per document, the totals
├── markdown_fences.rs     recognises the lines that open and close a fenced code block
├── path_regions.rs        where a path sits: code trees, the README span, exempt folders, size-exempt areas
├── prelude.rs             the names a caller imports: the entry point, the request, the vocabulary
├── tests/                 the link-check gate smoke tests and the fixture checkout they use
└── tracked_tree.rs        the files git ls-files lists, untracked ones under --with-untracked, and their folders
```

## How it works

```text
verify <gate> [--path <dir>]... [--with-untracked] [--report]
  └─▶ tracked_tree.rs: git ls-files -z  (+ git ls-files --others --exclude-standard -z)
        └─▶ gate_run.rs prepare: refuse a failed or empty listing; gate_scope.rs resolves --path
              └─▶ link_check.rs
                    └─▶ one verdict per judged item ─▶ verification_core::Report ─▶ exit 0, 1, 2
```

Each run lists the index once with `git ls-files -z` (`tracked_tree.rs`) and judges only what that
listing holds: a file on disk that git does not track is invisible, and a folder exists when it
holds a listed file. This committed view is the one CI judges. A listed file's text is read from
the working tree, so an uncommitted edit to a tracked file is judged as it stands; bytes that are
not UTF-8 are replaced, not refused.

With `--with-untracked`, the run also lists the untracked files git does not ignore, with
`git ls-files --others --exclude-standard -z`, and judges each exactly like a tracked file: the
folders it creates count, and links and backticked paths to it resolve. A file git ignores stays invisible, and a nested repository, which
that listing names as a folder without its files, is skipped. Wherever a rule below names a tracked
file, it then means a tracked or a listed untracked file. The header's scope line counts the
untracked files apart, and the summary line names the flag, as in
`link-check --with-untracked (untracked files included): OK`, so such a result is never
mistaken for a check of the committed files.

`prepare` in `gate_run.rs` refuses to judge anything when either listing cannot run, fails, is killed or passes its
120-second deadline, when the index lists no file, and when a `--path` value names no folder the
listing holds (`gate_scope.rs`); the gate then turns every judged item into one
`verification_core` verdict and prints them through the shared report.

Exit status: 0 when every judged item held, 1 when at least one broke a rule, 2 when a check did not
run: the listing failed or was empty, a judged file could not be read, the scope was refused, or the
scope selected nothing to judge.

### link-check

The judged documents are every tracked Markdown file under the documentation root, every tracked
README.md anywhere (the repository root's included), the project instructions (`PROJECT_INSTRUCTIONS`), the Markdown files directly in the
ticket folder (`TICKETS_DIR`), and the Markdown and `.mdc` files under the Cursor rule folders
(`CURSOR_RULE_DIRS`). Nothing in the agent artifact tree (`ARTIFACTS_DIR`: `.ai/artifacts`) is
judged, its README.md included. The ticket records and the archive are frozen: only the link rules
(1 to 7 below) judge them. Every other judged document is live, the pending-merge area included,
and rules 8 and 9 judge the live documents alone.

Each document is scanned the way a renderer reads it. Inline links and images, angle destinations,
autolinks, and reference definitions with their full, collapsed and shortcut uses count as links;
front matter, fenced and indented code blocks, HTML comments and inline code spans do not. Each
break names its rule:

1. missing target: a destination starting with `/` resolves from the repository root, any other
   from the document's folder, after percent-decoding and `.`/`..` normalisation; it must name a
   tracked file or a folder that holds one. An untracked file is missing unless `--with-untracked`
   lists it.
2. escapes repository: the path climbs above the repository root.
3. undefined reference: a full (`[text][label]`) or collapsed (`[text][]`) reference names a label
   the document never defines; a shortcut (`[text]`) without a definition is plain text, and so is
   a link with empty text before an undefined label (`[][label]`, as in `points[][2]`), which shows
   nothing to follow. An image with empty alt text (`![][label]`) still shows, so it keeps the rule.
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
   first `/`-separated segment is a tracked top-level folder of the checkout, or a retired
   top-level folder (`RETIRED_TOP_LEVEL_FOLDERS`: `docs`, `apps`) whether or not it still holds files, is a repository
   path. A span that holds whitespace (a command), `://` (a URL), `*`, `?` or `[` (a glob), `<` or
   `>` (a placeholder), `{` or `}` (a set), `$` (a variable), or `...` or `…` (an elision) is a
   pattern, counted and skipped. Any other such span loses a `#` fragment and a trailing `:N`,
   `:N-M` or `:N:M` line suffix, has its `.`, `..` and empty segments resolved, and must then name a
   tracked file or a folder that holds one; a trailing `/` asks for a folder. A path the tracked
   tree does not hold passes when git ignores it (runtime output such as a build tree, export
   scratch or a local secrets file) or when the layout module lists it as a historical spelling a
   live document names on purpose (`HISTORICAL_PATH_SPELLINGS`, each entry with its reason; the
   list is empty). Every path of a run the tracked tree does not hold is asked in one
   `git check-ignore --stdin -z`, a span without a trailing `/` both as written and as a folder, so
   a folder-only ignore pattern answers the same whether or not the folder exists on disk. A `..`
   that climbs above the repository root names nothing.
9. cited command does not exist: every `cargo xtask` in an inline code span, and in each line of a
   fenced code block whatever its info string, is a citation; `cargo` must stand as a word of its
   own, so `hcargo xtask` is none, and a fenced line ending in `\` continues on the next. The words
   after `cargo xtask` run up to a `#` comment, a pipe, `&&`, `;`, `&`, a redirection, the `)` of a
   command substitution, the closing quote of a string the citation sits in, or a word ending in
   `,`, `:` or `.` (the word counts without it), since a citation inside a sentence carries the
   sentence's punctuation and no subcommand name ends in one; quotes around a word are removed.
   The words are walked down xtask's own clap command tree, which the caller injects in a
   `CommandVocabulary` (the binary hands in `Cli`'s tree from `tools/xtask/src/cli/command_vocabulary.rs`)
   and which is built as clap builds it before parsing. A flag (`-x`, `--name`, `--name=value`) is skipped wherever it stands,
   together with the value an option of the command reached takes. While that command has
   subcommands, a placeholder (`<…>`, `[…]`, `{…}`, `…`, `...`) ends the walk without a break, and
   any other word must name a subcommand or one of its aliases. The next word stands as the first
   positional argument of the command reached: when that argument declares possible values (a
   `ValueEnum` type, or a list given to `value_parser`), the word must be one of them or an alias
   of one, and a placeholder passes. `mk` and `ci` take their first argument as a free string and
   look it up at run time, in the build recipes (`TARGETS` in
   `tools/commands/ci_task_catalog/src/build_lane/recipes.rs`) and in the CI task table (`TASKS` in
   `tools/commands/ci_task_catalog/src/task_definitions.rs`), so the tree declares those names as that argument's
   possible values: `cargo xtask mk leptos` and `cargo xtask ci ci-local` pass, and a recipe or
   task neither table holds breaks. An argument that declares no values, and every word after the
   first argument, is never judged, so `cargo xtask` and `cargo xtask --help` pass. The break names
   the path up to and including the first word that names no subcommand or no declared value.

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

- `cargo xtask verify link-check [--report] [--path <dir>]... [--with-untracked]`

`--path` is repeatable and repository-relative: the check judges only the files at or under the
named folders. `.` or the checkout root means
the whole repository, which is also the default. A value that climbs out with `..`, lies outside the
checkout, names a file or names no folder the listing holds is refused with exit 2.

`--with-untracked` adds the untracked files git does not ignore to what a gate judges, so new files
can be checked before they are committed. Without it a gate judges the committed view, which is what
CI runs. Both arguments come from one argument set
(`DocumentationGateArgs` in `tools/xtask/src/commands/verify/cli.rs`), which the dispatcher
turns into the `GateRequest` every gate takes. `--report` belongs to link-check alone.

## Boundaries

- Depends on: `verification_core` (verdicts, the shared report); `process_runner`, which runs
  `git ls-files`, `git cat-file` and `git check-ignore`; the `regex` crate for globs; `clap`, whose
  command tree a citation walks; the command
  vocabulary its caller injects (xtask's clap command tree, the build recipe names and the CI task
  names, declared as the values of `mk` and `ci`), never the command line itself; and the layout
  constants of the `repository_layout` crate.
- Used by: `tools/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify link-check`.
- Rules:
  - every path and region comes from the layout module;
  - a check that could not examine its input reports "did not run", never a pass
    (`an_unreadable_document_or_a_failed_listing_did_not_run`);
  - the judged areas are fixed in `link_check/judged_documents.rs`
    (`every_judged_area_is_judged_and_nothing_else`).

## Related documentation

- [README standard](/documentation/standards/readme_standard.md) — the README core, the kinds
  and how writers run these gates.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  documentation tree's layout, placement and size rules.
