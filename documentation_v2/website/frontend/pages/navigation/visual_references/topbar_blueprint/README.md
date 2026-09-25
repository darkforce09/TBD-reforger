**Status:** live

# Top bar blueprint

Design-phase reference for the top bar of the navigation frame, the header above every chromed
page: a breadcrumb on the left and the viewer's identity pill, avatar and account menu on the right.
It gives colour and layout context and is not an implementation source; the built UI is the Leptos
code in `apps/website/frontend/src/v2/pages/navigation/top_nav.rs`.

## Contents

```text
documentation_v2/website/frontend/pages/navigation/visual_references/topbar_blueprint/
└── design_tokens.md  the Stitch design brief with its colour, type, radius and spacing tokens
```

## How it works

The set is a Stitch brief, not a drawing: YAML front matter of tokens, then a prompt describing
the bar. It asks for a 64px bar on `#0b1120` with a `#1e3a5f` bottom border; a breadcrumb with a
muted parent and a white current page; a green pill reading "Linked: 765611..." on green at 20%
opacity; a round avatar beside the name "Admin Dave"; and an open menu, a `#1f2937` card with a
blue border, holding "Settings", "Link Arma Identity" and "Sign Out" in red. Its primary colour is
`#3b82f6` and its type is Inter.

The built bar keeps the height, the breadcrumb weights, the pill's "Linked: " prefix with a
truncated id, the avatar and name, the three menu items and the red "Sign Out". It differs: the bar
is the theme's translucent low surface with a blur; the theme's primary is `#adc6ff` and
`#3b82f6` is its action colour; the pill fills with `#064e3b`; the menu is translucent glass with
a grey border, icons and a divider; and the bar adds a chevron, an "Unlinked" pill, a signed-out
"Sign in with Discord" link and a "TBD Reforger" fallback for routes without a breadcrumb. The
[app layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
feature doc holds the full comparison.

## Code

- [Navigation frame](/apps/website/frontend/src/v2/pages/navigation/) — `top_nav.rs`, the bar this
  set was drawn for.

## Boundaries

- Depends on: nothing; the brief is plain text.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured, its React wording included; the token file keeps the name
  `design_tokens.md`.
