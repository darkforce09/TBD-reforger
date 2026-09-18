# Missions Subsystem (`missions/`)

Scenario library, 2D/3D CAD version snapshots, maker review queue, virtual arsenal armory catalog, vehicle/weapon prefab registry, compatibility graph, and Enfusion compiler bridge.

---

## 1. Subsystem Topology & Responsibilities

The `missions/` domain decomposes the 2,912 LOC `missions.rs` monolith, externalizes the massive 938 LOC test suite in `mission_compile.rs`, adopts mission staging injection from `field_tools.rs`, and encapsulates prefab catalogs:

```text
src/missions/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/missions & /api/v1/registry sub-router (<100 LOC)
│
├── models/
│   ├── mod.rs
│   ├── mission.rs                      <-- Mission, MissionVersion, MissionArmory, Bookmark
│   ├── registry.rs                     <-- RegistryItem (22 physical fields), RegistryCompatEdge
│   └── faction.rs                      <-- UserFaction authored palette doc
│
├── handlers/
│   ├── mod.rs
│   ├── library.rs                      <-- Catalog listing, search, details, bookmarks (<240 LOC)
│   ├── lifecycle.rs                    <-- Create, patch, soft-delete, submit for review (<320 LOC)
│   ├── upload_ingest.rs                <-- CAD editor JSON versions, SemVer 2.0, rollback (<360 LOC)
│   ├── approvals_queue.rs              <-- Review queue, approve, reject (<260 LOC)
│   ├── armory_catalog.rs               <-- Armory inventory lines & cargo physical capacity (<190 LOC)
│   ├── prefab_registry.rs              <-- Items & compatibility graph with weak ETag (<410 LOC)
│   ├── game_server_injection.rs        <-- JSON export & staging injection (<180 LOC)
│   ├── compiler.rs                     <-- Flat document compile & diagnostics headers (<340 LOC)
│   └── validation.rs                   <-- Common string, enum, and authorization predicates (<180 LOC)
│
├── contract/
│   ├── mod.rs
│   ├── validate.rs                     <-- Runtime JSON-Schema validation & quantisation (<420 LOC)
│   ├── generated/                      <-- Quicktype generated types (from schema/*.json)
│   └── tests/validate.rs               <-- Sibling unit tests (<350 LOC)
│
├── services/
│   ├── mission_compile.rs              <-- Flatten compiler bridge to website-map-engine (<140 LOC)
│   ├── registry_import.rs              <-- Idempotent batch envelope ingest (<400 LOC)
│   └── tests/mission_compile.rs        <-- Sibling unit tests (<950 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── library.rs
    ├── lifecycle.rs
    ├── upload_ingest.rs
    ├── approvals_queue.rs
    ├── armory_catalog.rs
    ├── prefab_registry.rs
    └── compiler.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/missions` | `library::list_missions` | `AuthUser` | Filterable scenario library with scope and terrain filters. |
| `POST` | `/api/v1/missions` | `lifecycle::create_mission` | `MissionMakerUser` | Create draft mission with initial `0.1.0` version. |
| `GET` | `/api/v1/missions/{id}` | `library::get_mission` | `AuthUser` | Scenario overview with author card and armory preview. |
| `PATCH` | `/api/v1/missions/{id}` | `lifecycle::update_mission` | `MissionMakerUser` | Partial metadata update (title, terrain, mode, weather). |
| `DELETE` | `/api/v1/missions/{id}` | `lifecycle::delete_mission` | `MissionMakerUser` | Soft-delete mission (blocked if in active `event_missions`). |
| `POST` | `/api/v1/missions/{id}/submit` | `lifecycle::submit_mission` | `MissionMakerUser` | Transition draft to `pending_approval` for review. |
| `POST` | `/api/v1/missions/{id}/versions` | `upload_ingest::create_version`| `MissionMakerUser` | Save CAD editor payload (custom 256 MB body limit). |
| `GET` | `/api/v1/missions/{id}/versions/{vid}`| `upload_ingest::get_version` | `AuthUser` | Retrieve specific version JSON payload. |
| `POST` | `/api/v1/missions/{id}/versions/{vid}/set-current`| `upload_ingest::set_current_version`| `MissionMakerUser`| Roll back or re-point `current_version_id` to past version. |
| `GET` | `/api/v1/missions/{id}/armory` | `armory_catalog::get_armory` | `AuthUser` | List virtual arsenal lines sorted by `sort_order`. |
| `PUT` | `/api/v1/missions/{id}/armory` | `armory_catalog::set_armory` | `MissionMakerUser` | Atomic wholesale replacement of mission armory. |
| `POST` | `/api/v1/missions/{id}/bookmark` | `library::bookmark_mission` | `AuthUser` | Idempotent scenario bookmark. |
| `DELETE`| `/api/v1/missions/{id}/bookmark` | `library::remove_bookmark` | `AuthUser` | Remove scenario bookmark. |
| `GET` | `/api/v1/missions/{id}/export` | `game_server_injection::export_mission`| `MissionMakerUser`| Download export envelope as JSON attachment. |
| `POST` | `/api/v1/missions/{id}/inject` | `game_server_injection::inject_mission`| `AdminUser` | Compile and stage `mission.json` to server bridge directory. |
| `GET` | `/api/v1/missions/{id}/compiled` | `compiler::get_compiled_mission`| `ServiceAuth` | Serve compiled mod document with diagnostic headers. |
| `GET` | `/api/v1/ingest/missions` | `upload_ingest::ingest_list_missions`| `ServiceAuth` | Service listing of runnable missions with slot counts. |
| `GET` | `/api/v1/admin/mission-default-overrides`| `compiler::mission_default_overrides`| `AdminUser`| Corpus-wide zone rule default override metrics. |
| `GET` | `/api/v1/approvals` | `approvals_queue::list_approvals`| `AdminUser` | Review queue of missions in `pending_approval`. |
| `POST` | `/api/v1/approvals/{id}/approve` | `approvals_queue::approve_mission`| `AdminUser` | Promote mission to `live`. |
| `POST` | `/api/v1/approvals/{id}/reject` | `approvals_queue::reject_mission` | `AdminUser` | Return mission to `rejected` with explanation. |
| `GET` | `/api/v1/registry` | `prefab_registry::list_registry`| `AuthUser` | Modpack item registry with weak ETag (`W/"..."`). |
| `GET` | `/api/v1/registry/compat` | `prefab_registry::list_registry_compat`| `AuthUser` | Directed compatibility graph with weak ETag. |

---

## 3. Key Invariants & Compiler Integration

### 3.1 Zone Quantisation & Save-Time Validation
The compile engine rounds authored zone coordinates to 0.1 m (`round_coord(v)`). To prevent a payload that compiles successfully from failing at `/compiled` serve time, `contract/validate.rs::scan_authored_zones` projects authored shapes to their post-quantisation shape prior to committing the version row.

### 3.2 Strict Separation of Presentation and Engine
The API crate contains zero map geometry rendering or spatial algorithms. It communicates with `website-map-engine` strictly through `website_map_engine::data::scenario::flatten` to compile authored editor documents into Enfusion-compatible JSON.
