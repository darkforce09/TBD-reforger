# Arland Dataset (`assets_v2/terrains/arland/`)

The 4.1 km × 4.1 km secondary island. Registered and declared; its export has not run.

---

## 1. File Layout

```text
assets_v2/terrains/arland/
├── README.md
└── manifest.json                       <-- Bounds, DEM scaling, and tile paths
```

---

## 2. Status

`terrain-registry.json` marks Arland `queued` with no import phases shipped. The manifest declares the island's bounds (`0, 0, 4096, 4096`), its 2 m DEM scale and its elevation band (−163 m to 148.38 m), and names the DEM and tile paths the export will write. Those files do not exist yet, and the DEM's pixel dimensions are still zero.

That combination is deliberate and is what `queued` means: the terrain resolves through the registry and its manifest parses, so tooling can enumerate it and report it as unexported, while asset verifiers skip it rather than failing on absent files. Shipping the dataset is the export pipeline's job — running `cargo xtask map export-terrain arland` and committing what it produces — after which the registry entry flips to `active`.
