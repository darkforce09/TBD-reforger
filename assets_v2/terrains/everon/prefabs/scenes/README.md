# Everon building scene specs

Hand-written placements of extra objects around a building, the input from which the blueprint
compiler builds the building's scene file. The one spec here puts four spruces around the wooden
farmhouse for the debug building viewer.

## Contents

```text
assets_v2/terrains/everon/prefabs/scenes/
└── FarmHouse_E_1L01_Wood.scene.spec.json  four trees placed around the wooden farmhouse
```

## Format

- Encoding: UTF-8 JSON named `<slug>.scene.spec.json` after the building it surrounds. The file
  holds a `$comment` and `entries[]`, each with an `id`, the
  [Enfusion](/documentation_v2/glossary.md#enfusion) `prefab` path, `pos` in the building's local
  frame (metres, y up), and optionally `anglesDeg` as `[pitch, yaw, roll]` in degrees (default all
  zero) and a uniform `scale` (default 1).
- Schema: no JSON Schema; the reader is `SceneSpec` in
  `tools_v2/developer-tools/src/blueprint/bvh/batch_processing.rs`, which ignores `$comment`.
- Adding a file: a person writes it, then
  `cargo xtask map bvh-batch --prefab <Prefabs/…/X.et> --scene <spec>` walks each entry's prefab
  out of the game paks, writes the meshes it needs to `prefabs/blas/` and writes
  `prefabs/buildings/<slug>.scene.json`.

## Producers and consumers

- Producers: a person; no tool writes these files.
- Consumers: `cargo xtask map bvh-batch --scene` only, through the path given on its command
  line; no code opens this folder by name. The debug building viewer reads the emitted
  `assets_v2/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.scene.json`, never the spec.

## Boundaries

- Depends on: the Enfusion prefabs each entry names, which the batch must find in the game paks.
- Used by: `cargo xtask map bvh-batch`, when a person passes a spec's path to `--scene`.
- Rules: a spec and its emitted scene file change together, since the viewer reads only the
  emitted one; a spec's `pos` and angles use the same local frame as the building's blueprint.

## Related documentation

- [Building architecture](/apps/website/map-engine/src/world/architecture/README.md) — the
  compound model the emitted scene file is added to.
