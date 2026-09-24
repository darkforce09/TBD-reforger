**Status:** live

# README standard

Every folder in the code trees and in `documentation_v2/` carries a README.md built the same way: a
fixed core of sections, the sections its kind adds, and a Contents block that a gate checks against
the folder. The README tells developers and AI agents what a folder holds, how it works and where
it stops, and links the documents that go deeper.

## Which folders carry a README

The README span is every tracked folder at or under the code trees (`apps/`, `tools_v2/`,
`contracts_v2/`, `assets_v2/`) and the documentation root (`documentation_v2/`), the roots
included, minus the exempt folders and everything below them:

- a folder named `tests` or `generated`;
- a folder whose name begins with `.` (tool configuration);
- the pending-merge area, `pending_merge/` directly under the documentation root.

An exempt folder needs no README, and a README.md inside one is neither required nor checked; the
parent's Contents block describes the exempt folder in one line. The repository root's README.md
lies outside the span. A folder exists when it holds a tracked file, so a folder that git does not
track needs nothing.

## The README core

Every README holds these sections, in this order. A README under `documentation_v2/` starts with
the status line `**Status:** live` and a blank line above its title; a README in a code tree carries
no status line.

1. **Title.** An H1 with a human-readable name: no path, no backticks (`# Missions domain`, never a
   folder name such as `missions/`).
2. **Purpose.** One to three sentences under the title saying what the folder is for.
3. **`## Contents`.** The folder's direct children as a `text` tree, one line each with its role
   (see [The Contents block](#the-contents-block)).
4. **`## How it works`.** How the children work together: the flow of data and control through the
   folder, the main types, the invariants that span files, and an ASCII diagram in a `text` block
   when one helps. A folder with no child folders besides exempt ones and at most three files,
   whatever its kind, may leave this section out (README.md not counted).
5. **The kind sections.** The sections the folder's kind adds (see [Kinds](#kinds)), in the order
   the kind table lists them.
6. **`## Boundaries`.** Exactly three top-level bullets, in this order and spelled this way:
   - `Depends on:` what the folder uses: modules, crates, files, services and tools, read from its
     imports and calls.
   - `Used by:` everything outside the folder that uses it, found with `git grep` (callers, routes,
     tools); `nothing` when nothing does.
   - `Rules:` the invariants particular to this folder that a change must keep (layering,
     placement, generated files, limits), each with the test or gate that holds it wherever one
     does; repository-wide laws are not restated.

   A bullet may run over several lines and may hold a nested list.
7. **`## Related documentation`.** Repository-root links to the documents that go deeper into this
   folder, each followed by a few words on what it covers. Left out when there are none.

Headings are unnumbered and spelled exactly as above. A README holds no other `##` heading; finer
structure goes into `###` headings inside How it works or a kind section. A kind section with
nothing to hold keeps its heading and says so in one line (`None: the folder serves no route.`), so
every README of a kind has the same headings.

## The Contents block

`cargo xtask verify readme-coverage` reads the Contents block and checks it against the folder. The
grammar below is stated exactly as the gate's own
[README](/tools_v2/xtask/src/verifications/documentation/README.md#the-contents-grammar) states it,
and the gate's code is the final word.

The Contents block is the first fenced code block whose info string is exactly `text` and that opens
after the `## Contents` heading and before the next `## ` heading. A heading or fence inside another
fenced block does not count. A README without the heading, a section without such a block, and a
block that never closes each fail.

- Root line: line 1 of the block is exactly the folder's repository-relative path followed by `/`;
  trailing whitespace is ignored.
- Spacer lines: a line after the root line made only of whitespace and the tree-drawing characters
  `├`, `└`, `│` and `─` lists nothing and is ignored, whether it is blank or a spacer such as `│` or
  `│   │`.
- Entry lines: every other line is one direct-child entry, made of an optional tree-drawing prefix,
  the entry token, two or more spaces, and a non-empty role.
  - The prefix is the leading run of `├── `, `└── `, `│   ` and single spaces. A direct child's
    prefix is empty, one `├── ` or `└── `, or at most four spaces. A prefix that holds `│   `, a
    second branch or deeper indentation marks a nested line, which fails.
  - The token is a name or a glob, and it runs up to the first two consecutive spaces. A folder
    entry ends in `/` and a file entry does not; any other `/` in the token fails, and `/` alone
    names nothing and fails. On a line without two consecutive spaces the token is the first
    space-separated word and the line has no role; a line whose two spaces are followed by nothing
    has no role either. A line without a role fails, though its token still lists its child.
  - Globs: `*` matches any run of characters, a leading dot included; `?` matches one character;
    `[…]` matches one character of a set, where `!` or `^` first negates it, `a-z` is a range and a
    `]` in first place is a member; `{a,b}` matches either alternative, and alternatives may nest
    and hold globs. A glob matches whole names only; an unclosed `[` or `{`, or a reversed range,
    fails.
- Matching: every tracked direct child except `README.md` matches exactly one entry of its own kind,
  a file against file entries and a folder against folder entries, and every entry matches at least
  one tracked child. Untracked and ignored files are invisible.

Every violation prints as `path:line: message`: a child that no entry matches is reported at the
root line, a missing heading at line 1, and every other violation at the line it concerns.

### Writing the block

The gate checks the grammar; these conventions keep every block readable the same way.

- Draw the tree: `├── ` before each entry and `└── ` before the last. Entries run in name order,
  and the roles line up in one column. Name order is case-insensitive: it compares the lowercased
  names by character code, puts a name before every longer name that starts with it, and compares
  a folder without its `/`. So `.gitignore` comes before `Cargo.toml`, `mod.rs` before `models/`,
  and `mission_editor/` before `mission_editor.rs`.
- A role says what the child is for, not what it is made of: a lowercase phrase with no closing
  period, short enough that the line stays within about 100 characters.
- List dot-files: `.gitignore` and `.env.example` are tracked children like any other.
- `tests/` and `generated/` get one line each; their insides are exempt.
- A homogeneous collection gets one glob line, such as `*.sql` in a migrations folder. A child must
  never match both a glob and a name, since each child matches exactly one entry.
- A folder that holds only its README.md has a block with the root line alone.
- Keep diagrams and other `text` blocks out of the Contents section: the gate reads the first one
  after the heading as the tree.

```text
apps/website/map-engine/src/spatial/los/interior/
├── mod.rs     declares both modules, compiled only with the `io` feature
├── tests/     unit tests for the walker and the wash
├── walker.rs  observer-to-target traces through a compound building, with blocking and concealment
└── wash.rs    per-floor visibility rasters around an observer, whole or in budgeted batches
```

## Kinds

A folder's kind decides which sections follow How it works. Each kind has a template, named in the
table, with a skeleton and a worked sample written from a real folder; the
[templates folder](/documentation_v2/standards/templates/README.md) lists the templates it holds.

| Kind | Kind sections, in order | Template | Examples |
|---|---|---|---|
| area root | Getting started | `readme_area_root.md` | `apps/website/`, `apps/mod/`, `tools_v2/` |
| crate, package or addon root | Getting started, Configuration, Public surface | `readme_crate_root.md` | `apps/website/api_v2/`, `tools_v2/enfusion_mcp_node_package/`, `apps/mod/tbd-framework/` |
| domain or subsystem | Public surface | `readme_domain.md` | `apps/website/api_v2/src/missions/`, `apps/website/map-engine/src/spatial/` |
| leaf | none | `readme_leaf.md` | `apps/website/map-engine/src/spatial/los/interior/` |
| page | Routes, Data, States | `readme_page.md` | `apps/website/frontend/src/v2/pages/operations/schedule/` |
| app | Routes, Public surface | `readme_app.md` | `apps/website/frontend/src/v2/apps/editor/` |
| command-line | Commands | `readme_command_line.md` | `tools_v2/developer-tools/src/bin/`, `tools_v2/xtask/src/commands/db/` |
| data (contracts, assets, fixtures, migrations, seeds) | Format, Producers and consumers | `readme_data.md` | `contracts_v2/fixtures/missions/`, `apps/website/api_v2/migrations/` |
| mod scripts | Authority | `readme_mod_scripts.md` | `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/` |
| mod assets | Format, Referenced by | `readme_mod_assets.md` | `apps/mod/tbd-framework/Prefabs/` |
| deploy or config | Configuration, Installed by | `readme_deploy_config.md` | `tools_v2/xtask/deploy/` |
| documentation_v2 folder | Code | `readme_documentation_folder.md` | `documentation_v2/runbooks/` |

### What each kind section holds

- **`## Getting started`**: the few commands, run from the repository root, that build, run or test
  what the folder holds, each with what to expect; a `cargo xtask` command wherever one exists. The
  commands are listed in the order they must run; a command that stays in the foreground (a server,
  a watcher) says so, and a command that waits for an earlier one says what it waits for. The full
  procedure lives in a runbook, linked under Related documentation.
- **`## Configuration`**: every setting the folder's code or files read (environment variables,
  config files, feature flags, profile keys), with its default, whether it is required, and the
  file that reads it.
- **`## Public surface`**: what code outside the folder may use: the modules, types and functions
  other folders import, the binaries, the HTTP routes the folder owns. Only what crosses the folder's
  boundary, never an inventory of everything marked `pub`.
- **`## Routes`**: each browser route the folder renders: path, component, access tier and layout
  flags, as `apps/website/frontend/src/app_routes.rs` and `apps/website/frontend/src/router.rs`
  declare them.
- **`## Data`**: each API call the page makes (method, path, the DTO it reads or sends), the context
  and storage it reads, and what it writes.
- **`## States`**: each state the viewer can see (restoring, signed out, loading, empty, failed,
  loaded and the rest), with what shows on screen and what moves the page between them.
- **`## Commands`**: each command the folder defines: its synopsis, what it does, its exit codes and
  one example, as the command's own argument parser declares it. A command that stays in the
  foreground says so, and one that needs another running first names it.
- **`## Format`**: the file format: encoding, the schema it follows (linked in `contracts_v2/`), the
  naming convention, and how to add a file.
- **`## Producers and consumers`**: what writes the files (a command, a tool, a person) and what
  reads them (loaders, tests, the game), each with its path.
- **`## Authority`**: where each script's behaviour runs: server, client or owner, the RPCs with
  their reliability and receivers, and the replicated properties, matching the `@authority`, `@rpc`
  and `@replicated` tags in the scripts.
- **`## Referenced by`**: what refers to these assets (prefabs, configs, layouts, scripts, mission
  headers) and how: by resource GUID, by path or by class.
- **`## Installed by`**: what puts these files to work and where: the `cargo xtask` command or the
  manual step, and the place on the host or server they end up.
- **`## Code`**: the code folders the documents describe, as repository-root links.

### Picking a kind

Go down this list and take the first kind that fits.

1. **documentation_v2 folder**: any folder under `documentation_v2/`.
2. **area root**: the top of a code tree, or a folder that groups several products without being
   one (`apps/`, `apps/website/`, `apps/mod/`, `tools_v2/`, `contracts_v2/`, `assets_v2/`).
3. **crate, package or addon root**: the folder that holds a `Cargo.toml`, a `package.json` or an
   Enfusion `addon.gproj` (`apps/website/api_v2/`, `apps/ticketboard/`,
   `tools_v2/enfusion_mcp_node_package/`, `apps/mod/tbd-framework/`).
4. **mod scripts**: a folder at or under an addon's `Scripts/`
   (`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/AI/`).
5. **mod assets**: any other folder inside an addon (`apps/mod/tbd-framework/Prefabs/`,
   `apps/mod/tbd-framework/Configs/`).
6. **deploy or config**: templates, service units and profiles that set up a host or a server
   (`tools_v2/xtask/deploy/`, `tools_v2/xtask/dedicated_server_profiles/`).
7. **data**: schemas, fixtures, migrations, seeds and asset data that code reads rather than runs
   (`contracts_v2/definitions/`, `apps/website/api_v2/seeds/`, `assets_v2/terrains/`,
   `apps/website/shared/`).
8. **command-line**: a crate's `src/bin/`, and each folder directly under
   `tools_v2/xtask/src/commands/` (`tools_v2/developer-tools/src/bin/`,
   `apps/website/api_v2/src/bin/`, `tools_v2/xtask/src/commands/db/`). Any other folder that
   parses or runs commands, such as a folder inside a command group, is a domain or a leaf and
   links the command-line README it belongs to.
9. **app**: a workspace directly under `apps/website/frontend/src/v2/apps/`.
10. **page**: a folder under `apps/website/frontend/src/v2/pages/` that holds a route component.
11. **domain or subsystem**: any other folder that has child folders besides exempt ones.
12. **leaf**: any other folder.

### A folder that is two kinds

The first kind that fits sets the sections. When the folder also holds, directly, files of a later
kind that a reader configures or runs by hand, the README adds that kind's sections after the first
kind's, in kind-table order, each heading once, covering only those files. `apps/website/` is an
area root that also holds the API's release `Dockerfile` and the staging compose file, so its README
adds Configuration and Installed by after Getting started, as the
[area root template](/documentation_v2/standards/templates/readme_area_root.md) shows. A file nobody
configures or runs by hand, such as a crate's `rustfmt.toml`, needs only its Contents line.

## Writing rules

- **Truth.** Every claim is checked against the code it describes: paths with `git ls-files`, routes
  in `apps/website/frontend/src/app_routes.rs` and `apps/website/api_v2/src/<domain>/routes.rs`,
  commands in the xtask command tree (`tools_v2/xtask/src/cli/` and
  `tools_v2/xtask/src/commands/<group>/cli.rs`) and safe `--help` runs,
  environment variables in `apps/website/api_v2/.env.example` and the code that reads them, callers
  with `git grep`. When a document and the code disagree, the code wins.
- **Present tense.** A README says what the folder is and does. It holds no history (no dates, no
  "formerly", "previously", "legacy", "migrated" or "renamed from", no phase or wave story) and no
  plans; commit history owns the past, and feature docs own open work.
- **No tickets.** A README never names or links a ticket.
- **Links.** Repository-root links, such as `[API overview](/documentation_v2/website/api_v2/api_overview.md)`,
  never `../` climbs.
- **Paths.** A backticked path to anything outside the README's folder is a full repository path;
  inside the folder, a path relative to it (`src/bin/api.rs`); a bare file name only after the
  same paragraph gave that file's or its folder's full path. `link-check` verifies every backticked
  full repository path outside a fenced block of a live document.
- **Terminology.** The editor is the Mission Creator. The document a mission maker authors is a
  mission, and Enfusion's world plus game-mode configuration is the mission header. An event is a
  scheduled session record (its time, missions, ORBAT slots, sign-ups and waitlist), and operations
  is the domain around events, the ORBAT and service records. Code identifiers keep their spelling
  (`scenario`, `EventHub`), and interface text is quoted as the code writes it. The glossary at the
  documentation root defines these terms, one `###` entry per term; a README links a term's first
  use to its entry, as `[mission](/documentation_v2/glossary.md#mission)`, whenever the glossary
  holds one.
- **Parents and children.** A parent summarises each child in its one Contents line and, where it
  helps, one clause in How it works. Its kind sections state what holds at its own boundary or
  across all its children (an item, route or setting that crosses the boundary; a format every
  child shares), one line each even when a child defines it; the detail stays in the child's
  README. A parent never inventories a child: no nested trees, no list of a child's files, no item
  that stays inside it.
- **README or feature doc.** The README describes its own folder at a high level: what is here, how
  it fits together, how to use it and where it stops. Behaviour specifications, interface and UX
  design, decisions, roadmaps, research and evidence live in feature docs in the matching folder
  under `documentation_v2/`, and the README links them under Related documentation. A page's or
  app's README holds what the code declares: routes, calls with their DTOs, states with their exact
  text. Its feature doc holds the flows, rules and reasons, what each call means server-side,
  design, open work and decisions, and links the README's Routes, Data and States instead of
  repeating them.
- **Same commit.** A code change updates, in the same commit, the README of every folder whose
  Contents line, surface, commands or boundaries it changes.
- **Frozen trees.** The README indexes inside `documentation_v2/tickets/` and
  `documentation_v2/archive/` are live documents (`**Status:** live`, updated as files land),
  although the trees they index are frozen. `link-check` and `markdown-placement` judge everything
  under those two trees as frozen, with no path, command or size check, so the writer checks an
  index's backticked paths, commands and length by hand.
- **Diagrams.** ASCII, in `text` blocks, placed in How it works or a kind section.
- **Names.** Code identifiers go in backticks exactly as spelled; everything else is plain words.
- **Hosts and paths.** No IP address of a host, and no personal absolute path. The deploy host is
  named by `TBD_SSH_HOST` in `tools_v2/xtask/deploy/deploy.env`.
- **Length.** A leaf runs about 10 to 60 lines, a domain or subsystem 40 to 200, a root up to 400.
  No README passes 500 lines; one that needs more moves the depth into a feature doc and links it.

## Writing a README

1. List the folder's tracked direct children, each folder with a trailing `/`, with
   `git -C <folder> ls-files | sed 's|/.*|/|' | sort -u`; the list holds README.md, which Contents
   leaves out, and Contents puts the rest in name order. Pick the folder's kind.
2. Read enough code to be exact: the module docs, the public items, the module wiring, the imports
   (for Depends on) and the callers found with `git grep` (for Used by). For scripts, read the class
   names, attributes, RPCs and replicated properties; for data, read the loader or schema that
   consumes it.
3. Copy the kind's skeleton from its template and fill every placeholder from what the code shows.
   Where the folder is two kinds, add the second kind's sections.
4. Rewriting an existing README, carry every fact that is still true, and drop boilerplate, bare
   file lists, history and every claim the code contradicts.
5. Run the gates over the folder.

## Gates

Three gates check documentation. Each takes a repeatable `--path <dir>` that limits it to part of the
repository, and exits 0 when every judged item held, 1 when at least one broke a rule, and 2 when a
check could not run. The [documentation gates README](/tools_v2/xtask/src/verifications/documentation/README.md)
specifies every rule.

| Gate | What it checks |
|---|---|
| `cargo xtask verify readme-coverage` | every folder in the README span has a tracked README.md, and every README.md in the span passes the Contents grammar |
| `cargo xtask verify markdown-placement` | the code trees hold no Markdown besides README.md (outside `tests`, `generated` and dot-folders), the retired documentation root holds no file, and every live document under `documentation_v2/` stays within 500 lines, apart from the exemptions the gates README lists |
| `cargo xtask verify link-check` | every link in every README and documentation file reaches a tracked target, anchors and line ranges included; in live documents, every backticked repository path exists and every cited `cargo xtask` command exists in the command tree, fenced blocks included |

The Contents check is the only structural rule a gate enforces. The section order, the kind
sections and the writing rules are held by the writers and reviewers who apply this standard.
`cargo xtask verify readme-coverage` is the one README checker: a crate's own tests check its module
layout, never the contents of its READMEs.

Before committing a README change, run the gates over the folders it touches:

```bash
cargo xtask verify readme-coverage --path <folder>
cargo xtask verify link-check --path <folder>
cargo xtask verify markdown-placement
```

Writers check new, uncommitted files with `--with-untracked` before handing over; the committed
view, without the flag, is what CI judges.
