# DOM oracle inventories

Frozen CSV and text inventories of the single-page app the DOM oracle was captured from: its route
table, its components, its data hooks, its npm dependencies and its CSS custom properties. Only the
route table is read by code; `gate s-routes` holds the live router to it.

## Contents

```text
tools_v2/developer-tools/fixtures/dom_oracle/manifests/
├── components.csv  the reference app's components: name, kind (shell, layout, page…) and source path
├── css_tokens.txt  the CSS custom properties the reference app defines, one `--name` per line
├── deps.csv        the reference app's npm packages, each with its disposition and replacement
├── hooks.csv       the reference app's data hooks: name, query or mutation, method and API URL
└── routes.csv      the route table the live router must equal, one row per route
```

## How it works

`gate s-routes` (`tools_v2/developer-tools/src/browser_testing/route_drift.rs`) reads the `ROUTES`
array of `apps/website/frontend/src/router.rs`, builds one row per `RouteDef` from its `path`,
`component`, `full_bleed`, `chromeless` and `auth` fields, sorts the rows by path and compares the
resulting CSV with `routes.csv` byte for byte. Equal files exit 0; otherwise the gate prints each
path that is missing on either side or whose row differs, and exits 1.

The other four files are reference data: no code, test or gate reads them.

## Format

- Encoding: UTF-8 with LF line endings. The CSV files have a header row and no quoting;
  `css_tokens.txt` is one property name per line.
- Schema: `routes.csv` has the header `path,component,fullBleed,chromeless,router_auth`, the rows
  sorted by path in byte order (`*`, the not-found route, first), the flags spelled `true` or
  `false`, the tier `none` or a role such as `admin`, and a final newline. Paths keep the `:param`
  shape the router uses (`/events/:id`).
- Adding a file: a route change edits `router.rs` and `routes.csv` together, then `cargo run -q -p
  developer-tools --bin gate -- s-routes` checks them. The other files are not edited.

## Producers and consumers

- Producers: people; `routes.csv` is edited by hand alongside the router.
- Consumers: `gate s-routes`, which people run; no xtask recipe or CI task runs it.

## Boundaries

- Depends on: the `RouteDef` field names of `apps/website/frontend/src/router.rs`, which the gate
  parses with regular expressions.
- Used by: `tools_v2/developer-tools/src/browser_testing/route_drift.rs`.
- Rules: `routes.csv` equals the router's table exactly, so a route added, removed or re-flagged
  changes both files in the same commit (`gate s-routes`).

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the browser gates `gate` runs.
