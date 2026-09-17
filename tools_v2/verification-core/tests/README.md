# Verification Core Unit Tests (`verification-core/tests`)

Unit and property tests for `verification-core` components.

In accordance with **Law 7 (File Size Limits & Test Placement)**:
- No inline `#[cfg(test)] mod tests { ... }` modules exist inside production files.
- Tests live in dedicated test files in this directory.
- Test files remain strictly under 1,000 LOC.
