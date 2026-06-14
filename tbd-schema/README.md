# tbd-schema

Shared contracts for the TBD Reforger event platform. This is the most important
artifact in the project: changes here ripple to the web wizard, the validation
engine, the Enfusion framework loader, the ORBAT/slotting systems, and the
partner VOIP bridge. Version it carefully.

## Contents

| Path | Purpose |
|---|---|
| [`schema/mission.schema.json`](schema/mission.schema.json) | Mission JSON contract **v1.1** (`slots[]` required) |
| [`schema/registry.schema.json`](schema/registry.schema.json) | Registry format: alias to prefab GUID + web metadata |
| [`golden-missions/`](golden-missions/) | Hand-maintained missions that must always validate and load |
| [`registry/registry.example.json`](registry/registry.example.json) | Example registry used by the compatibility test |
| [`bridge/bridge-contract.md`](bridge/bridge-contract.md) | Game to voice-client bridge contract (VOIP integration boundary) |
| [`bridge/bridge-messages.schema.json`](bridge/bridge-messages.schema.json) | JSON Schema for bridge messages |
| [`bridge/samples/`](bridge/samples/) | Canonical bridge message examples |
| [`spikes/voip-spike-brief.md`](spikes/voip-spike-brief.md) | Phase 0.2 brief handed to the partner VOIP track |
| [`spikes/registry-poc-0.4.md`](spikes/registry-poc-0.4.md) | Registry alias → GUID POC (GREEN; superseded for spawn) |

## Schema versions

- **1.0** — initial mission contract (factions, orbat, zones, flow)
- **1.1** — adds required `slots[]` with exact spawn positions per ORBAT slot instance

Use `scripts/flatten-orbat-slots.mjs` to generate `slots[]` from ORBAT definitions.
Validate a single file: `node scripts/validate-file.mjs path/to/mission.json`.
| [`spikes/rest-spike-0.1.md`](spikes/rest-spike-0.1.md) | REST loop spike (GREEN) |

## Rules

- Published missions are **immutable**; edits create a new version (content-hash id).
- The wizard and missions speak **registry aliases** (`kit:us_rifleman`,
  `comp:checkpoint_small`) — never raw prefab GUIDs.
- Schema changes go through a lightweight RFC. The framework advertises the schema
  versions it supports; the backend refuses to serve a mission a server cannot run.
- Golden missions are the compatibility suite: they must always validate (CI) and
  load (manual, pre-release, in the Enfusion loader).

## Validate

The compatibility test validates every golden mission against the mission schema,
the example registry against the registry schema, and the bridge samples against
the bridge schema.

```bash
npm install
npm run validate
```

Run this in CI for the web validator and manually before each release for the
Enfusion loader.
