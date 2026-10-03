**Status:** live

# Field tools pages

The documentation of the standalone tactical aids, one folder per page. The section holds one
page, the mortar calculator, whose folder holds its feature doc and its design reference.
Developers and AI agents read it before changing a field tool.

## Contents

```text
documentation/apps/frontend/pages/field_tools/
└── mortar/  the mortar calculator page: on-device firing solutions and fire missions saved to an event
```

## How it works

The folder mirrors the page folders under `apps/frontend/src/pages/field_tools/` and
keeps their spelling. A page's folder holds a README index, its feature doc, which follows the
[feature doc template](/documentation/standards/templates/feature_doc.md), and a
`visual_references/` folder of design-phase sets.

A field tool stands on its own: it hangs off no [mission](/documentation/glossary/g_to_m.md#mission),
owns a route under `/tools/` and sits in the sidebar's "Field Tools" section. The mortar calculator
is public and works offline; only its save area renders inside `AuthGate`. The debug benches are apps, documented under
[the debug benches documentation](/documentation/apps/frontend/workspaces/debug/README.md).

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Mortar calculator | `/tools/mortar`, `MortarCalculatorPage` | Mortar Calculator | [mortar_calculator_page.md](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md) |

A new field tool gets a folder here named like its code folder, with a README and its feature doc,
a line in Contents and a row in the table.

## Code

- [Field tools pages](/apps/frontend/src/pages/field_tools/) — the route components the
  feature docs describe.
- [Operations domain](/crates/api/api_operations/src/) — the ballistics catalog and
  fire-mission routes behind the mortar calculator.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the page code, the API handlers it calls and the
  ticket registry in `.ai/tickets/`, which the feature docs are written from.
- Used by: the in-code READMEs of `field_tools/` and its page folders, which link the feature docs
  under Related documentation.
- Rules: one folder per page folder of the code, spelled the same; a feature doc keeps its name,
  since the READMEs link it; a feature doc stays within 500 lines; design references live only in
  `visual_references/`, and no document holds a screenshot of the built UI.
