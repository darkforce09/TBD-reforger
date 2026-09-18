# Built-in Terrains (`assets_v2/terrains/`)

The island datasets that ship in the repository, served at `/map-assets` and used by local development, the headless editor gates, and CI.

---

## 1. Directory Topology

```text
assets_v2/terrains/
├── README.md                           <-- Terrain contract (this document)
├── terrain-registry.json               <-- Catalog of every registered terrain
├── everon/                             <-- Primary island, fully exported
└── arland/                             <-- Registered and queued; manifest only
```

---

## 2. The Terrain Registry

`terrain-registry.json` conforms to `contracts_v2/definitions/terrain-registry.schema.json` and is the list every tool walks when it needs "all terrains". Each entry carries the terrain id, display name, world bounds in metres, the path to its manifest, the Workbench world it was exported from, and how far its object import has progressed:

| Terrain | World bounds (m) | Status | Import phases shipped |
|:---|:---|:---|:---|
| `everon` | `0, 0, 12800, 12800` | `active` | P1 buildings → P5 props |
| `arland` | `0, 0, 4096, 4096` | `queued` | none |

`status` is what separates a terrain that must validate from one that is merely declared. Verifiers skip a `queued` terrain's asset checks and still require its registry entry and manifest to parse.

---

## 3. Dataset Contract

A terrain directory is addressed only through its `manifest.json`. Nothing hard-codes a file name inside a terrain: the manifest names the DEM, the satellite bundle, the tile pyramids, the object chunk directory, the road and label archives, and the level-of-detail gates. A dataset is therefore free to ship a subset — Arland today is a manifest and nothing else — and every consumer degrades on the manifest rather than on a missing file.

Coordinates are metric with the origin at the south-west corner. Elevation is authored by the game engine's own surface probe, so the DEM is a resample of `GetSurfaceY` rather than an independent height source, and spawn height remains the engine's to decide.

---

## 4. Storage Seam

This directory is the **development and CI tier**: committed, available offline, and enough to boot the Scenario Creator with no external storage. Terrains uploaded by the community at runtime live on the production volume described in [`../storage_spec/README.md`](../storage_spec/README.md) and are served through the same `/map-assets` mount. Neither tier knows about the other; the API resolves a directory per tier and serves both.
