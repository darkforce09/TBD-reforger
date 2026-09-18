# Missions Handlers (`missions/handlers/`)

HTTP endpoint handlers for scenario library, authoring lifecycle, version uploads, armory catalogs, approvals review, prefab registry, and staging injection.

---

## 1. Handlers & Route Mappings

### `library.rs` (<240 LOC)
- **`GET /api/v1/missions`** (`list_missions`): Filterable scenario library.
- **`GET /api/v1/missions/{id}`** (`get_mission`): Scenario overview with author card and armory.
- **`POST /api/v1/missions/{id}/bookmark`** (`bookmark_mission`): Bookmark scenario.
- **`DELETE /api/v1/missions/{id}/bookmark`** (`remove_bookmark`): Remove bookmark.

### `lifecycle.rs` (<320 LOC)
- **`POST /api/v1/missions`** (`create_mission`): Create draft scenario with initial version.
- **`PATCH /api/v1/missions/{id}`** (`update_mission`): Metadata update.
- **`DELETE /api/v1/missions/{id}`** (`delete_mission`): Soft-delete scenario.
- **`POST /api/v1/missions/{id}/submit`** (`submit_mission`): Transition draft to `pending_approval`.

### `upload_ingest.rs` (<360 LOC)
- **`POST /api/v1/missions/{id}/versions`** (`create_version`): Save editor payload (256 MB body limit).
- **`GET /api/v1/missions/{id}/versions/{vid}`** (`get_version`): Fetch specific version payload.
- **`POST /api/v1/missions/{id}/versions/{vid}/set-current`** (`set_current_version`): Roll back or re-point version tip.
- **`GET /api/v1/ingest/missions`** (`ingest_list_missions`): Ingest listing of runnable missions with slot counts.

### `armory_catalog.rs` (<190 LOC)
- **`GET /api/v1/missions/{id}/armory`** (`get_armory`): Virtual arsenal inventory lines.
- **`PUT /api/v1/missions/{id}/armory`** (`set_armory`): Wholesale armory replacement.

### `prefab_registry.rs` (<410 LOC)
- **`GET /api/v1/registry`** (`list_registry`): Modpack item registry with weak ETag.
- **`GET /api/v1/registry/compat`** (`list_registry_compat`): Compatibility edges with weak ETag.

### `approvals_queue.rs` (<260 LOC)
- **`GET /api/v1/approvals`** (`list_approvals`): Review queue of pending submissions.
- **`POST /api/v1/approvals/{id}/approve`** (`approve_mission`): Promote mission to `live`.
- **`POST /api/v1/approvals/{id}/reject`** (`reject_mission`): Return mission to `rejected` with explanation.

### `game_server_injection.rs` (<180 LOC)
- **`GET /api/v1/missions/{id}/export`** (`export_mission`): Export envelope JSON download.
- **`POST /api/v1/missions/{id}/inject`** (`inject_mission`): Stage `mission.json` to server bridge directory.

### `compiler.rs` (<340 LOC)
- **`GET /api/v1/missions/{id}/compiled`** (`get_compiled_mission`): Compile scenario with diagnostics.
- **`GET /api/v1/admin/mission-default-overrides`** (`mission_default_overrides`): Zone rule override metrics.

### `validation.rs` (<180 LOC)
- Common validation helpers (`validated_mission_title`, `validated_thumbnail_url`, SemVer parsing).
