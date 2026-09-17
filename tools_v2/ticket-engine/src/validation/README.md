# Ticket Validation Suite (`ticket-engine/src/validation`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Complete static analysis and integrity check suite for the ticket corpus.

Relocated from `xtask/src/check.rs` (which had 1,264 lines of production code and 1,066 lines of inline tests). Decomposed into modular single-responsibility files (<350 LOC each) with tests extracted into `ticket-engine/tests/ticket_check_tests.rs`.

---

## Submodules

- **`schema.rs`**: Validates every ticket against `schema.json` via `jsonschema`.
- **`scope.rs`**: Enforces valid `owns` scopes, classification rules, and scope surface consistency.
- **`body_rules.rs`**: Enforces strict word caps:
  - Title: maximum 10 words.
  - Summary: maximum 40 words.
  - Body lines: maximum 30 words per line.
- **`gates.rs`**: Readiness criteria (acceptance rules present, user story present) and ship criteria (SHA stamped).
- **`debt.rs`**: Ratchet debt pins (`TITLE_DEBT_PIN`, `MAIN_GOAL_DEBT_PIN`, `MIGRATION_LEGACY_PIN`).
- **`hierarchy.rs`**: Parent-child relationship integrity and dotted number verification.
- **`runner.rs`**: Top-level entry point `validate_corpus(&Corpus) -> ValidationReport`.
