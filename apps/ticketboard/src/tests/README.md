# Crate-wide verification

`architecture_rules.rs` enforces source size, module roots, no flat sources, the crate README's
presence, dependency boundaries, and external test placement over the egui half. It reads Rust
source through `ticketboard_model::test_support::source_inspection`, which the dev-dependency on
`ticketboard_model` with its `test_fixtures` feature provides, together with the temporary-repository
and ticket-construction helpers the rendering tests use.

Feature unit tests live beside their owning source modules; the model tests live in
`tools/tickets/ticketboard_model/`. All test files contain at most 1,000 raw lines.
