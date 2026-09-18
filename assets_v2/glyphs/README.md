# World Glyphs (`assets_v2/glyphs/`)

The icon set the map draws world objects with: a packed raster atlas, its rectangle index, and the SVG sources both are generated from.

---

## 1. Directory Topology

```text
assets_v2/glyphs/
├── README.md                           <-- Glyph specification (this document)
├── manifest.json                       <-- Glyph registry: source, size, anchor, tint, default colour
├── atlas/
│   ├── world-glyphs.webp               <-- Packed raster atlas
│   └── world-glyphs.json               <-- Per-glyph rectangles within the atlas
└── svg/                                <-- 29 authored SVG sources, one per glyph
```

---

## 2. Glyph Registry

`manifest.json` declares a reference zoom, the atlas image and rectangle index, and one entry per glyph. Each entry names its SVG source, its base size in pixels, its anchor as a unit offset within its own box, whether it accepts a tint, and the colour used when it is drawn untinted.

The 29 glyphs cover the object classes the exporter emits: buildings by function (residential, military, industrial, agricultural, civic, commercial, hangar, tower, bunker, castle, lighthouse, ruin, shed, tent, garage, container, bridge, generic), three badge overlays that mark a building as military, bunker or tower, vegetation (conifer, deciduous and palm trees, bush), rock boulders, fences, powerlines, and an unknown-prop fallback.

An anchor of `[0.5, 1.0]` puts the glyph's origin at the bottom centre, which is what makes a tree icon stand on its world position rather than float centred over it.

---

## 3. Rendering Contract

Glyphs are raster, not signed-distance, so the atlas is authored at the reference zoom and sampled around it. The classification that picks a glyph is `contracts_v2/rules/prefab-classify.json`, which maps each Enfusion prefab to a `render.iconKey`; a key with no entry in this manifest is a classification bug, not a missing asset, and the enumeration gate fails on it.

Served at `/map-assets/glyphs`, alongside but separate from the terrain mount, because the glyph set is shared by every terrain.

---

## 4. Consumers

- **`website-map-engine`** resolves each world object's icon key to an atlas rectangle and emits the draw.
- **`website-graphics-engine`** binds the atlas texture for the instanced glyph pass.
- **`developer-tools`** regenerates the atlas and its rectangle index from the SVG sources.
