//! Verifies where the server keeps its map assets, before the deploy touches anything.
//!
//! The API resolves `/map-assets` from `MAP_ASSETS_DIR` (default `../../../assets_v2/terrains`,
//! relative to the process working directory) and `ServeDir` does no I/O when it is constructed, so
//! a directory that is absent or in the wrong place is not a boot failure — it is a 404 per request
//! with nothing in the log. The asset relocation moved that directory's canonical location, and the
//! rsync deliberately never carries it, so the only thing standing between a working deploy and a
//! silently empty map is whether the server's own copy sits where this build will look.
//!
//! This is also the last moment at which that is cheap to check: the next step is an `--delete`
//! rsync of the monorepo root.

/// What the server's asset layout is, as judged by the remote probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLayout {
    /// `assets_v2/terrains` is populated (its registry is present). Nothing to do.
    Ready,
    /// The server's older `packages/map-assets` tree is still the only copy.
    OldPackagesTree,
    /// Neither exists. Legitimate for a library-only host that never serves the map.
    Absent,
    /// The probe itself did not answer — unreachable host, missing remote directory, ssh failure.
    Indeterminate(i32),
}

/// Exit code the probe uses for "still on the old layout".
const OLD_PACKAGES_TREE: i32 = 10;
/// Exit code the probe uses for "no asset tree at all".
const ABSENT: i32 = 11;

/// The file whose presence means the terrain tree is populated: the registry every terrain hangs
/// off. It is never rsynced, so it cannot arrive by accident, and a bare directory cannot stand in
/// for it — Docker creates a missing bind-mount source as an empty root-owned directory, and a
/// half-finished `mkdir -p` leaves the same thing.
pub const TERRAIN_TREE_MARKER: &str = "assets_v2/terrains/terrain-registry.json";

/// The remote shell that answers the question, as an exit code rather than parsed text.
pub fn probe_script(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}' 2>/dev/null || exit 12; \
         if [ -f {TERRAIN_TREE_MARKER} ]; then exit 0; fi; \
         if [ -d packages/map-assets ]; then exit {OLD_PACKAGES_TREE}; fi; \
         exit {ABSENT}"
    )
}

pub fn classify(code: i32) -> AssetLayout {
    match code {
        0 => AssetLayout::Ready,
        OLD_PACKAGES_TREE => AssetLayout::OldPackagesTree,
        ABSENT => AssetLayout::Absent,
        other => AssetLayout::Indeterminate(other),
    }
}

/// The exact commands an operator runs on the server to adopt the new layout.
///
/// Deliberately not run by the deploy. This moves ~590 MB of production data, it is the kind of
/// thing that wants a human looking at the disk, and a half-finished automatic `mv` across a
/// filesystem boundary would be far worse than a refused deploy.
pub fn remediation(remote_dir: &str) -> String {
    format!(
        "    cd '{remote_dir}'\n\
         \x20   mkdir -p assets_v2/terrains\n\
         \x20   mv packages/map-assets/everon \\\n\
         \x20      packages/map-assets/arland \\\n\
         \x20      packages/map-assets/terrain-registry.json \\\n\
         \x20      assets_v2/terrains/"
    )
}

/// Prints the verdict. `Ok(())` lets the deploy continue; `Err(code)` stops it before the rsync.
pub fn report(layout: AssetLayout, remote_dir: &str) -> Result<(), u8> {
    match layout {
        AssetLayout::Ready => {
            println!("    {remote_dir}/{TERRAIN_TREE_MARKER} present — terrain tree populated");
            Ok(())
        }
        AssetLayout::Absent => {
            // A site that only serves the mission library never asks for `/map-assets`, and
            // documentation_v2/runbooks/website_deployment.md documents that as a supported
            // first cutover. Say so and continue.
            println!(
                "    WARN: no map asset tree on the server (neither assets_v2/terrains nor \
                 packages/map-assets)."
            );
            println!(
                "          The site will serve every /map-assets request as 404. That is expected \
                 for a library-only host."
            );
            Ok(())
        }
        AssetLayout::OldPackagesTree => {
            eprintln!(
                "ERROR: {remote_dir}/assets_v2/terrains is missing, but the pre-relocation \
                 packages/map-assets is present."
            );
            eprintln!(
                "       This build resolves /map-assets from assets_v2/terrains, so deploying now \
                 would serve 404 for every map asset."
            );
            eprintln!("       Move the tree on the server, then re-run the deploy:");
            eprintln!("{}", remediation(remote_dir));
            eprintln!(
                "       The glyph atlas needs no action — it is tracked and arrives with the rsync."
            );
            Err(1)
        }
        AssetLayout::Indeterminate(code) => {
            eprintln!(
                "ERROR: could not determine the server's map asset layout (probe exit {code})."
            );
            eprintln!(
                "       Refusing to continue: the next step is an --delete rsync, and it is not \
                 safe to run that against a server whose layout could not be read."
            );
            Err(1)
        }
    }
}
