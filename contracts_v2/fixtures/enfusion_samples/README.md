# Enfusion Runtime Samples (`contracts_v2/fixtures/enfusion_samples/`)

Ten payloads captured exactly as the game mod emits them.

| Sample | Emitted for |
|:---|:---|
| `root.sample.json` | The top-level mission envelope |
| `meta.sample.json` | Mission metadata block |
| `faction.sample.json` | A faction |
| `orbatFaction.sample.json` | A faction in ORBAT form |
| `group.sample.json` | A group within an ORBAT |
| `role.sample.json` | A role within a group |
| `slot.sample.json` | A player slot |
| `zone.sample.json` | A mission zone |
| `shape.sample.json` | A zone's polygon shape |
| `circle.sample.json` | A zone's circular shape |

These are the game's output, not the platform's input format, and that is the point: they pin what the mod actually writes, including the engine's own JSON quirks. The Enfusion JSON writer emits some floats in a form that its own parser then reads back imprecisely, so a sample here is the evidence for how the API must be tolerant when reading, rather than an assertion that the mod is well behaved.
