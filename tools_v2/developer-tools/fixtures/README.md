# Developer tools gate fixtures

Committed reference data that the `gate` binary's browser gates compare the single-page app against.
The fixtures the library's unit tests load live in `tools_v2/developer-tools/test_fixtures/`.

## Contents

```text
tools_v2/developer-tools/fixtures/
└── dom_oracle/  DOM goldens, screenshots and route inventories for `gate v-suite` and `gate s-routes`
```

## How it works

The gate code resolves each fixture path from the checkout root at run time, reads it, and never
writes it except through `gate v-suite accept`, which replaces one route's golden. Nothing here is
compiled into the crate, so `cargo test -p developer-tools` does not read this tree.

## Format

- Encoding: JSON, PNG, CSV and plain text, laid out per gate; each subfolder's README gives its
  format.
- Schema: set by the gate code in `tools_v2/developer-tools/src/browser_testing/`, which reads these
  paths directly.
- Adding a file: through the gate that owns the subfolder (`gate v-suite accept`), or by hand for
  the route table.

## Producers and consumers

- Producers: `gate v-suite accept`, and people editing the route table.
- Consumers: `gate v-suite verify` and `gate s-routes`.

## Boundaries

- Depends on: the single-page app's built output, router and API fixtures in
  `apps/website/frontend/`.
- Used by: `tools_v2/developer-tools/src/browser_testing/`.
- Rules: the prose rules of `tools_v2/xtask/src/tests/tooling_prose_rules.rs` exempt this tree from
  the ticket-id and Rust-file-name rules, since the goldens are captured pages; the gates address it
  by its full path, so a move updates
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs` and
  `tools_v2/developer-tools/src/browser_testing/route_drift.rs` in the same change.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the browser gates.
