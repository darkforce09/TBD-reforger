# Trim export tests

The `ballistics_trim_export_*` tests and the synthetic export they run over.

## Contents

```text
tools_v2/xtask/src/commands/ballistics/tests/trim_export/
├── mod.rs               the tests: determinism, schema validity, row evidence, four refusals
└── synthetic_export.rs  writes a one-mortar export and its oracle output into a temporary folder
```
