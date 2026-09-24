# Missions handlers

The HTTP handlers of the [missions](/documentation_v2/glossary.md#missions) domain, one module per
surface: the [mission](/documentation_v2/glossary.md#mission) library and its lifecycle, the
versions the [Mission Creator](/documentation_v2/glossary.md#mission-creator) saves, the
[armory](/documentation_v2/glossary.md#armory), submission, reviews and
[approvals](/documentation_v2/glossary.md#approvals), the faction library, the item
[registry](/documentation_v2/glossary.md#registry),
[mission deployments](/documentation_v2/glossary.md#mission-deployment) and the game-runtime routes.

## Contents

```text
apps/website/api_v2/src/missions/handlers/
├── approvals_queue.rs             the approval queue; approve or reject the artifact under review
├── artifact_document_response.rs  an artifact's exact bytes, entity tag and diagnostics headers
├── faction_library.rs             create, read, replace and delete the caller's own reusable factions
├── game_runtime_missions.rs       what a game runtime runs, its artifact bytes, in-game deployments
├── mission_armory.rs              read the armory, and replace it whole
├── mission_default_overrides.rs   how many missions change each schema default of a zone rule
├── mission_deployments.rs         an administrator requests, reads and cancels a server's deployments
├── mission_export.rs              the export document an author downloads as `mission.json`
├── mission_library.rs             the library list with scopes and filters, one mission, bookmarks
├── mission_lifecycle.rs           create a draft with its first version, patch metadata, soft delete
├── mission_reviews.rs             the review history, a comment, an artifact and its review workspace
├── mission_submission.rs          submit: compile the current version into an artifact under review
├── mission_versions.rs            save a version, read one back, move the current version
├── mod.rs                         the module tree
├── registry_compat_graph.rs       one modpack's item compatibility edges, raw or as cargo defaults
├── registry_items.rs              the flat item catalog of one modpack, with the helpers both share
└── tests/                         unit tests for queue order, headers, tiers, saves and paging
```

## How it works

The extractor a handler takes sets its tier: `AuthUser` for reads, `MissionMakerUser` for authored
writes, `AdminUser` for the queue, the [deployments](/documentation_v2/glossary.md#deployment) and
the default-override aggregate, and `MachineCaller` with the `mod_runtime` executor kind for the
game-runtime routes. A mission row is visible to everyone once it is `live` and before that to its
author and administrators (`validation::access`); a read of a hidden mission answers 404 like a
missing one, and a write by someone who may not edit the mission answers 403.

- `mission_lifecycle.rs` creates a `draft` with version `0.1.0`, and a patch may only archive or
  unarchive; archiving a mission attached to an upcoming
  [event](/documentation_v2/glossary.md#event), or deleting one attached to any event, answers 409.
  The patch, the delete, submission and review comments take `services::mission_write_lock`, which
  rechecks the [role](/documentation_v2/glossary.md#role) and ownership after the lock wait.
- `mission_versions.rs` holds `validate_payload`, the one gate of every stored version payload (the
  editor schema, cargo capacity against the current catalog, the compiler's type scan), and a save
  also refuses a malformed semver, a vacuous payload and a duplicate version (409).
- `mission_submission.rs` alone writes `pending_approval`: it compiles the current version into an
  [artifact](/documentation_v2/glossary.md#artifact) under review, and `approvals_queue.rs` decides
  exactly that artifact, both through `services::mission_reviews`.
- `mission_armory.rs` validates every row before its transaction deletes the armory, so a
  malformed body never clears it.
- `artifact_document_response.rs` serves the stored bytes unchanged, with their SHA-256 as the
  strong `ETag` and the compile's findings in the `x-compile-diagnostics-count` and
  `x-compile-diagnostics-rules` headers, since `mission.schema.json` admits no extra key.

## Boundaries

- Depends on: the domain's `models`, `services`, `validation` and `contract`; `core` for the
  extractors, errors, pagination and wire formats; `administration` for the audit rows;
  `identity_and_access` for the reviewer recheck; `server_infrastructure` for `MachineCaller`;
  `community_content::models` for a registry's modpack; `website_map_engine::data::scenario` for
  the save-time type scan; `contracts_v2/definitions/mission.schema.json`, embedded.
- Used by: the domain's `routes.rs`; over HTTP, the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/`, the mission hub pages in
  `apps/website/frontend/src/v2/pages/mission_hub/`, the approvals and
  [server control](/documentation_v2/glossary.md#server-control) pages in
  `apps/website/frontend/src/v2/pages/administration/`, the
  [game runtime](/documentation_v2/glossary.md#game-runtime) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/`, and the `cargo xtask mod` commands' client in
  `tools_v2/xtask/src/commands/mod_ops/website_api_client/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); every
  mission write takes `MissionMakerUser` as well as ownership, so a demotion revokes it
  (`mission_mutators_require_mission_maker_tier` in `tests/mission_lifecycle.rs`).

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — submission, review decisions, artifact reads and deployments.
- [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the review queue as administrators use it.
