**Status:** live

# Personnel roster page documentation

The feature documentation of the `/admin/personnel` page, where administrators search the member
roster, ban, unban and warn members, and resync every member's
[role](/documentation_v2/glossary.md#role) from Discord, with the page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/administration/personnel/
├── personnel_roster_page.md  the feature doc: the roster, the dossier, moderation and the API
└── visual_references/        the design-phase blueprint of a roster table beside a dossier
```

## How it works

Read [personnel_roster_page.md](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each call means in the
[API](/documentation_v2/glossary.md#api), including the role change the API refuses, and compares
the built page with the blueprint in `visual_references/`. The blueprint is a design-phase
reference: it filters by rank and shows photos, while the built page filters by ban status and shows
initials. The code folder's README lists the page's files.

## Code

- [Personnel roster page](/apps/website/frontend/src/v2/pages/administration/personnel/) — the
  route component `PersonnelRosterPage`, the roster, the dossier and the moderation dialogs.
- [Administration domain](/apps/website/api_v2/src/administration/) — the roster, ban, warning and
  role routes the page calls.

## Boundaries

- Depends on: the feature doc template; the page code, the administration handlers and the ticket
  registry the feature doc is written from.
- Used by: the [personnel](/documentation_v2/glossary.md#personnel) glossary entry, the page's
  in-code README and the administration domain README, which link the feature doc; the
  administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Administration domain](/apps/website/api_v2/src/administration/README.md) — the API side of the
  roster, moderation and the role resync.
