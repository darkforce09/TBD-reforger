# Ticket engine integration tests

The crate's tests that sit outside `src/`: the compile-fail test that keeps the scope `Domain` enum
closed, and the recorded agent CLI outputs that the token parsers are tested against. The unit
tests live beside their modules, in the `tests/` folders under `src/`.

- `trybuild.rs` compiles `fail/mod_frontend.rs`, which names `Domain::Frontend`, and passes only
  while that fails to compile with the output recorded in `fail/mod_frontend.stderr`.
- `fixtures/execution_receipts/` holds three recorded final JSON outputs of an agent run: the
  Cursor agent's dialect, Claude's dialect, and one without a usage block. The metrics unit tests
  in `tools_v2/ticket-engine/src/metrics/tests/` and the `platform slice-run` tests in
  `tools_v2/xtask/src/commands/platform/tests/slice_execution/tests.rs` both read them, and they
  stay byte for byte as recorded.
