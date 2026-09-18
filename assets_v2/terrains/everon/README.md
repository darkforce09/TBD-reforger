# Everon Dataset (`assets_v2/terrains/everon/`)

The fully exported 12.8 km × 12.8 km primary island: elevation, satellite imagery, 1.2 million world objects, roads, labels, and building geometry.

---

## 1. File Layout

```text
assets_v2/terrains/everon/
├── README.md
├── manifest.json                       <-- Bounds, DEM scaling, asset paths, LOD gates
├── locations.json                      <-- Settlements, hills, and landmarks
├── road-names.json                     <-- Named road segments
├── height-labels.json                  <-- Spot elevations and contour labels
├── anchors/
│   ├── verification.json               <-- Engine-probed surface heights, the alignment gate's oracle
│   ├── verification.example.json       <-- Shape sample for the schema
│   └── surface-y-log.txt               <-- Raw probe log the verification was reduced from
├── dem/
│   └── everon-dem-16bit.png            <-- 6400² 16-bit elevation grid, 2 m per pixel
├── locations/
│   └── map_labels.rkyv                 <-- Placed label archive (Tier 2)
├── objects/
│   ├── chunks/{cx}_{cy}.bin            <-- 315 populated 512 m chunks, ObjectInstancePod arrays
│   ├── chunks/{cx}_{cy}.json.gz        <-- JSON parity twin of each chunk
│   ├── density/*.bin                   <-- 625 TBDD forest-density tiles, 8 m cells
│   ├── prefabs.rkyv / prefabs.json.gz  <-- Prefab catalog, 1,623 entries
│   ├── forest-regions.rkyv / .json.gz  <-- Derived forest outlines
│   ├── type-inventory.rkyv / .json     <-- Corpus-wide prefab type counts
│   └── roads.json.gz                   <-- Road network in JSON form
├── prefabs/
│   ├── blas/*.bvh                      <-- 1,690 bottom-level acceleration structures
│   ├── blas-manifest.json              <-- BLAS geometry index
│   ├── building_blueprints.rkyv        <-- Voxelised building interiors (Tier 2)
│   ├── buildings/                      <-- Authored blueprint and instance JSON
│   ├── descriptors/                    <-- 1,623 per-prefab property and bounds records
│   └── scenes/                         <-- Scene specs for the building viewer
├── roads/
│   └── road_network.rkyv               <-- Road splines and intersection nodes (Tier 2)
├── satellite/
│   └── everon-sat.tbd-sat              <-- Unified 12,800² satellite pyramid, 14 mips, ~153 MB
└── tiles/                              <-- WebP tile pyramids, generated locally (gitignored)
```

---

## 2. Geographic Parameters

| Parameter | Value |
|:---|:---|
| World extent | 12,800 m × 12,800 m |
| DEM resolution | 2.0 m per pixel, 6400 × 6400 samples |
| Elevation range | −204.78 m to 375.53 m, `uint16` linear |
| Origin | South-west corner at `0, 0` |
| Object chunk grid | 512 m cells; 315 of the 625 cells carry objects |
| Forest density cell | 8 m |
| Objects | 1,216,066 instances across 1,623 prefab types |

---

## 3. Surface Alignment Anchors

`anchors/verification.json` holds the engine-probed surface height at a set of named coordinates, each recorded with the Workbench build that produced it. The alignment gate resamples the DEM at those coordinates and fails when any anchor drifts past the manifest's threshold of 1.0 m. This is what keeps a re-export from silently shifting the world under the placed objects, since both the DEM and the anchors derive from the same engine probe.

---

## 4. Consumers

- **`website-map-engine`** streams chunks against a residency budget, raymarches the DEM for line of sight, and builds spatial indexes from the BLAS archives.
- **`website-graphics-engine`** receives satellite mips and instance batches as GPU uploads.
- **`developer-tools`** writes every file here from a Workbench export and verifies them afterwards.
