# Upstream code leak scan

The body of `cargo xtask verify no-crf-leak`: the four checks that keep code and asset GUIDs from
the read-only upstream reference frameworks out of the shipping [mod](/documentation_v2/glossary/g_to_m.md#mod).
The parent file `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs` holds the
lanes, the patterns, the output log and the tests wiring.

## Contents

```text
tools_v2/xtask/src/verifications/licensing/upstream_code_leaks/
└── verify_crf_leak.rs  the entry, the identifier and GUID arms, the vanilla pak probe, grep-compatible reading
```

## Boundaries

- Depends on: the parent file's `Lanes`, `Log` and constants; `verification_core` (`scan`,
  `proc::Run`, `Verdict`, `NotRun`); `regex`; the `grep` binary for the vanilla `.pak` probe;
  `crate::core::repository_layout::documentation` for the two documents the failure text names.
- Used by: the parent file, which re-exports `verify_crf_leak` to
  `tools_v2/xtask/src/commands/verify/dispatch.rs`.
- Rules: every line goes through `Log::say`, so the printed text and the text the tests assert
  are one; a tree the scan could not read exits 2 through `refuse`, never 0
  (`an_absent_mod_tree_does_not_read_as_clean` in
  `tools_v2/xtask/src/verifications/licensing/tests/upstream_code_leaks/tests.rs`).
