**Status:** live

# Documentation templates

Copyable skeletons for every README kind of the
[README standard](/documentation_v2/standards/readme_standard.md) and for the documents under
`documentation_v2/`: feature docs, runbooks, decisions entries, known bugs and glossary entries.
Each template carries a worked sample written from real code or a real document.

## Contents

```text
documentation_v2/standards/templates/
├── decisions_entry.md              template for a decisions.md entry; sample: the terrain height map
├── feature_doc.md                  template for a feature doc; sample: the event schedule page
├── glossary_entry.md               template for a glossary entry; sample: mission header
├── known_bug.md                    template for a known bug; sample: the editor gate boot wedge
├── readme_app.md                   README template for an app workspace; sample: the Mission Creator
├── readme_area_root.md             README template for an area root; sample: the website area
├── readme_command_line.md          README template for a command-line folder; sample: developer tools
├── readme_crate_root.md            README template for a crate, package or addon root; sample: the API
├── readme_data.md                  README template for a data folder; sample: the mission fixtures
├── readme_deploy_config.md         README template for deploy or config; sample: the deploy templates
├── readme_documentation_folder.md  README template for a documentation_v2 folder; sample: known bugs
├── readme_domain.md                README template for a domain or subsystem; sample: missions domain
├── readme_leaf.md                  README template for a leaf folder; sample: line of sight in buildings
├── readme_mod_assets.md            README template for mod assets; sample: the framework prefabs
├── readme_mod_scripts.md           README template for mod scripts; sample: the AI group runtime
├── readme_page.md                  README template for a page; sample: the event schedule page
└── runbook.md                      template for a runbook; sample: the local database and API
```

## How it works

Each template opens with a When to use line naming the folders or documents it fits, then gives
the skeleton: the sections in order inside a fenced `markdown` block, every placeholder written as
`<…>` and saying what goes there. A worked sample follows in a second fenced block, written from
real material and checked against the code: a README sample's Contents block lists exactly its
folder's tracked children, and a document sample quotes only commands, routes, files and tickets
that exist.

The README templates are named `readme_<kind>.md` after the kind table in the standard. The
document templates follow the section orders the standard fixes for them: a feature doc runs Where
it lives, Behaviour, Data, Design, Open work, Decisions; a runbook runs Prerequisites, numbered
Steps, Verify, Troubleshooting, Related; a decisions entry runs Context, Decision, Consequences,
Supersedes under a dated heading; a known bug runs Status, Symptom, Cause, Workaround, Fix, Related
tickets under its number; a glossary entry gives the term, its definition, `In code:` and `See:`.

The skeletons and samples sit in fences, so no gate reads them as documents: `readme-coverage`
judges only files named README.md, and `link-check` reads neither links nor backticked paths inside
a fence, though it does check every `cargo xtask` command a fence cites. A writer picks the kind or
the document type, copies the skeleton, fills every placeholder from the code, and runs the gates
the standard names.

## Code

- [Documentation gates](/tools_v2/xtask/src/verifications/documentation/README.md) — the
  `readme-coverage`, `markdown-placement` and `link-check` gates that check every README and
  document these templates shape.

## Boundaries

- Depends on: the README standard, which defines the README core, the kinds, the document section
  orders and every rule a template spells out.
- Used by: everyone who writes or reviews a README in the code trees, or a document under
  `documentation_v2/`.
- Rules: one template per README kind, named as the standard's kind table names it, and one per
  document type; each holds a When to use line, a skeleton and one worked sample, both fenced; a
  README sample's Contents block matches its folder's tracked children; every template stays within
  500 lines.

## Related documentation

- [README standard](/documentation_v2/standards/readme_standard.md) — the rules, the kinds and the
  gates.
