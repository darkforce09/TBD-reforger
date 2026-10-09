# Browser gate fixtures

Committed reference data that the browser gates of `browser_gate_suites` compare the single-page
app against, through the `gate` binary.

## Contents

```text
tools/browser_testing/browser_gate_suites/fixtures/
└── dom_oracle/  DOM goldens, screenshots and route inventories for `gate v-suite` and `gate s-routes`
```

## How it works

The gate code resolves each fixture path from the checkout root at run time, reads it, and never
writes it except through `gate v-suite accept`, which replaces one route's golden. Nothing here is
compiled into the crate, so `cargo test -p developer_tools` does not read this tree.

## Format

- Encoding: JSON, PNG, CSV and plain text, laid out per gate; each subfolder's README gives its
  format.
- Schema: set by the gate code in `tools/browser_testing/browser_gate_suites/`, which reads these
  paths directly.
- Adding a file: through the gate that owns the subfolder (`gate v-suite accept`), or by hand for
  the route table.

## Producers and consumers

- Producers: `gate v-suite accept`, and people editing the route table.
- Consumers: `gate v-suite verify` and `gate s-routes`.

## Boundaries

- Depends on: the single-page app's built output, router and API fixtures in
  `crates/frontend/shell/frontend_application/`.
- Used by: `tools/browser_testing/browser_gate_suites/`.
- Rules: the prose rules of `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs` exempt this tree from
  the ticket-id and Rust-file-name rules, since the goldens are captured pages; the gates address it
  by its full path, so a move updates
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs` and
  `tools/browser_testing/browser_gate_suites/src/route_drift.rs` in the same change.

## Related documentation

- [Editor gates](/documentation/runbooks/editor_gates.md) — running the browser gates.
