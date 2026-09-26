**Status:** live

# Mission visual diff blueprint

Design-phase reference for a visual diff of two [mission](/documentation_v2/glossary/g_to_m.md#mission)
versions in the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator): the changes
between versions drawn on the map instead of read as text. It gives colour and layout context and
is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/visual_references/mission_visual_diff_blueprint/
├── mission_visual_diff_blueprint.html  the Stitch export of the diff screen
└── mission_visual_diff_blueprint.png   its screenshot
```

## How it works

The blueprint draws the screen of the
[prototype mock-up](/documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_prototype_mockup/README.md),
with the same menus, outliner, objective popover and asset palette, and the "Visual Diff" toggle
on. A deleted asset, a tank, shows as a red dashed ghost with a hover label ending "Tank (Deleted)",
and an added one as a green icon labelled "+ Supply Truck (Added)".

Nothing in the built UI draws a diff on the map: the Mission Creator has no "Visual Diff" toggle.
The mission library's upload preview compares an uploaded version with the current one as a list
instead: per collection, the counts of added, removed, moved and edited rows and a few named rows,
matched by id (`apps/website/frontend/src/v2/pages/mission_hub/library/mission_diff.rs`). A
map diff of two versions is planned; the
[Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
lists the work.

## Code

- [Mission library](/apps/website/frontend/src/v2/pages/mission_hub/library/) — the list
  comparison of an upload with the current version, which stands in for the map diff.
- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor whose screen the
  blueprint draws.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and one Google-hosted placeholder image, which
  the html loads when opened; the png needs nothing.
- Used by: the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
