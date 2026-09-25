**Status:** live

# Go and React era designs

The platform's first design documents: the product blueprint and context handoff, a backend plan for
a Go server, the frontend documentation hub with its roadmap and page specs, and the first [Mission
Creator](/documentation_v2/glossary.md#mission-creator) problem statement, design, engineering plan,
roadmap and UX specs. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/go_and_react_era_design/
├── event_registration_flow_redesign.md    redesign of event registration for multi-mission events
├── frontend_documentation_index.md        master index of the frontend surface specs
├── frontend_documentation_readme.md       entry page of the frontend documentation hub
├── frontend_page_spec_template.md         template every frontend surface spec followed
├── frontend_roadmap.md                    shipped and deferred frontend routes, with their specs
├── frontend_work_tracking.md              pointer from the frontend hub to the ticket registry
├── go_backend_architecture_plan.md        backend plan for a Go server: schema, routes, services
├── macos_ux_architecture.md               macOS-style UX methodology: context, disclosure, action
├── mission_creator_design.md              Mission Creator and planner design blueprint
├── mission_creator_engineering_plan.md    Mission Creator engineering plan: renderer and stack choices
├── mission_creator_problem_statement.md   Mission Creator as a visual compiler and 2D GIS: the problem
├── mission_creator_react_era_roadmap.md   Mission Creator roadmap of the React frontend
├── mission_creator_react_era_ux_spec.md   Mission Creator UX spec modelled on the Eden editor
├── mission_creator_setup_wizard_page.md   page spec of the mission setup wizard route
├── mission_editor_react_era_page_spec.md  page spec of the 2D canvas editor
└── platform_context_handoff.md            product blueprint and context handoff of the whole platform
```

## How it works

These documents plan a Go backend and a React frontend, and track the first Leptos pages, in layouts
the repository does not contain: the platform is an Axum API and a Leptos single-page app under
`apps/website/`. Each is read for the intent behind a feature, never for how it works now. Where a
live document took over a file's subject, the file's status line links it:

| Archived files | Live replacement |
|---|---|
| the frontend documentation index, readme, roadmap and work tracking | [frontend documentation](/documentation_v2/website/frontend/README.md) |
| the frontend page spec template | [feature doc template](/documentation_v2/standards/templates/feature_doc.md) |
| the macOS UX methodology | [design system](/documentation_v2/design_system/README.md) |
| the Mission Creator roadmap | [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) |
| the Mission Creator UX spec and the editor page spec | [Mission Creator UX spec](/documentation_v2/website/frontend/apps/editor/ux_spec.md) |
| the setup wizard page | [mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md) |

The [event](/documentation_v2/glossary.md#event) registration redesign, the Go backend plan, the
Mission Creator design, engineering plan and problem statement, and the platform context handoff
have no single replacement: the [API documentation](/documentation_v2/website/api_v2/README.md) and
the [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md)
describe what was built.

## Code

- [Frontend](/apps/website/frontend/) and [API](/apps/website/api_v2/) — the Rust code that
  implements the platform these documents planned.
- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor the Mission Creator
  documents planned.

## Boundaries

- Depends on: nothing live; the documents quote a codebase the repository does not hold.
- Used by: the page feature docs and folder READMEs under `documentation_v2/website/frontend/` that
  link the design a page came from; the [ticket](/documentation_v2/glossary.md#ticket) files in
  `.ai/tickets/` whose citations name these documents, most often the platform context handoff; a
  comment in `apps/website/frontend/src/v2/apps/editor/ui/inspector/env.rs` that cites the
  engineering plan; the stubs in the [redirect
  stubs](/documentation_v2/archive/redirect_stubs/README.md) topic.
- Rules: never reworded, only links change; a fact still true is carried to a live document, not
  corrected here.

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md) — every route and its
  feature doc.
- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  live editor documents.
- [API documentation](/documentation_v2/website/api_v2/README.md) — the backend as built.
