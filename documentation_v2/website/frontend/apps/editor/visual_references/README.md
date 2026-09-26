**Status:** live

# Mission Creator design references

The design references of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
shell, map canvas and version diff: five design-phase sets, kept for colour and layout context
rather than as an implementation source. The Arsenal's sets sit in
[its own folder](/documentation_v2/website/frontend/apps/editor/arsenal/visual_references/README.md).

## Contents

```text
documentation_v2/website/frontend/apps/editor/visual_references/
├── mission_creator_canvas_blueprint/  floating outliner, palette and properties over a contour map
├── mission_creator_prototype_mockup/  menu bar, objective logic and asset palette over a dark map
├── mission_creator_shell_blueprint/   a navigation rail, hierarchy and a unit popover over the map
├── mission_visual_diff_blueprint/     the prototype screen with deleted and added assets as ghosts
└── satellite_backdrop_render/         a satellite image of an island under a grid and reticles
```

## How it works

A set is a folder named after its subject and kind. A blueprint or mock-up holds the Stitch export
as an html file and its screenshot as a png, both named after the set; a render holds only its
png. Each set has a README that says what it shows and how the built editor differs.

None of the sets is the live design target. The editor follows the layout and interactions of the
Arma 3 Eden editor, styled with the Aegis glass tokens, as the
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) records it; the
sets contradict each other and settle styling only. Their colour tokens are the Aegis export in
the [design system](/documentation_v2/design_system/token_exports/aegis_design_tokens.md).

| Set | What it was drawn for |
|---|---|
| mission_creator_shell_blueprint | an early exploration of the editor chrome |
| mission_creator_canvas_blueprint | the map canvas styling |
| mission_creator_prototype_mockup | a layout exploration with objective logic, the source of the Aegis tokens |
| mission_visual_diff_blueprint | a visual diff of two mission versions on the map |
| satellite_backdrop_render | the mood of the satellite map |

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the built editor the sets were
  drawn for.

## Boundaries

- Depends on: nothing in the repository; each html loads its styles, fonts and any images from the
  network.
- Used by: the UX specification's Design section, which links the shell and canvas blueprints; the
  Mission Creator documentation README.
- Rules: a set is kept as captured and never edited to match the built editor; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
