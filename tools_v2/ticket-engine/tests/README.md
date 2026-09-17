# Ticket Engine Test Suite (`ticket-engine/tests`)

Contains extracted test suites adhering to **Law 7 (No inline test modules)**:

- **`proptest_roundtrip.rs`**: Property tests ensuring `parse(render(t)) == t` for all possible ticket inputs.
- **`ticket_check_tests.rs`**: Extracted from `xtask/src/check.rs` (1,066 lines of unit tests).
- **`ticket_cmds_tests.rs`**: Extracted from `xtask/src/cmds.rs` (871 lines of unit tests).
- **`wave_lock_tests.rs`**: Extracted from `xtask/src/wave_lock.rs` (1,027 lines of unit tests).
