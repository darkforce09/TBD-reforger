//! `cargo xtask mk reclaim-target-ci`: deletes the continuous-integration scratch folder.
//!
//! **Role:** [`reclaim_target_ci`] is the body of `cargo xtask mk reclaim-target-ci`.
//! **Position:** dispatched by the `mk` recipes (`crate::build_lane::recipes`); reclaims under the
//! build output folder of `repository_layout::build_output`.
//! **Signals & state:** none held; [`reclaim_target_ci`] deletes folders under the root it is
//! given.
//! **Invariants:** a reclaim path never equals the shared cache and always ends in its own name.

use std::path::Path;

use crate::Result;
use process_runner::Run;
use repository_layout::build_output::{
    BUILD_OUTPUT_FOLDER, CONTINUOUS_INTEGRATION_SUBFOLDER, ToolchainEnvironment,
    build_output_subfolder,
};

/// Delete the continuous-integration scratch folder of each toolchain environment
/// (`root/target/host/ci`, `root/target/container/ci`), and the retired root-level `root/target-ci`
/// when the machine still holds it. Exit 0 when both are gone, 1 on a refusal.
///
/// `root` is a parameter so the destructive path is testable against a scratch tree, and no path
/// is derived from anything but `root`, so a slice's own folder is unreachable by construction.
/// Two refusals guard every deletion and run before any of them: the path never equals the shared
/// cache `root/target`, and it ends in its own name (`/target/<environment>/ci`, `/target-ci`). An empty `root`
/// yields a relative path, which fails the second.
pub(crate) fn reclaim_target_ci(root: &Path) -> Result<u8> {
    let warm = root.join(BUILD_OUTPUT_FOLDER).display().to_string();
    let folders = [
        (
            build_output_subfolder(
                root,
                ToolchainEnvironment::Host,
                CONTINUOUS_INTEGRATION_SUBFOLDER,
            ),
            "/target/host/ci",
        ),
        (
            build_output_subfolder(
                root,
                ToolchainEnvironment::Container,
                CONTINUOUS_INTEGRATION_SUBFOLDER,
            ),
            "/target/container/ci",
        ),
        (root.join("target-ci"), "/target-ci"),
    ];

    for (folder, suffix) in &folders {
        let shown = folder.display().to_string();
        if shown == warm || shown == format!("{warm}/") {
            println!("REFUSING: reclaim path collides with shared target/ ({warm})");
            return Ok(1);
        }
        if !(shown.ends_with(suffix) || shown.ends_with(&format!("{suffix}/"))) {
            println!("REFUSING: path '{shown}' is not …{suffix}");
            return Ok(1);
        }
    }
    for (folder, _) in &folders {
        if !folder.exists() {
            println!("already absent: {}", folder.display());
            continue;
        }
        // `du -sh` on the inherited terminal: its output (size TAB path) is part of the target's
        // contract.
        let _ = Run::new("du").arg("-sh").arg(folder).terminal();
        std::fs::remove_dir_all(folder)?;
        println!(
            "removed {} (shared target/ left intact at {warm})",
            folder.display()
        );
    }
    Ok(0)
}
