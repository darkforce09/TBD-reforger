# Tool test support source

The two process-global locks the tool tests share and the test checkout root resolved under one of
them.

## Contents

```text
tools/foundation/tool_test_support/src/
├── environment_lock.rs        `ENV_LOCK` and `lock_env`: tests that write a process variable
├── lib.rs                     the crate root: module header, `mod` lines and the re-exports
├── prelude.rs                 `lock_env`, `CwdGuard` and `test_repo_root` for glob import
├── test_checkout_root.rs      `test_repo_root`: the checkout root, resolved under the working-directory lock
├── tests/                     the test root from nested tooling folders, under the guard
└── working_directory_lock.rs  `CwdGuard` and `resolve_under_lock`: the one working-directory lock
```

## How it works

- `environment_lock`: `lock_env` takes `ENV_LOCK` and takes over a poisoned lock; it serialises the
  writers of process variables only, so a `PATH` a test sets keeps `/usr/bin:/bin`.
- `working_directory_lock`: `CwdGuard::enter_resolved` takes the lock, then resolves the target, then
  changes into it, so resolution never races the test that owns the working directory;
  `resolve_under_lock` runs a reader under the same lock. The guard changes back on drop.
- `test_checkout_root`: `test_repo_root` is `repository_root::find_repository_root()` under that lock,
  so a test reads the checkout it runs in, never a sibling worktree a shared build folder compiled.

## Boundaries

- Depends on: `repository_root` and `std::sync`.
- Used by: the crate root's re-exports, read by the tool crates' tests.
- Rules: the mutex is not reentrant, so a caller holding a `CwdGuard` never calls `test_repo_root`;
  `tests/test_checkout_root_tests.rs` resolves the root from nested tooling folders under the guard.
