**Status:** live

# Admin help ticket design references

The design references of the [admin help ticket](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md):
one design-phase Stitch set of the admin tickets module.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/admin_help_ticket/visual_references/
└── admin_tickets_panel_mockup/  the ticket queue, one ticket's detail and the resolution actions
```

## Code

- [Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) — the admin code the module
  would join; no ticket code exists.

## Boundaries

- Depends on: nothing in the repository; the set is self-contained apart from what its html loads
  from the network.
- Used by: the admin help ticket specification and its folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
