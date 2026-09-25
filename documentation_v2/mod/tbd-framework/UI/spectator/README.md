**Status:** live

# Spectator documentation

The documentation of the spectator, where a dead player watches the rest of an
[event](/documentation_v2/glossary.md#event) under one life: its camera, its roster and the design
references its overlays were drawn from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/
├── spectator_specification.md  the spectator as built, its wire, design target, open work, decisions
└── visual_references/          the Stitch mockup sets and the Arma 3 captures
```

## Code

- [Spectator](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/) — the camera, the
  controller, the roster rules and the streaming host
- [Spectator roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) —
  `TBD_SpectatorScreen`
- [Spectator layouts](/apps/mod/tbd-framework/UI/layouts/Session/Spectator/) — none yet; the
  roster uses the shared shell

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it.
