# Frontend layering, crate-edge mode

The second mode of the frontend-layering law in
`tools/foundation/repository_laws/src/workspace_laws/frontend_layering.rs`: the dependency
edges between frontend crates, judged with the same layer order and the same order types the
in-crate mode applies to modules.

## Contents

```text
tools/foundation/repository_laws/src/workspace_laws/frontend_layering/
└── crate_edges.rs   the frontend crates' places, the crate orders and every edge that breaks them
```

## How it works

`crate_edge_scan` reads the workspace members and places each frontend crate: a member directly
inside a configured layer folder (`FrontendLayerFolder`) has that folder's layer, with its package
name as its area; the app crate is the shell. It then walks every dependency edge (normal, dev and
build) from one placed crate to another and reports the edges that break:

- the layer order of the parent module (`breaks_order`): a lower layer depending on a higher one,
  pages and workspaces depending on each other, one page crate depending on another;
- a crate order (`SubAreaOrder` over package names, its `parent` the layer folder): a crate
  depending on a crate of its own or a higher tier, unless the tier is one mutual group; a
  test-only crate reached by any edge but a dev-dependency;
- the independence of two orders of one layer folder: their crates never depend on each other.

A crate in no layer folder, a crate in a layer folder that has orders but in none of them, and a
crate an order names that sits in another layer folder are findings, and so is a crate an order
names that no member carries.

## Boundaries

- Depends on: `super` (the layer order, `SubAreaOrder`, `LayeringEdge`) and
  `crate::workspace_members`.
- Used by: `super::frontend_layering_outcome`, which prints its findings after the in-crate
  mode's.
- Rules: the mode knows no crate or folder name; xtask passes every one of them.
