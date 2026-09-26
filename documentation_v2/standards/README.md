**Status:** live

# Standards

The rules every change in the repository follows: how code is written and commented, how READMEs
and documents are built, where a new file goes, what a commit carries, how a
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) id is spelled and which crate may name which across
the website's engines. Developers and AI agents read the matching standard before they write.

## Contents

```text
documentation_v2/standards/
├── coding_standards/           code rules by topic, each with a stable rule id and the gate that holds it
├── commit_checklist.md         what a commit that changes code carries and verifies before it lands
├── documentation_standards.md  comment rules, cross-boundary tags, the documentation tree and lifecycle
├── engine_boundary_rules.md    layer rules between the graphics engine, the map engine and the frontend
├── readme_standard.md          README sections, kinds and the Contents block the gate checks
├── templates/                  copyable skeletons for every README kind and document type
├── ticket_identifiers.md       ticket id grammar, ticket, spec and plan paths, ids in commits and docs
└── where_does_x_go.md          the home of each kind of file, and the gate that enforces it
```

## How it works

Each standard owns one question, and the others link it rather than restate it:

| Question | Standard |
|---|---|
| How is code written and laid out? | [coding standards](/documentation_v2/standards/coding_standards/README.md) |
| How are comments, tags and documents written? | [documentation standards](/documentation_v2/standards/documentation_standards.md) |
| How is a README built? | [README standard](/documentation_v2/standards/readme_standard.md), with the [templates](/documentation_v2/standards/templates/README.md) |
| Which engine crate may name which? | [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) |
| Where does a new file go? | [where does X go?](/documentation_v2/standards/where_does_x_go.md) |
| What does a commit carry? | [commit checklist](/documentation_v2/standards/commit_checklist.md) |
| How is a ticket id formed and cited? | [ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) |

`CLAUDE.md` states the repository-wide laws in brief; a standard holds the detail and names the
`cargo xtask verify` gate or test that enforces each rule it states, and a rule with no gate is a
convention reviewers hold. A new standard is added here only when it answers a question none of
these answers; a rule that fits an existing standard goes into it.

## Code

- [Verification gates](/tools_v2/xtask/src/verifications/) — the `cargo xtask verify` checks the
  standards cite: file length, README coverage, link check, markdown placement, route tags,
  contract citations and the engine layer walls.
- [CI task list](/tools_v2/xtask/src/commands/ci/) — the `ci-local` steps that run those gates.
- [Graphics engine](/apps/website/graphics-engine/), [map engine](/apps/website/map-engine/) and
  [frontend](/apps/website/frontend/) — the crates the engine boundary rules govern.
- [Ticket engine](/tools_v2/ticket-engine/) — the id, spec and plan paths the ticket identifiers
  standard describes.

## Boundaries

- Depends on: `CLAUDE.md`, whose laws the standards expand; the gate code under
  `tools_v2/xtask/src/verifications/`, which is the final word where a standard and a gate differ;
  the [glossary](/documentation_v2/glossary/README.md) for terms.
- Used by: every README and document under `documentation_v2/` and the code trees, which follow
  the README standard and the templates; gate code and CI that cite a standard by section
  (`tools_v2/xtask/src/commands/ci/task_definitions.rs`,
  `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs`,
  `.github/workflows/ci.yml`, `.github/workflows/contracts.yml`, `.editorconfig`); the Cursor rule
  `.cursor/rules/tbd-platform.mdc`; the runbooks and the entry README.
- Rules: each standard names the gate that holds each enforced rule; a standard stays at or under
  500 lines and splits into a folder with a README index when longer, as `coding_standards/` does
  (`cargo xtask verify markdown-placement`); a section number a gate or CI file cites keeps its
  number, or the citing code changes in the same commit.

## Related documentation

- [Documentation entry](/documentation_v2/README.md) — the map of the documentation tree.
- [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) — running the gates the
  standards name before a push.
