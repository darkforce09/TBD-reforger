# Tool test support

The `tool_test_support` crate: the locks and the checkout root the tool crates' tests share. A test
that changes a process variable or the working directory, or reads fixtures from the checkout,
takes them from here, so every crate's tests serialise on the same two locks.

## Contents

```text
tools/foundation/tool_test_support/
├── Cargo.toml  the `tool_test_support` library package: `repository_root` only, layout tier 1, a dev-dependency only
└── src/        the environment lock, the working-directory lock and its guard, the test checkout root
```

## How it works

```text
lock_env()                    ── ENV_LOCK: tests that write PATH or another process variable
CwdGuard::enter(dir)          ──┐
CwdGuard::enter_resolved(f)   ──┼── one working-directory lock; the guard changes back on drop
resolve_under_lock(f)         ──┤
test_repo_root()              ──┘── repository_root::find_repository_root(), under that lock
```

`cargo test` runs tests on several threads, and the working directory and the environment are
process state. Every test that moves the working directory holds the one lock, and so does every
reader that resolves the checkout from it, since a scratch checkout carries the root marker the walk
looks for. The crate is consumed only from `[dev-dependencies]`, and its items are not
`cfg(test)`-gated, because a `cfg(test)` item is invisible to the crates that test with it.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p tool_test_support   # the test root from nested tooling folders, under the guard
```

## Configuration

No feature and no environment variable.

## Public surface

- At the crate root: `ENV_LOCK`, `lock_env`, `CwdGuard`, `resolve_under_lock` and
  `test_repo_root`.
- `prelude`: `lock_env`, `CwdGuard` and `test_repo_root`.

## Boundaries

- Depends on: `repository_root` (the root walk).
- Used by: the tests of `xtask`, from its `[dev-dependencies]`; the `PATH` guard those tests hold
  the environment lock for is `process_runner::PathGuard`.
- Rules: tier 1 of `tools/foundation` (`cargo xtask verify crate-tiers`); no `[dependencies]`
  table of a workspace member names it.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
