# Documentation reorganization changelog (T-043)

Old → new paths. Stubs remain at old locations where noted.

## Platform docs

| Old | New |
|-----|-----|
| `Claude_Context_Handoff.md` | `docs/platform/context_handoff.md` (stub at root) |
| `REGISTRATION_FLOW_DESIGN.md` | `docs/platform/registration_flow.md` (stub at root) |
| `Design_Docs/macOS_UX_Architecture.md` | `docs/platform/macos_ux_architecture.md` (stub) |
| `BACKEND_ARCHITECTURE.md` | `docs/backend/architecture.md` (stub at root) |

## New hubs

| Path | Role |
|------|------|
| `docs/README.md` | Platform documentation hub |
| `docs/frontend/README.md` | Frontend master index |
| `docs/backend/README.md` | Backend master index |
| `docs/archive/README.md` | Archive master index |
| `docs/TAGS.md` | T-0xx vs FD-0xx glossary |
| `README.md` | Repo root quick start |

## Mission Creator Architecture

| Old | New |
|-----|-----|
| `00_feature_entry_schema.md` | `reference/feds_schema.md` |
| `01_technical_specification.md` | `01_problem_statement.md` |
| `04_eden_editor_ux_spec.md` | `04_eden_ux_spec.md` |
| `06_tbd_mission_creator_feature_inventory.md` | `06_tbd_feature_inventory.md` |
| `07_arma3_eden_editor_feature_reference.md` | `eden/interactions.md` |
| `07a_eden_ui_anatomy.md` | `eden/ui_anatomy.md` |
| `07b_eden_attribute_catalog.md` | `eden/attributes.md` |
| `08_eden_gap_analysis.md` | `eden/gap_analysis.md` |
| `09_eden_wiki_manifest.yaml` | `eden/wiki_manifest.yaml` |
| `10_mission_creator_roadmap.md` | `02_roadmap.md` |
| *(new)* | `README.md` |

## Mock-ups

| Old | New |
|-----|-----|
| `Mission_Creator_Mock_Up/Mission ccreator/` | `Mission_Creator_Mock_Up/aegis_tokens/` (stub at old path) |

## Frontend surface docs (new)

| File | Route |
|------|-------|
| `frontend/docs/pages/mission-editor.md` | `/missions/:id/edit` |
| `frontend/docs/pages/vehicle-database.md` | `/vehicles` |
| `frontend/docs/pages/event-hub.md` | `/events/:id` |

## Tag rename

Frontend deferred work: `T-001..T-009` in `frontend/docs/TRACKING.md` → `FD-001..FD-009`.
