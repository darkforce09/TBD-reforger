# DOM oracle fixtures

The reference data the browser gates hold the single-page app to: the goldens of each route's
rendered page, and the inventories of the app they were captured from, among them the route table
the live router must match.

## Contents

```text
tools_v2/developer-tools/fixtures/dom_oracle/
├── manifests/      inventories of the reference app; `routes.csv` is the route table `gate s-routes` checks
└── oracle-freeze/  per-route DOM goldens and screenshots, and the manifest `gate v-suite` maintains
```

## How it works

Two gates of the `gate` binary read this folder, both from
`tools_v2/developer-tools/src/browser_testing/`. `gate v-suite verify` captures each route of the
built app with its API calls answered from `apps/website/frontend/tests/fixtures/api/` and diffs the
page's DOM against `oracle-freeze/`; `gate s-routes` compares the router's route table with
`manifests/routes.csv`. Neither gate regenerates the folder: `gate v-suite accept` replaces one
route's golden at a time, and `routes.csv` is edited by hand.

## Format

- Encoding: JSON goldens and PNG screenshots in `oracle-freeze/`, CSV and plain text in
  `manifests/`; each child README gives the layout.
- Schema: a golden is the tree the serializer in
  `tools_v2/developer-tools/src/browser_testing/fixture_injection.rs` emits; `routes.csv` mirrors
  the `RouteDef` fields of `apps/website/frontend/src/router.rs`.
- Adding a file: through `gate v-suite accept` for a golden, by hand for `routes.csv`.

## Producers and consumers

- Producers: `gate v-suite accept`, and people editing `manifests/routes.csv`.
- Consumers: `gate v-suite verify`, run by `cargo xtask mk leptos-gates`; `gate s-routes`, run by
  people.

## Boundaries

- Depends on: the page serializer and the API fixtures named above, and the router's route table.
- Used by: `tools_v2/developer-tools/src/browser_testing/dom_oracle/` and
  `tools_v2/developer-tools/src/browser_testing/route_drift.rs`.
- Rules: the goldens are never regenerated in bulk; the route table and `routes.csv` change in the
  same commit.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the browser gates and their
  preflight.
