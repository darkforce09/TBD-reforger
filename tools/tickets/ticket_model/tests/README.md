# Ticket model integration tests

The crate's one test outside `src/`: the compile-fail test that keeps the scope `Domain` enum
closed. The unit tests live beside their modules, in the `tests/` folders under `src/`.

- `trybuild.rs` compiles `fail/mod_frontend.rs`, which names `Domain::Frontend`, and passes only
  while that fails to compile with the output recorded in `fail/mod_frontend.stderr`. The
  recording pins the fixture's line and column, so an edit above the `Domain::Frontend` line
  updates the recording too.
