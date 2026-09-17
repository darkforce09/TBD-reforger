# Core Foundational Utilities (`xtask/src/core`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Shared low-level primitives for the task runner.

---

## Submodules

- **`root.rs`**: Traverses ancestor directories to discover `.git` and return the canonical repository root path.
- **`constants.rs`**: Auto-generated header comments and terminal formatting strings.
- **`prompt.rs`**: User confirmation prompt utilities.
- **`hostrun.rs`**: Container-to-host execution bridge.
- **`test_env.rs`**: Environment variable isolation helpers for test execution.
