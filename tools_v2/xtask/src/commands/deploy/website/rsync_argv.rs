//! The `rsync` argument list of `cargo xtask deploy website`, and the lines its dry run prints.
//!
//! **Role:** builds the argv after the program name, source and destination included, whose
//! exclude list is also the `--delete` guard, and renders that list for the dry run.
//!
//! **Position:** called by `tools_v2/xtask/src/commands/deploy/website.rs`; the exclusions it
//! shares with `cargo xtask deploy staging` come from
//! [`crate::commands::deploy::development_machine_only_paths`].
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** the argv ends with the source and the destination; the dry run prints every
//! exclusion of the argv, in argv order.

use crate::commands::deploy::development_machine_only_paths;

/// The `rsync` argv after the program name, source and destination included.
///
/// Pure, so the exclude list can be asserted without spawning — the sibling `deploy staging` lane
/// keeps its argv builder pure for the same reason.
///
/// ── WHY `--exclude` IS ALSO A DELETE GUARD ───────────────────────────────────────────────────
///
/// This is an `--delete` rsync of the whole monorepo root, and no `--delete-excluded` is passed.
/// rsync therefore treats every excluded path as *protected on the receiver*: it is neither sent
/// nor removed. An exclusion here consequently does two jobs, and dropping one silently enables
/// deletion of whatever it named on the server.
pub fn rsync_argv(rsync_e: &str, mono: &str, dest: &str) -> Vec<String> {
    let mut argv: Vec<String> = vec![
        "-e".into(),
        rsync_e.to_string(),
        "-avz".into(),
        "--delete".into(),
        "--exclude=.git/".into(),
        // All build output, the gates' private folders included: every tool writes under
        // `target/` (one subfolder per purpose).
        "--exclude=target/".into(),
        "--exclude=**/node_modules/".into(),
        "--exclude=apps/website/frontend/dist/".into(),
        "--exclude=apps/website/api_v2/.env".into(),
        "--exclude=apps/website/api_v2/.tools/".into(),
        format!("--exclude={}", crate::core::repository_layout::DEPLOY_ENV),
        // The served terrain tree: ~590 MB of LFS content plus the gitignored tile pyramids
        // nested under it. The server carries its own copy; it is never pushed from a dev PC.
        "--exclude=assets_v2/terrains/".into(),
        // Local export intermediates, 1.5 GB and gitignored (`.gitignore`, `assets_v2/scratch/`).
        // They are the terrain tree's sibling, so the terrain exclusion above does not reach them.
        "--exclude=assets_v2/scratch/".into(),
        "--exclude=assets_v2/equipment/".into(),
        // `assets_v2/glyphs/` is deliberately NOT excluded. It is 188 KB, it is tracked, and the
        // API serves it at `/map-assets/glyphs`, so the server takes its copy from this rsync.
        //
        // Nothing in the repository writes to `packages/`. This exclusion exists purely to keep
        // `--delete` away from a server that still holds its map assets there; it can go once
        // every host serves them from `assets_v2/terrains`.
        "--exclude=packages/".into(),
        "--exclude=apps/mod/crf_framework/".into(),
        "--exclude=apps/mod/vanilla_reference/".into(),
        "--exclude=apps/mod/playable_selector/".into(),
        "--exclude=apps/mod/.local-test-profile/".into(),
    ];
    // What only a development machine holds, excluded by `deploy staging` too: the retired and
    // hand-set cargo target folders beside `target/`, the retired gate and debug app builds,
    // worktrees, and the local state of its agents and tools.
    argv.extend(development_machine_only_paths::exclude_arguments());
    argv.push(mono.to_string());
    argv.push(dest.to_string());
    argv
}

/// The `--exclude=` values alone, in argv order. For printing a plan and for assertions.
pub fn exclusions(argv: &[String]) -> Vec<&str> {
    argv.iter()
        .filter_map(|a| a.strip_prefix("--exclude="))
        .collect()
}

/// The dry run's rsync lines: the transfer, then one `--exclude=` line per exclusion, in argv
/// order.
///
/// The exclude list is the whole point of a dry run: with `--delete` and no `--delete-excluded`,
/// every entry is also what keeps rsync from removing that path on the server, so each one is
/// printed rather than elided behind the ellipsis.
pub fn dry_run_lines(host: &str, remote_dir: &str) -> Vec<String> {
    let argv = rsync_argv("", "", "");
    let mut lines = vec![format!(
        "[dry-run] rsync -avz --delete … {host}:{remote_dir}/"
    )];
    lines.extend(
        exclusions(&argv)
            .into_iter()
            .map(|excluded| format!("[dry-run]   --exclude={excluded}")),
    );
    lines
}
