# Wire Schema Definitions (`contracts_v2/definitions/`)

The authoritative JSON Schema files. Every cross-boundary payload on the platform is shaped by one of them.

---

## 1. Contents

```text
definitions/
├── README.md
│
│   # Web API
├── current-profile.schema.json               <-- GET /me: the caller's account and membership state
├── reservation-response.schema.json          <-- Registration result: reservation and attendance apart
├── event-hub.schema.json                     <-- GET /events/:id projected for the viewer
├── event-orbat.schema.json                   <-- GET /event-missions/:id/orbat with seat access
├── event-viewer-access.schema.json           <-- Visibility, pool class, pool availability, my_* fields
├── event-access-administration.schema.json   <-- Policies, groups, quotas, evidence and their changes
├── waitlist-promotion-response.schema.json   <-- Leader promotion from the waiting list
├── machine-credential.schema.json            <-- Per-server executor credentials (secret shown once)
├── fleet-command.schema.json                 <-- Command ledger: receipts, claims, executor reports
│
│   # Game runtime (machine credential)
├── game-runtime-session.schema.json          <-- Runtime sessions and generation-fenced heartbeats
├── game-runtime-roster.schema.json           <-- Roster wire version 2 (camelCase)
├── game-runtime-deployment.schema.json       <-- Deployment decisions and ended lives
│
│   # Missions
├── mission.schema.json                       <-- Mission contract: slots, sides, zones, flow
├── mission-editor-payload.schema.json        <-- Editor superset posted to /missions/:id/versions
├── mission-review.schema.json                <-- Artifacts, reviews, decisions, thread, workspace
├── mission-deployment.schema.json            <-- Deployments, runtime reads, fleet scenarios
│
│   # Arsenal and loadouts
├── registry-items.schema.json                <-- Item catalog keyed by Enfusion resource name
├── registry-compat.schema.json               <-- Weapon/attachment compatibility edges
├── registry.schema.json                      <-- Alias-to-GUID spawn layer
├── loadout-export.schema.json                <-- Gear slots: primary, uniform, vest, helmet
├── faction-library.schema.json               <-- Faction roles, gear, and vehicle rosters
│
│   # Terrain
├── terrain-manifest.schema.json              <-- Island bounds, DEM scaling, asset paths
├── terrain-anchors.schema.json               <-- Engine-probed surface height log
├── terrain-registry.schema.json              <-- Registered terrain catalog
│
│   # Map objects
├── map-object-catalog.schema.json            <-- Prefab catalog with classification metadata
├── map-object-enums.schema.json              <-- Closed enums for object kinds and classes
├── map-object-instance.schema.json           <-- One placed instance row
├── map-object-prefab.schema.json             <-- Prefab metadata and dimensions
├── map-object-region.schema.json             <-- Chunk spatial boundary
├── map-object-resolved.schema.json           <-- Resolved reference with transform and tags
├── map-object-roads.schema.json              <-- Road splines and intersection nodes
├── map-object-type-inventory.schema.json     <-- Corpus-wide prefab type counts
│
│   # Geometry
├── blas-manifest.schema.json                 <-- Acceleration-structure geometry index
├── building-blueprint.schema.json            <-- Voxelised building interior
├── building-instances.schema.json            <-- Building instances, floor plates, door links
├── prefab-descriptor.schema.json             <-- Prefab properties and bounding box
│
│   # Cartography
├── locations.schema.json                     <-- Settlements, hills, landmarks
├── height-labels.schema.json                 <-- Spot elevations and contour labels
│
│   # Voice
├── bridge-messages.schema.json               <-- Voice client IPC messages
└── bridge-messages.md                        <-- Handshake and lifecycle narrative
```

---

## 2. Consumers and Generation

| Schema | Consumed by | How the type is produced |
|:---|:---|:---|
| `mission-editor-payload` | API, frontend | Generated: `missions/contract/generated/mission_editor.rs` |
| `registry-items` | API, frontend, developer-tools | Generated: `registry_items.rs` |
| `registry-compat` | API, frontend | Generated: `registry_compat.rs` |
| `faction-library` | API, frontend | Generated: `faction_library.rs` |
| `current-profile`, `reservation-response` | API, frontend | Generated into the owning domain's `models/generated/`; contract tests decode live responses |
| `event-hub`, `event-orbat`, `event-viewer-access`, `event-access-administration`, `waitlist-promotion-response` | API, frontend | Generated: `operations/models/generated/`; `tests/event_access_contract.rs` validates live responses and the frontend goldens |
| `machine-credential`, `fleet-command`, `game-runtime-session` | API, frontend, host agent, game mod | Generated: `server_infrastructure/models/generated/`; `tests/game_runtime_contract.rs` validates live responses |
| `game-runtime-roster`, `game-runtime-deployment` | API, game mod | Generated: `operations/models/generated/`; `tests/game_runtime_contract.rs` validates live responses |
| `mission-review` | API, frontend | Generated: `missions/models/generated/mission_review.rs`; `tests/mission_review_contract.rs` validates live responses and request bodies |
| `mission-deployment` | API, frontend, game mod | Generated: `missions/models/generated/mission_deployment.rs`; `tests/mission_review_contract.rs` validates live responses and request bodies |
| `loadout-export` | API, game mod | Hand-written `loadout_projection.rs`, held by round-trip tests |
| `mission` | API, map engine, game mod | Hand-mapped in the map engine's scenario document model |
| `terrain-manifest` | Map engine, developer-tools | Hand-mapped in the world loader |
| `bridge-messages` | API, game mod, voice client | Read as messages on both sides |
| Map object and geometry schemas | Export pipeline, map engine | Validated at export; read as binary archives at runtime |

Regenerate with `cargo xtask schema codegen`; CI fails on any drift.

---

## 3. Invariants

1. **Closed objects reject unknown keys.** Where a schema sets `additionalProperties: false`, the API returns a validation error rather than dropping the field, so a typo in an authored mission surfaces at submit time and not at mission load.
2. **Published versions are frozen.** A mission version, once published, is never reinterpreted under a newer schema.
3. **Every schema is named for its domain.** A reader who has never seen this repository can tell what `terrain-anchors.schema.json` governs.
