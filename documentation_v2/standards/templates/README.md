**Status:** live

# Documentation templates

Copyable skeletons for README.md files, one per README kind of the
[README standard](/documentation_v2/standards/readme_standard.md), each with a worked sample written
from a real folder of that kind.

## Contents

```text
documentation_v2/standards/templates/
├── readme_app.md         README template for an app workspace; sample: the Mission Creator
├── readme_area_root.md   README template for an area root; sample: the website area, also two kinds
├── readme_crate_root.md  README template for a crate, package or addon root; sample: the website API
├── readme_domain.md      README template for a domain or subsystem; sample: the API's missions domain
├── readme_leaf.md        README template for a leaf folder; sample: line of sight inside buildings
└── readme_page.md        README template for a page; sample: the event schedule page
```

## How it works

Each template opens with a When to use line naming the folders of its kind, then gives the skeleton:
the README's sections in order inside a fenced `markdown` block, every placeholder written as `<…>`
and saying what goes there. A worked sample follows in a second fenced block, written from a real
folder and checked against its code: its Contents block lists exactly that folder's tracked
children.

The skeleton and the sample sit in fences, so no gate reads them as READMEs: `readme-coverage`
judges only files named README.md, and `link-check` reads neither links nor backticked paths inside
a fence, though it does check every `cargo xtask` command a fence cites. A writer picks the folder's
kind in the standard, copies the skeleton, fills every placeholder from the code, and runs the
gates the standard names.

## Code

- [Documentation gates](/tools_v2/xtask/src/verifications/documentation/README.md) — the
  `readme-coverage`, `markdown-placement` and `link-check` gates that check every README these
  templates shape.

## Boundaries

- Depends on: the README standard, which defines the core, the kinds and every section a template
  spells out.
- Used by: everyone who writes or reviews a README in the code trees or under `documentation_v2/`.
- Rules: one template per README kind, named as the standard's kind table names it; each holds a
  When to use line, a skeleton and one worked sample, both fenced; a sample's Contents block matches
  its folder's tracked children.

## Related documentation

- [README standard](/documentation_v2/standards/readme_standard.md) — the rules, the kinds and the
  gates.
