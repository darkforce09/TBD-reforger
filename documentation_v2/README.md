**Status:** live

# Documentation

The documentation of the TBD Reforger platform: feature docs, runbooks, standards, design
references, known bugs, the product roadmap, ticket specs and plans, and the archive, laid out as a
mirror of the code. Start here to find the document that covers a subject and to learn which source
wins when two disagree.

## Contents

```text
documentation_v2/
├── archive/                 frozen history, one folder per topic
├── assets_v2/               documents on the terrain export and the map data in assets_v2/
├── contracts_v2/            documents on the contracts in contracts_v2/
├── design_system/           design tokens, symbology and interaction patterns the website and mod share
├── fleet_host_agent/        documents on the fleet host agent in apps/fleet_host_agent/
├── glossary/                the project's terms and abbreviations, split by first letter
├── known_bugs/              the live registry of known bugs
├── mod/                     documents on the Enfusion mod suite in apps/mod/
├── product_roadmap.md       the planned product items by area and the open product questions
├── refactor_*               program records: plan, brief, style lock, progress and working lists
├── refactor_move_manifest/  program record: the manifest's summary and checkpoint answers
├── runbooks/                operator procedures: development, deployment, gates, playtests
├── standards/               documentation and code standards, and the templates
├── ticketboard/             documents on the ticketboard viewer in apps/ticketboard/
├── tickets/                 ticket specs and plans, flat, frozen once the ticket closes
├── tools_v2/                documents on the developer tools in tools_v2/
└── website/                 documents on the website in apps/website/
```

## How it works

Two layers document the code. The README.md in each code folder says what the folder holds, how
it fits together and where it stops; the documents here go deeper, and each code README links
them. A document about code sits at the code's path with `apps/`, `src/`, `src/v2/` and
`Scripts/Game/TBD/` left out: the [event](/documentation_v2/glossary/a_to_f.md#event) schedule page in
`apps/website/frontend/src/v2/pages/operations/schedule/` is documented in
`website/frontend/pages/operations/schedule/`, the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) in `apps/fleet_host_agent/` in
`fleet_host_agent/`, and all [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
material sits in `website/frontend/apps/editor/`. What spans the code has a top-level folder of its
own: `runbooks/`, `standards/`, `design_system/`, `known_bugs/`, `tickets/` and `archive/`, with
the `glossary/` folder and `product_roadmap.md` beside them. The
[documentation standards](/documentation_v2/standards/documentation_standards.md) set the layout,
names and lifecycle; the [README standard](/documentation_v2/standards/readme_standard.md) shapes
every README.

```text
code folder README ──links──▶ documentation_v2/<code path>/   feature docs, evidence, visual references
                                   │ first use of a term ──▶ glossary/
                                   │ open work ────────────▶ .ai/tickets/ ──spec, plan──▶ tickets/
                                   └ history ──────────────▶ archive/<topic>/
```

Every document opens with its status line. A live document tracks the code and changes in the same
commit as the code it describes. A frozen record (the spec or plan of a closed
[ticket](/documentation_v2/glossary/n_to_z.md#ticket)) and an archived document keep their words; only
their links change. The `refactor_*` files and `refactor_move_manifest/` belong to the
documentation program that builds this tree and hold its plan, brief, manifest and progress; while
the program runs, a writer stages the secondary sources it merges in `pending_merge/<writer>/` and
deletes each once merged, so the folder holds no tracked file.

### Authority ladder

When two sources disagree, the higher one wins and the lower one is corrected:

1. The running code.
2. [CLAUDE.md](/CLAUDE.md): the project laws, the directory atlas and the canonical commands.
3. This README: the map of the documentation.
4. The [documentation standards](/documentation_v2/standards/documentation_standards.md), the
   [README standard](/documentation_v2/standards/readme_standard.md), the
   [templates](/documentation_v2/standards/templates/README.md) and the other standards.
5. Feature docs, runbooks, the product roadmap and the other live documents.
6. Frozen specs and plans under `tickets/`.
7. The archive, which records history and is never current.

### Where to find what

| To find | Look in |
|---|---|
| what a code folder holds and how to use it | the README.md in that folder |
| a web page's behaviour, design, open work and decisions | `website/frontend/pages/<area>/<page>/`, indexed by the [frontend README](/documentation_v2/website/frontend/README.md) |
| the Mission Creator: features, roadmap, UX decisions, Eden reference | [website/frontend/apps/editor/](/documentation_v2/website/frontend/apps/editor/README.md) |
| the [API](/documentation_v2/glossary/a_to_f.md#api)'s areas and its verification evidence | [website/api_v2/](/documentation_v2/website/api_v2/README.md), starting at the [API overview](/documentation_v2/website/api_v2/api_overview.md) |
| the map engine and the graphics engine | [website/](/documentation_v2/website/README.md) |
| the [mod](/documentation_v2/glossary/g_to_m.md#mod)'s design, screens and export evidence | [mod/](/documentation_v2/mod/README.md) |
| how a terrain becomes the map data the platform serves | [assets_v2/](/documentation_v2/assets_v2/README.md) |
| how a game host carries out server commands | [fleet_host_agent/](/documentation_v2/fleet_host_agent/README.md) |
| the ticket viewer | [ticketboard/](/documentation_v2/ticketboard/README.md) |
| the developer tools and the contracts | [tools_v2/](/documentation_v2/tools_v2/README.md) and [contracts_v2/](/documentation_v2/contracts_v2/README.md) |
| how to run, test, deploy or play-test anything | [runbooks/](/documentation_v2/runbooks/README.md), starting at [local development](/documentation_v2/runbooks/local_development.md) |
| the rules for code, comments, documents and commits | [standards/](/documentation_v2/standards/README.md) |
| a term or abbreviation | the [glossary](/documentation_v2/glossary/README.md) |
| design tokens, symbology and interaction patterns | [design_system/](/documentation_v2/design_system/README.md) |
| a known bug and its workaround | [known_bugs/](/documentation_v2/known_bugs/README.md) |
| what the product plans to build, and the open product questions | the [product roadmap](/documentation_v2/product_roadmap.md) |
| what to work on next | the ticket registry: `cargo xtask ticket next`, `.ai/tickets/queue.json` or [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) |
| a ticket's spec or plan | [tickets/](/documentation_v2/tickets/README.md); the ticket itself is `.ai/tickets/T-<id>.toml` |
| why something was built the way it was, or what came before | [archive/](/documentation_v2/archive/README.md) and the commit history |

## Code

- [Website](/apps/website/README.md) — documented under `website/`.
- [Mod suite](/apps/mod/README.md) — documented under `mod/`.
- [Developer tools](/tools_v2/README.md) — documented under `tools_v2/`.
- [Contracts](/contracts_v2/README.md) — documented under `contracts_v2/`.
- [Assets](/assets_v2/README.md) — documented under `assets_v2/`.
- [Fleet host agent](/apps/fleet_host_agent/README.md) — documented under `fleet_host_agent/`.
- [Ticketboard](/apps/ticketboard/README.md) — documented under `ticketboard/`.

## Boundaries

- Depends on: the code each document describes, which it is checked against; the
  [README standard](/documentation_v2/standards/readme_standard.md), the
  [documentation standards](/documentation_v2/standards/documentation_standards.md) and the
  [templates](/documentation_v2/standards/templates/README.md) that shape it.
- Used by: the code READMEs, comments and `CLAUDE.md`, which link its documents;
  `cargo xtask ticket sync` (`tools_v2/ticket-engine/`), which updates the Mission Creator roadmap
  between markers and runs its ticket-column writer over the Eden gap analysis, which finds no
  table to rewrite there; ticketboard, which opens each ticket's spec and plan
  from `tickets/`; `cargo run -q -p developer-tools --bin enf -- citations`, which checks every
  `@idx` citation here against the [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) symbol index.
- Rules: every folder carries a README.md whose Contents block lists its tracked children
  (`cargo xtask verify readme-coverage`); a live document stays within 500 lines
  (`cargo xtask verify markdown-placement`); every link, backticked path and cited command resolves
  (`cargo xtask verify link-check`); every document opens with its status line; frozen records
  and archived documents are never reworded; file names are snake_case, apart from README.md,
  `t-<id>_plan.md` and the hyphenated evidence JSON.

## Related documentation

- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — comment
  rules, the tree's layout, names and lifecycle, and the gates.
- [README standard](/documentation_v2/standards/readme_standard.md) — how every README is built.
- [Glossary](/documentation_v2/glossary/README.md) — the terms the documents use.
- [Product roadmap](/documentation_v2/product_roadmap.md) — what is planned and not yet built.
