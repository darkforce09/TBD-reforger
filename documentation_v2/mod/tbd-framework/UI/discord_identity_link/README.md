**Status:** live

# Discord identity link documentation

The documentation of how a player links their game identity to their platform account with
`#tbd link`, so an [event](/documentation_v2/glossary/a_to_f.md#event)'s attendance and statistics reach
their profile.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/discord_identity_link/
├── discord_identity_link_specification.md  the flow as built, its API calls, design target, decisions
└── visual_references/                      the Stitch mockup set of a link dialog
```

## Code

- [Backend bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/) — `TBD_IdentityLink`, the
  command, and `TBD_PlayerIdentity`, the identity accessor
- [Identity and access](/apps/website/api_v2/src/identity_and_access/) — the link code and link
  confirmation handlers

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path.
