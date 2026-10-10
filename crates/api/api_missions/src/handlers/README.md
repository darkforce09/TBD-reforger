# Missions handlers

The HTTP handlers of the [missions](/documentation/glossary/g_to_m.md#missions) domain, one module per
surface: the [mission](/documentation/glossary/g_to_m.md#mission) library and its lifecycle, the
versions the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) saves, the
[armory](/documentation/glossary/a_to_f.md#armory), submission, reviews and
[approvals](/documentation/glossary/a_to_f.md#approvals), the faction library, the item
[registry](/documentation/glossary/n_to_z.md#registry),
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) and the game-runtime routes.

## Contents

```text
crates/api/api_missions/src/handlers/
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
writes, `AdminUser` for the queue, the [deployments](/documentation/glossary/a_to_f.md#deployment) and
the default-override aggregate, and `MachineCaller` with the `mod_runtime` executor kind for the
game-runtime routes. A mission row is visible to everyone once it is `live` and before that to its
author and administrators (`validation::access`); a read of a hidden mission answers 404 like a
missing one, and a write by someone who may not edit the mission answers 403.

- `mission_lifecycle.rs` creates a `draft` with version `0.1.0`, and a patch may only archive or
  unarchive; archiving a mission attached to an upcoming
  [event](/documentation/glossary/a_to_f.md#event), or deleting one attached to any event, answers 409.
  The patch, the delete, submission and review comments take `services::mission_write_lock`, which
  rechecks the [role](/documentation/glossary/n_to_z.md#role) and ownership after the lock wait.
- `mission_versions.rs` holds `validate_payload`, the one gate of every stored version payload (the
  editor schema, cargo capacity against the current catalog, the compiler's type scan), and a save
  also refuses a malformed semver, a vacuous payload and a duplicate version (409).
- `mission_submission.rs` alone writes `pending_approval`: it compiles the current version into an
  [artifact](/documentation/glossary/a_to_f.md#artifact) under review, and `approvals_queue.rs` decides
  exactly that artifact, both through `services::mission_reviews`.
- `mission_armory.rs` validates every row before its transaction deletes the armory, so a
  malformed body (a blank name or faction, a negative `quantity`) never clears it.
- `mission_library.rs` applies the same visibility to the overview, both bookmark writes and the
  `bookmarked` scope: a bookmark write on a mission the caller cannot see answers 404 like a missing
  one, and the `bookmarked` scope lists only the bookmarked missions the caller can see now.
- `artifact_document_response.rs` serves the stored bytes unchanged, with their SHA-256 as the
  strong `ETag` and the compile's findings in the `x-compile-diagnostics-count` and
  `x-compile-diagnostics-rules` headers, since `mission.schema.json` admits no extra key.

## Boundaries

- Depends on: the domain's `models`, `services`, `validation` and `contract`; `api_http_layer`
  for the extractors, `api_foundation` for errors, pagination and wire formats; the API crates for the audit rows
  (`api_audit_log`), the reviewer recheck and `MachineCaller` (`api_caller_identity`);
  `api_community_content::models` for a registry's modpack; `mission_compiler` for
  the save-time type scan; `contracts/definitions/mission.schema.json`, embedded.
- Used by: the domain's `routes.rs`; over HTTP, the Mission Creator in
  `crates/frontend/workspaces/mission_creator_workspace/src/`, the mission hub pages in
  `crates/frontend/pages/mission_hub_pages/src/`, the approvals and
  [server control](/documentation/glossary/n_to_z.md#server-control) pages in
  `crates/frontend/pages/administration_pages/src/`, the
  [game runtime](/documentation/glossary/g_to_m.md#game-runtime) in
  `mod/tbd-framework/Scripts/Game/TBD/`, and the `cargo xtask mod` commands' client in
  `tools/commands/mod_operations/src/website_api_client/`.
- Rules: every handler carries its `/// @route` tag; no handler
  imports another domain's handlers; every
  mission write takes `MissionMakerUser` as well as ownership, so a demotion revokes it.
- Body decoding: every JSON body is read through `ApiError::from_json_rejection`: 413 with
  `details.code = request_too_large` over the body limit, 415 without a JSON content type, and 400
  with the decoder's message (which names the failing field) otherwise.
- Path decoding: every path segment is read through `api_foundation::http::path_parameters::PathParams`: a
  segment that does not decode into its type answers 400 in the `{error}` envelope with a message
  naming the parameter, never axum's plain-text rejection.

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation/crates/api/api_server/design_notes/mission_artifacts.md)
  — submission, review decisions, artifact reads and deployments.
- [Mission approvals page](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md)
  — the review queue as administrators use it.
