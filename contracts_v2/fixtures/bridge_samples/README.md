# Voice Bridge Samples (`contracts_v2/fixtures/bridge_samples/`)

Six messages spanning the voice client's lifecycle, conforming to `definitions/bridge-messages.schema.json`.

| Sample | Point in the lifecycle |
|:---|:---|
| `hello.json` | Client handshake and capability advertisement |
| `spawn.json` | A player enters the world and gains a voice presence |
| `net_change.json` | A radio net assignment changes |
| `ptt.json` | Push-to-talk transitions |
| `stage_change.json` | The mission stage changes, moving which nets are live |
| `death.json` | A player dies and loses voice presence |

The bridge is an external process on the far side of a socket, maintained separately from this repository. These samples are the shared reference both sides implement against, which is what lets either side change independently without a coordinated release.
