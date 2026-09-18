# Classification & Alias Rules (`contracts_v2/rules/`)

Deterministic lookup tables consulted while producing platform data.

---

## 1. Contents

```text
rules/
├── README.md
├── prefab-classify.json                <-- Enfusion prefab → gameplay classification
└── kit-aliases.json                    <-- Enfusion resource name → mission kit alias
```

---

## 2. `prefab-classify.json`

98 ordered rules and one fallback. Each rule matches on substrings of an Enfusion resource name and emits the full classification for every prefab that matches:

```json
{
  "match": { "resourceNameContains": ["World/Locations/"] },
  "kind": "prop",
  "class": "composition",
  "ai": {
    "summary": "Location composition prefab (whole-POI grouping entity); zero extents; children export as their own rows — never render.",
    "taxonomyPath": "misc/prop/composition",
    "confidence": 0.95
  },
  "spatial": { "model": "obb", "pivot": "base", "halfExtentsM": { "x": 0.5, "y": 0.5, "z": 0.5 }, "heightM": 1.0 },
  "gameplay": { "cover": { … }, "movement": { … }, "lineOfSight": "none" }
}
```

Each rule therefore drives four consumers at once: the AI taxonomy summary, the spatial model used for collision and line of sight, the gameplay properties (cover, movement blocking, sight blocking), and the render icon key that selects a glyph.

**Rules are evaluated in file order and the first substring match wins.** New rules are *appended*, never inserted: an appended rule can only claim prefabs that currently fall through to `fallback`, whereas an inserted one silently reclassifies whatever a rule below it was already matching.

The fallback classifies anything unmatched as an unknown prop with `confidence: 0.0` and `needsReview: true`, which is what makes an unclassified prefab visible rather than plausible.

**An edit here changes nothing until the catalog is rebuilt.** `assets_v2/terrains/<terrain>/objects/prefabs.rkyv` is a committed artifact carrying the classification from the last export run. A gate run straight after a rule edit is validating the rules, not the shipped catalog; re-verify after `cargo xtask map export-terrain`.

---

## 3. `kit-aliases.json`

Maps Enfusion resource names to the semantic kit aliases missions are authored against, so a mission says `kit:us_rifleman` rather than a GUID path. The map engine's scenario compiler embeds this table at build time and resolves aliases while compiling. A mission naming an alias absent from this table is rejected, and `fixtures/missions/invalid/kit-alias-not-in-registry.json` pins that rejection.
