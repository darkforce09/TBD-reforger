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
    vec![
        "-e".into(),
        rsync_e.to_string(),
        "-avz".into(),
        "--delete".into(),
        "--exclude=.git/".into(),
        "--exclude=target/".into(),
        "--exclude=target-gate-*/".into(),
        "--exclude=dist-gate-*/".into(),
        "--exclude=**/node_modules/".into(),
        "--exclude=apps/website/frontend/dist/".into(),
        "--exclude=apps/website/api_v2/.env".into(),
        "--exclude=apps/website/api_v2/.tools/".into(),
        "--exclude=scripts/deploy/deploy.env".into(),
        // The served terrain tree: ~590 MB of LFS content plus the gitignored tile pyramids
        // nested under it. The server carries its own copy; it is never pushed from a dev PC.
        "--exclude=assets_v2/terrains/".into(),
        // Local export intermediates, 1.5 GB and gitignored (`.gitignore`, `assets_v2/scratch/`).
        // These used to live *inside* the terrain directory, so one exclusion covered both; the
        // relocation made them siblings and the terrain exclusion alone no longer reaches them.
        "--exclude=assets_v2/scratch/".into(),
        // `assets_v2/glyphs/` is deliberately NOT excluded. It is 188 KB, it is tracked, and the
        // API serves it at `/map-assets/glyphs`, so the server takes its copy from this rsync.
        //
        // The pre-relocation asset location. Nothing in the repo writes here any more, so this
        // exclusion exists purely to keep `--delete` away from a server that still holds its
        // assets at the old path. Retire it once every host has moved to `assets_v2/terrains`.
        "--exclude=packages/".into(),
        "--exclude=apps/mod/crf_framework/".into(),
        "--exclude=apps/mod/vanilla_reference/".into(),
        "--exclude=apps/mod/playable_selector/".into(),
        "--exclude=apps/mod/.local-test-profile/".into(),
        mono.to_string(),
        dest.to_string(),
    ]
}

/// The `--exclude=` values alone, in argv order. For printing a plan and for assertions.
pub fn exclusions(argv: &[String]) -> Vec<&str> {
    argv.iter()
        .filter_map(|a| a.strip_prefix("--exclude="))
        .collect()
}
