# world

The static, immutable facts about the terrain being drawn: the scene anchor, and the
synthetic instance scenes measured against it.

## Contents

- `mod.rs`
- `scene.rs`
- `tests`

## Boundaries

Nothing here is authored, undoable or persisted — it describes the ground. The GPU instance
layouts these scenes fill live in `website-graphics-engine` (`draw::instances`), which must
never learn the 12.8 km square this module is written in.
