**Status:** live

# Documentation

The documentation of the TBD Reforger platform: feature docs, runbooks, standards, design
references, known bugs, the product roadmap, ticket specs and plans, and the archive, laid out as a
mirror of the code. Start here to find the document that covers a subject and to learn which source
wins when two disagree.

The [workspace layout](/documentation/architecture/workspace_layout.md) describes the repository as
it stands. A workspace restructure program is active: its plan, target file tree and progress
tracker are in [restructure/](/documentation/restructure/README.md).

## Contents

```text
documentation/
├── apps/                    documents on the products in apps/: the API, the app, the host agent, ticketboard
├── architecture/            the workspace as it stands: top-level folders, members, where everything lives
├── archive/                 frozen history, one folder per topic
├── assets/                  documents on the terrain export and the map data in assets/
├── contracts/               documents on the contracts in contracts/
├── design_system/           design tokens, symbology and interaction patterns the website and mod share
├── glossary/                the project's terms and abbreviations, split by first letter
├── known_bugs/              the live registry of known bugs
├── legacy/                  documents on the map and graphics engines parked in legacy/
├── mod/                     documents on the Enfusion mod suite in apps/mod/
├── product_roadmap.md       the planned product items by area and the open product questions
├── restructure/             the active workspace restructure program: plan, target tree, progress
├── runbooks/                operator procedures: development, deployment, gates, playtests
├── standards/               documentation and code standards, and the templates
├── tickets/                 ticket specs and plans, flat, frozen once the ticket closes
└── tools/                   documents on the developer tools in tools/
```

## How it works

Two layers document the code. The README.md in each code folder says what the folder holds, how
it fits together and where it stops; the documents here go deeper, and each code README links
them. A document about code sits here at the code's path without `src/`: the documents on
`tools/developer_tools/` are in `documentation/tools/developer_tools/`, and the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) in `apps/fleet_host_agent/` is
documented in `apps/fleet_host_agent/`. Two code trees keep a shorter document path until the
[restructure](/documentation/restructure/README.md) reshapes them: a document about the single-page
app also leaves out `src/v2/`, so the [event](/documentation/glossary/a_to_f.md#event) schedule page
in `apps/frontend/src/v2/pages/operations/schedule/` is documented in
`documentation/apps/frontend/pages/operations/schedule/` and all
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) material sits in
`documentation/apps/frontend/apps/editor/`; a document about the mod leaves out `apps/` and `Scripts/Game/TBD/`
and sits in `mod/`. What spans the code has a top-level folder of its
own: `architecture/`, `runbooks/`, `standards/`, `design_system/`, `known_bugs/`, `tickets/` and
`archive/`, with the `glossary/` folder and `product_roadmap.md` beside them, and the active
program has `restructure/`. The
[documentation standards](/documentation/standards/documentation_standards.md) set the layout,
names and lifecycle; the [README standard](/documentation/standards/readme_standard.md) shapes
every README.

```text
code folder README ──links──▶ documentation/<code path>/   feature docs, evidence, visual references
                                   │ first use of a term ──▶ glossary/
                                   │ open work ────────────▶ .ai/tickets/ ──spec, plan──▶ tickets/
                                   └ history ──────────────▶ archive/<topic>/
```

Every document opens with its status line. A live document tracks the code and changes in the same
commit as the code it describes. A frozen record (the spec or plan of a closed
[ticket](/documentation/glossary/n_to_z.md#ticket)) and an archived document keep their words; only
their links change.

### Authority ladder

When two sources disagree, the higher one wins and the lower one is corrected:

1. The running code.
2. [CLAUDE.md](/CLAUDE.md): the project laws, the directory atlas and the canonical commands.
3. This README: the map of the documentation.
4. The [documentation standards](/documentation/standards/documentation_standards.md), the
   [README standard](/documentation/standards/readme_standard.md), the
   [templates](/documentation/standards/templates/README.md) and the other standards.
5. Feature docs, runbooks, the product roadmap and the other live documents.
6. Frozen specs and plans under `tickets/`.
7. The archive, which records history and is never current.

### Where to find what

| To find | Look in |
|---|---|
| what a code folder holds and how to use it | the README.md in that folder |
| the top-level folders, the workspace members and where code, contracts, assets and documents live | the [workspace layout](/documentation/architecture/workspace_layout.md) |
| a web page's behaviour, design, open work and decisions | `apps/frontend/pages/<area>/<page>/`, indexed by the [frontend README](/documentation/apps/frontend/README.md) |
| the Mission Creator: features, roadmap, UX decisions, Eden reference | [apps/frontend/apps/editor/](/documentation/apps/frontend/apps/editor/README.md) |
| the [API](/documentation/glossary/a_to_f.md#api)'s areas and its verification evidence | [apps/api/](/documentation/apps/api/README.md), starting at the [API overview](/documentation/apps/api/api_overview.md) |
| the map engine and the graphics engine | [legacy/](/documentation/legacy/README.md) |
| the [mod](/documentation/glossary/g_to_m.md#mod)'s design, screens and export evidence | [mod/](/documentation/mod/README.md) |
| how a terrain becomes the map data the platform serves | [assets/](/documentation/assets/README.md) |
| how a game host carries out server commands | [apps/fleet_host_agent/](/documentation/apps/fleet_host_agent/README.md) |
| the ticket viewer | [apps/ticketboard/](/documentation/apps/ticketboard/README.md) |
| the developer tools and the contracts | [tools/](/documentation/tools/README.md) and [contracts/](/documentation/contracts/README.md) |
| how to run, test, deploy or play-test anything | [runbooks/](/documentation/runbooks/README.md), starting at [local development](/documentation/runbooks/local_development.md) |
| the rules for code, comments, documents and commits | [standards/](/documentation/standards/README.md) |
| a term or abbreviation | the [glossary](/documentation/glossary/README.md) |
| design tokens, symbology and interaction patterns | [design_system/](/documentation/design_system/README.md) |
| a known bug and its workaround | [known_bugs/](/documentation/known_bugs/README.md) |
| what the product plans to build, and the open product questions | the [product roadmap](/documentation/product_roadmap.md) |
| what to work on next | the ticket registry: `cargo xtask ticket next`, `.ai/tickets/queue.json` or [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) |
| a ticket's spec or plan | [tickets/](/documentation/tickets/README.md); the ticket itself is `.ai/tickets/T-<id>.toml` |
| why something was built the way it was, or what came before | [archive/](/documentation/archive/README.md) and the commit history |

## Code

- [Applications](/apps/README.md) — the API, the app, the service worker, the host agent and
  ticketboard, documented under `apps/`.
- [Parked engines](/legacy/README.md) — the map and graphics engines, documented under `legacy/`.
- [Mod suite](/apps/mod/README.md) — documented under `mod/`.
- [Developer tools](/tools/README.md) — documented under `documentation/tools/`.
- [Contracts](/contracts/README.md) — documented under `documentation/contracts/`.
- [Assets](/assets/README.md) — documented under `documentation/assets/`.

## Boundaries

- Depends on: the code each document describes, which it is checked against; the
  [README standard](/documentation/standards/readme_standard.md), the
  [documentation standards](/documentation/standards/documentation_standards.md) and the
  [templates](/documentation/standards/templates/README.md) that shape it.
- Used by: the code READMEs, comments and `CLAUDE.md`, which link its documents;
  `cargo xtask ticket sync` (`tools/ticket_engine/`), which updates the Mission Creator roadmap
  between markers and runs its ticket-column writer over the Eden gap analysis, which finds no
  table to rewrite there; ticketboard, which opens each ticket's spec and plan
  from `tickets/`; `cargo run -q -p developer_tools --bin enf -- citations`, which checks every
  `@idx` citation here against the [Enfusion](/documentation/glossary/a_to_f.md#enfusion) symbol index.
- Rules: every folder carries a README.md whose Contents block lists its tracked children
  (`cargo xtask verify readme-coverage`); a live document stays within 500 lines
  (`cargo xtask verify markdown-placement`); every link, backticked path and cited command resolves
  (`cargo xtask verify link-check`); every document opens with its status line; frozen records
  and archived documents are never reworded; file names are snake_case, apart from README.md,
  `t-<id>_plan.md` and the hyphenated evidence JSON.

## Related documentation

- [Documentation standards](/documentation/standards/documentation_standards.md) — comment
  rules, the tree's layout, names and lifecycle, and the gates.
- [README standard](/documentation/standards/readme_standard.md) — how every README is built.
- [Glossary](/documentation/glossary/README.md) — the terms the documents use.
- [Product roadmap](/documentation/product_roadmap.md) — what is planned and not yet built.
