**Status:** live

# Modpacks page documentation

The feature documentation of the `/modpacks` page, where members see the modpacks the servers run
and the [mods](/documentation_v2/glossary.md#mod) each needs, and administrators maintain them,
with the page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/
├── modpacks_page.md    the feature doc: the pack list, the dossier, administration and the API
└── visual_references/  the design-phase blueprint of one wide modpack card
```

## How it works

Read [modpacks_page.md](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each modpack route of the
[API](/documentation_v2/glossary.md#api) does, including the delete refusal, and compares the
built page with the blueprint in `visual_references/`. The blueprint is a design-phase reference:
it draws one wide card with a connect button, while the built page lists every pack beside one
pack's dossier and adds the administrator's form. The code folder's README lists the page's
files.

## Code

- [Modpacks page](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/) — the route
  component `ModpacksPage`, the pack list, the dossier and the edit form.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the modpack routes.

## Boundaries

- Depends on: the feature doc template; the page code, the modpack handlers and the ticket registry
  the feature doc is written from.
- Used by: the in-code READMEs of the modpacks page and of the doctrine and info pages, which link
  the feature doc; the doctrine and info pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the API side
  of the modpacks.
