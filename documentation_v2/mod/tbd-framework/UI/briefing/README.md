**Status:** live

# Briefing screen documentation

The documentation of the Briefing tab, where every player reads their side's orders over the map
and deploys into their [slot](/documentation_v2/glossary.md#slot).

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/
├── briefing_specification.md  the screen as built: modes, pages, deploy, data, design, open work
└── visual_references/         the Stitch mockup sets and the Arma 3 captures
```

## Code

- [Briefing screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the screen,
  its navigation and its pages
- [Briefing](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/) — the catalog, the
  per-side payload and the ready tally
- [Briefing layouts](/apps/mod/tbd-framework/UI/layouts/Session/Briefing/) — the shell and panel
  layouts

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it; it stays within 500
  lines, and the capture breakdown lives in the reference screenshots README.

## Related documentation

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) — the
  roster and kit inspector the ORBAT page reuses
