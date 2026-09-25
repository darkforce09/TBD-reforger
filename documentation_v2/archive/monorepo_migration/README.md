**Status:** live

# Monorepo merge records

The records of how the website and game [mod](/documentation_v2/glossary.md#mod) repositories became
this monorepo and how their documentation was first reorganised in it: the merge runbook, the
changelog of moved document paths with before and after manifests, and the index READMEs of
documentation folders the repository does not hold. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/monorepo_migration/
├── doc_manifest_after.txt             size listing of every document after the reorganisation
├── doc_manifest_before.txt            size listing of every document before the reorganisation
├── docs_platform_readme.md            index of the cross-cutting platform documentation folder
├── docs_specs_readme.md               index of the design specs and Mission Creator engineering folder
├── docs_website_archive_readme.md     index of the website's design-phase archive
├── macos_blueprints_readme.md         index of the macOS-style HTML blueprint mockups
├── mission_creator_mock_up_readme.md  index of the Mission Creator mock-up explorations
├── monorepo_migration_runbook.md      runbook that merged the two repositories, commit history kept
└── reorg_changelog.md                 old to new path of each document the reorganisation moved
```

## How it works

The runbook records how the two source repositories were merged with their commit identities intact;
the changelog and the two manifests record the reorganisation of their documents that followed. The
five index READMEs are the entry pages of documentation folders the repository does not hold; the
documents they indexed sit in the other archive topics or were carried into live documents.

## Code

None: the records concern the repository's layout, not a code folder.

## Boundaries

- Depends on: nothing live; the records quote the repositories of their time.
- Used by: the repository's root README and one [ticket](/documentation_v2/glossary.md#ticket) file
  in `.ai/tickets/`, which link the merge runbook.
- Rules: never reworded, only links change.

## Related documentation

- [Documentation entry](/documentation_v2/README.md) — the documentation tree as it is.
- [Go and React era designs](/documentation_v2/archive/go_and_react_era_design/README.md) and
  [redirect stubs](/documentation_v2/archive/redirect_stubs/README.md) — the archived documents
  those indexes point at.
