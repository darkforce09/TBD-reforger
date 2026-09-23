# Crate-wide verification

`architecture_rules.rs` enforces source size, directory ownership, module documentation, and dependency boundaries. `source_inspection.rs` reads Rust source for those rules. `support/` holds temporary-repository and ticket-construction helpers shared by feature tests.

Feature unit tests live beside their owning source modules. All test files contain at most 1,000 raw lines.
