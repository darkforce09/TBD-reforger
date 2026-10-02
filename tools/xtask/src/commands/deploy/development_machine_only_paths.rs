//! The paths only a development machine holds, which both deploy rsyncs exclude.
//!
//! **Role:** lists, as rsync `--exclude` patterns, the build folders a development machine holds
//! beside `target/` (retired root-level ones and hand-set ones) and the local state of its agents
//! and tools, and renders them as the `--exclude=` arguments of [`exclude_arguments`].
//!
//! **Position:** read by the `rsync_argv` of `cargo xtask deploy website`
//! (`tools/xtask/src/commands/deploy/website/rsync_argv.rs`) and of `cargo xtask deploy
//! staging` (`tools/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`), which append these
//! arguments after their own exclusions (the secrets, `target/`, the asset trees, the reference
//! mods).
//!
//! **Signals & state:** none; a constant and a pure function.
//!
//! **Invariants:** every pattern starts with `/`, so rsync matches it at the checkout root only and
//! never a folder of the same name deeper in the tree; no tracked file matches a pattern, so the
//! host still receives the whole tracked tree; the host needs none of these paths. Both rsyncs run
//! with `--delete` and without `--delete-excluded`, so a copy of one of these paths that is already
//! on the host is neither updated nor deleted.

/// The rsync `--exclude` patterns of what only a development machine holds, anchored at the
/// checkout root: a trailing `/` matches a folder only, and `*` stands for any run of characters
/// within one path component.
pub(crate) const DEVELOPMENT_MACHINE_ONLY_PATHS: &[&str] = &[
    // Cargo target folders beside `target/`, tens of gigabytes together: the retired root-level
    // ones (`target-dev-api/`, `target-ci/`, `target-gate-*/`) that `cargo xtask platform wave
    // reclaim` deletes, and hand-set `CARGO_TARGET_DIR` folders such as `target-container/`. Every
    // tool now writes under `target/`, which each rsync excludes itself.
    "/target-*/",
    // The same folders beside the app, where cargo puts them when trunk builds the app under a
    // relative `CARGO_TARGET_DIR`.
    "/apps/frontend/target-*/",
    // The retired root-level gate app builds (`dist-gate-frontend/`, deleted by the same reclaim)
    // and a debug build of the app.
    "/dist-gate-*/",
    "/apps/frontend/dist-debug/",
    // The vanilla script count `cargo xtask mod compile` calibrates once per machine.
    "/.compile-vanilla-baseline",
    // Slice and ticket worktrees, each a whole checkout, and the wave gate's receipts.
    "/.ai/artifacts/worktrees/T-*/",
    "/.ai/artifacts/worktrees/TBD-*/",
    "/.ai/artifacts/last-verified",
    "/.ai/artifacts/verdicts/",
    // Claude Code's per-machine files and its agents' worktrees. `.claude/settings.json` is
    // tracked and ships with the checkout.
    "/.claude/settings.local.json",
    "/.claude/launch.json",
    "/.claude/worktrees/",
    "/.claude/checkpoints/",
    "/.claude/mailbox/",
    "/.claude/routines/.state/",
    "/.claude/scheduled_tasks.json",
    "/.claude/scheduled_tasks.lock",
    "/.claude/agent-registry.json",
    "/.claude/agent-memory-local",
    "/.claude/first-run",
    "/.claude/assistant-daemon-state.json",
    // Codex's configuration and hooks, and the MCP server configuration, which name this
    // machine's own paths.
    "/.codex/",
    "/.mcp.json",
];

/// One `--exclude=<pattern>` argument per [`DEVELOPMENT_MACHINE_ONLY_PATHS`] entry, in list order.
pub(crate) fn exclude_arguments() -> impl Iterator<Item = String> {
    DEVELOPMENT_MACHINE_ONLY_PATHS
        .iter()
        .map(|pattern| format!("--exclude={pattern}"))
}

#[cfg(test)]
#[path = "tests/development_machine_only_paths/tests.rs"]
mod tests;
