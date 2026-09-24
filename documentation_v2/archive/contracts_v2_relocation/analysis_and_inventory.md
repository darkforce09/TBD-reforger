# Contract Census (`contracts_v2`)

Every file in this tree, what it governs, and what reads it.

---

## 1. Definitions (25)

| Group | Schemas | Primary consumers |
|:---|:---|:---|
| Mission | `mission`, `mission-editor-payload` | API validation, map-engine scenario model, mod mission loader |
| Arsenal | `registry-items`, `registry-compat`, `registry`, `loadout-export` | API arsenal and compatibility endpoints, frontend, mod loadout equip |
| Factions | `faction-library` | API faction templates, frontend palette |
| Terrain | `terrain-manifest`, `terrain-anchors`, `terrain-registry` | Map-engine world loader, export verifiers |
| Map objects | `map-object-catalog`, `-enums`, `-instance`, `-prefab`, `-region`, `-resolved`, `-roads`, `-type-inventory` | World export pipeline, map-engine streaming |
| Geometry | `blas-manifest`, `building-blueprint`, `building-instances`, `prefab-descriptor` | Blueprint compiler, spatial indexes |
| Cartography | `locations`, `height-labels` | Label placement and symbology |
| Voice | `bridge-messages` | API, mod radio system, external voice client |

`bridge-messages.md` accompanies the voice schema with the handshake and lifecycle narrative the schema alone cannot express.

## 2. Rules (2)

| File | Role |
|:---|:---|
| `prefab-classify.json` | 98 ordered rules plus a fallback. Each matches Enfusion resource-name substrings and emits the object's kind, class, AI taxonomy summary, spatial model, gameplay properties, and render icon key. First match wins, so rules are appended, never reordered. |
| `kit-aliases.json` | Maps Enfusion resource names to the semantic kit aliases missions are authored against. |

Both shape a committed artifact rather than a live read: editing `prefab-classify.json` changes nothing until the object catalog is rebuilt, so a gate run after an edit validates the rules and not the shipped catalog.

## 3. Catalogs (2)

| File | Role |
|:---|:---|
| `registry-items.workbench.json` | Flat item catalog exported from the Workbench; ingested into Postgres and served as the Virtual Arsenal. |
| `registry-compat.workbench.json` | Directed weapon-to-attachment compatibility graph from the same export. |

Produced by the Workbench export plugins and replaced wholesale. `cargo xtask db seed` imports both.

## 4. Fixtures (49)

| Directory | Count | Role |
|:---|:---:|:---|
| `missions/valid/` | 9 | Missions that must always parse, validate and compile. Covers two-faction compilation, slot loadout coverage, absent versus present slot elevation, empty warning fields, and the schema 1.3 additions (tasks, tactical graphics, wire fields). |
| `missions/invalid/` | 6 | One rejection gate each: cargo container typo, kit alias absent from the registry, unbounded net range, slot callsign separator, newline in a zone label, zone rules key typo. |
| `map/` | 20 | Chunk samples in both binary and JSON, density fixture pairs, catalog, instances, prefabs, regions, resolved rows, roads, three terrain-manifest variants, a terrain registry sample, phased import fixtures, and a locations sample. |
| `registry/` | 8 | Item, compatibility, loadout (v1 and v2), faction library, editor payload, alias registry, and vanilla proof-of-concept samples. |
| `enfusion_samples/` | 10 | Raw payloads as the mod emits them: root, meta, faction, orbatFaction, group, role, slot, zone, shape, circle. |
| `bridge_samples/` | 6 | Voice lifecycle: hello, spawn, net change, push-to-talk, stage change, death. |

Binary and JSON twins in `map/` are a parity pair: a decode of one must equal a decode of the other, which is what keeps the binary reader honest.

---

## 5. Retired

`spikes/` held four proof-of-concept notes from before the implementations existed: a VOIP brief and capability matrix, an alias-registry prototype, and a REST evaluation. Each is superseded by shipped code and current documentation. `VERSION` and `CHANGELOG.md` recorded a package version nothing read; the schemas carry their own versions and git carries the history.
