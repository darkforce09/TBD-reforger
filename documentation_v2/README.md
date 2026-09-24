**Status:** live

# Documentation

The documentation of the TBD Reforger platform: feature docs, runbooks, standards, design
references, known bugs, ticket specs and plans, and the archive, laid out as a mirror of the code.
Start here to find the document that covers a subject and to learn which source wins when two
disagree.

## Contents

```text
documentation_v2/
├── archive/                         frozen history, one folder per topic
├── contracts_v2/                    documents on the contracts in contracts_v2/
├── design_system/                   design tokens, typography, colour and symbology
├── glossary.md                      the project's terms and abbreviations
├── known_bugs/                      the live registry of known bugs
├── mod/                             documents on the Enfusion mod suite in apps/mod/
├── pending_merge/                   sources a writer is merging into live documents
├── refactor_followup_tickets.md     program record: tickets the documentation program filed
├── refactor_move_manifest/          program record: the manifest's summary and checkpoint answers
├── refactor_move_manifest.tsv       program record: target, action and writer of each moved file
├── refactor_orphan_spec_links.tsv   program record: the ticket each orphan spec links to
├── refactor_pin_catalogue.md        program record: code and tool pins on documentation paths
├── refactor_program_plan.md         program record: the documentation program's plan
├── refactor_progress_checkpoint.md  program record: the program's resume file and roster
├── refactor_ticket_rewrites.tsv     program record: ticket fields rewritten to the new paths
├── refactor_writing_brief.md        program record: the brief every program writer reads
├── runbooks/                        operator procedures: development, deployment, gates, playtests
├── standards/                       documentation and code standards, and the templates
├── tickets/                         ticket specs and plans, flat, frozen once the ticket closes
├── tools_v2/                        documents on the developer tools in tools_v2/
└── website/                         documents on the website in apps/website/
```

## How it works

Two layers document the code. The README.md in each code folder says what the folder holds, how
it fits together and where it stops; the documents here go deeper, and each code README links
them. A document about code sits at the code's path with `apps/`, `src/`, `src/v2/` and
`Scripts/Game/TBD/` left out: the event schedule page in
`apps/website/frontend/src/v2/pages/operations/schedule/` is documented in
`website/frontend/pages/operations/schedule/`, and all Mission Creator material sits in
`website/frontend/apps/editor/`. What spans the code has a top-level folder of its own:
`runbooks/`, `standards/`, `design_system/`, `known_bugs/`, `tickets/` and `archive/`, with
`glossary.md` beside them. The
[documentation standards](/documentation_v2/standards/documentation_standards.md) set the layout,
names and lifecycle; the [README standard](/documentation_v2/standards/readme_standard.md) shapes
every README.

Every document opens with its status line. A live document tracks the code and changes in the same
commit as the code it describes. A frozen record (the spec or plan of a closed ticket) and an
archived document keep their words; only their links change. The `refactor_*` files and
`pending_merge/` belong to the documentation program that is building this tree: the records hold
its plan, brief, manifest and progress, and `pending_merge/` holds the secondary sources each
writer merges into a live document and then deletes.

### Authority ladder

When two sources disagree, the higher one wins and the lower one is corrected:

1. The running code.
2. [CLAUDE.md](/CLAUDE.md): the project laws, the directory atlas and the canonical commands.
3. This README: the map of the documentation.
4. The [documentation standards](/documentation_v2/standards/documentation_standards.md), the
   [README standard](/documentation_v2/standards/readme_standard.md), the
   [templates](/documentation_v2/standards/templates/README.md) and the other standards.
5. Feature docs, runbooks and the other live documents.
6. Frozen specs and plans under `tickets/`.
7. The archive, which records history and is never current.

### Where things live

| To find | Look in |
|---|---|
| what a code folder holds and how to use it | the README.md in that folder |
| a web page's behaviour, design, open work and decisions | `website/frontend/pages/<area>/<page>/`, indexed by the [frontend README](/documentation_v2/website/frontend/README.md) |
| the Mission Creator: features, roadmap, UX decisions, Eden reference | [website/frontend/apps/editor/](/documentation_v2/website/frontend/apps/editor/README.md) |
| the API's areas and its verification evidence | [website/api_v2/](/documentation_v2/website/api_v2/README.md), starting at the [API overview](/documentation_v2/website/api_v2/api_overview.md) |
| the mod's design, screens and export evidence | [mod/](/documentation_v2/mod/README.md) |
| how to run, test, deploy or play-test anything | `runbooks/`, starting at [local development](/documentation_v2/runbooks/local_development.md) |
| the rules for code, comments, documents and commits | `standards/` |
| a term or abbreviation | the [glossary](/documentation_v2/glossary.md) |
| design tokens, colour and symbology | `design_system/` |
| a known bug and its workaround | [known_bugs/](/documentation_v2/known_bugs/README.md) |
| what to work on next | the ticket registry: `cargo xtask ticket next`, `.ai/tickets/queue.json` or ticketboard |
| a ticket's spec or plan | `tickets/specs/` and `tickets/plans/`; the ticket itself is `.ai/tickets/T-<id>.toml` |
| why something was built the way it was, or what came before | `archive/<topic>/` and the commit history |

## Code

- [Website](/apps/website/README.md) — documented under `website/`.
- [Mod suite](/apps/mod/README.md) — documented under `mod/`.
- [Developer tools](/tools_v2/README.md) — documented under `tools_v2/`.
- [Contracts](/contracts_v2/README.md) — documented under `contracts_v2/`.
- [Assets](/assets_v2/README.md), the [fleet host agent](/apps/fleet_host_agent/README.md) and
  [ticketboard](/apps/ticketboard/README.md) — documented in their own READMEs.

## Boundaries

- Depends on: the code each document describes, which it is checked against; the
  [README standard](/documentation_v2/standards/readme_standard.md), the
  [documentation standards](/documentation_v2/standards/documentation_standards.md) and the
  [templates](/documentation_v2/standards/templates/README.md) that shape it.
- Used by: the code READMEs, comments and `CLAUDE.md`, which link its documents;
  `cargo xtask ticket sync` (`tools_v2/ticket-engine/`), which updates the Mission Creator roadmap
  and the Eden gap analysis between markers; ticketboard, which opens each ticket's spec and plan
  from `tickets/`; `cargo run -q -p developer-tools --bin enf -- citations`, which checks every
  `@idx` citation here against the Enfusion symbol index.
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
- [Glossary](/documentation_v2/glossary.md) — the terms the documents use.
