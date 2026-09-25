**Status:** live

# Discord identity link design references

The design references of the [identity link](/documentation_v2/mod/tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md):
one design-phase Stitch set of an in-game link dialog.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/discord_identity_link/visual_references/
└── identity_link_macos_modal_sheet_mockup/  the link dialog: status, code, two linking methods
```

## Code

- [Backend bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/) — the built chat command.

## Boundaries

- Depends on: nothing in the repository; the set is self-contained apart from what its html loads
  from the network.
- Used by: the identity link specification and the identity link folder README.
- Rules: a set is kept as captured and never edited to match the built flow; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
