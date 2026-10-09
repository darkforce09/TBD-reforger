**Status:** live

# Discord identity link documentation

The documentation of how a player links their game identity to their platform account with
`#tbd link`, so an [event](/documentation/glossary/a_to_f.md#event)'s attendance and statistics reach
their profile.

## Contents

```text
documentation/mod/tbd-framework/UI/discord_identity_link/
├── discord_identity_link_specification.md  the flow as built, its API calls, design target, decisions
└── visual_references/                      the Stitch mockup set of a link dialog
```

## Code

- [Backend bridge](/mod/tbd-framework/Scripts/Game/TBD/API/) — `TBD_IdentityLink`, the
  command, and `TBD_PlayerIdentity`, the identity accessor
- [Identity and access](/crates/api/api_identity_and_access/src/) — the link code and link
  confirmation handlers

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md).
- Used by: the [mod UI index](/documentation/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path.
