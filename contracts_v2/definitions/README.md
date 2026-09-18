# Wire Schema Definitions (`contracts_v2/definitions/`)

The 25 authoritative JSON Schema files. Every cross-boundary payload on the platform is shaped by one of them.

---

## 1. Contents

```text
definitions/
├── README.md
│
│   # Missions
├── mission.schema.json                       <-- Mission contract: slots, sides, zones, flow
├── mission-editor-payload.schema.json        <-- Editor superset posted to /missions/:id/versions
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
