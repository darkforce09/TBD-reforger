# Arsenal & Faction Fixtures (`contracts_v2/fixtures/registry/`)

Eight samples covering the arsenal, loadout, faction, and alias contracts.

| Fixture | Role |
|:---|:---|
| `registry-items.sample.json` | Item catalog sample; ingest normalisation tests |
| `registry-compat.sample.json` | Compatibility edge sample; matrix checks |
| `loadout-export.sample.json` | Loadout golden; serde round trip must be byte-identical |
| `loadout-export.v2.sample.json` | Versioned loadout branch, attachments and ammo counts |
| `faction-library.sample.json` | Faction roles, gear, and vehicle rosters |
| `mission-editor-payload.sample.json` | Editor payload shape |
| `registry.example.json` | Alias registry: semantic key to prefab GUID |
| `registry.vanilla-poc.json` | Alias resolution against vanilla content |

The **live** Workbench catalogs are not here — they are in [`../../catalogs/`](../../catalogs/), because they are production content rather than test data.

---

## Invariants

1. **Round-trip byte parity.** `loadout-export.sample.json` must deserialize and re-serialize identically. This is what holds the hand-written loadout model to its schema, since that model is deliberately excluded from codegen.
2. **Layer separation.** The item catalog speaks full Enfusion resource names and never aliases; the alias registry speaks semantic keys and resolves to GUIDs at load. A fixture that mixes the two would hide a layering bug.
