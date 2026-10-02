//! The `rsync` argument vector, and the two exit-code readings the deploy relies on.
//!
//! **Role:** the pure `rsync` argv builder, the mapping of a spawn that did not run to an exit
//! code, and the four-outcome reading of `mod remote-logs`; the `ssh` argv comes from
//! [`crate::core::secure_shell_transport`].
//!
//! **Position:** used by [`super::Runner`] and by the pipeline in `super::fleet_deploy`.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** no argument vector carries the ssh password (`sshpass -e` reads it from the
//! spawned process's environment); the rsync excludes every path the host keeps for itself, which
//! `--delete` then leaves alone, and every path only a development machine holds.

use super::*;

/// The full `rsync` argv. The exclude list is the licence boundary described in [`super`]'s header;
/// the ORDER is the bash's, because a wave log diff should not show reordered flags. The paths
/// only a development machine holds, which `deploy website` excludes too, follow this lane's own
/// exclusions.
pub fn rsync_argv(base: &SshBase, mono_root: &Path, host: &str, remote_dir: &str) -> Vec<String> {
    let mut argv: Vec<String> = vec![
        "rsync".into(),
        "-e".into(),
        base.rsync_e(),
        "-avz".into(),
        "--delete".into(),
        "--exclude=.git/".into(),
        "--exclude=apps/mod/crf_framework/".into(),
        "--exclude=apps/mod/vanilla_reference/".into(),
        "--exclude=apps/mod/playable_selector/".into(),
        "--exclude=apps/mod/Tbd_framework/".into(),
        "--exclude=apps/mod/.local-test-profile/".into(),
        "--exclude=**/node_modules/".into(),
        "--exclude=apps/api/.tools/".into(),
        "--exclude=apps/api/.env".into(),
        "--exclude=apps/mod/tbd-export/".into(),
        "--exclude=apps/mod/tbd-emcp/".into(),
        format!("--exclude={}", crate::core::repository_layout::DEPLOY_ENV),
        // Build output and the map asset trees: a game-server host needs none of it, and the
        // scratch tree alone is 1.5 GB of gitignored export intermediates. Excluded paths are
        // also protected from `--delete` (there is no `--delete-excluded`).
        "--exclude=target/".into(),
        "--exclude=assets/terrains/".into(),
        "--exclude=assets/scratch/".into(),
        "--exclude=assets/equipment/".into(),
        // The app `cargo xtask deploy website` built on the host, in the checkout both deploys
        // share, and which the staging Caddy serves: this rsync must neither replace it with the
        // development machine's build nor delete it.
        "--exclude=apps/frontend/dist/".into(),
    ];
    argv.extend(crate::commands::deploy::development_machine_only_paths::exclude_arguments());
    argv.push(format!("{}/", mono_root.display()));
    argv.push(format!("{host}:{remote_dir}/"));
    argv
}

pub(super) fn not_run_exit(e: &NotRun) -> u8 {
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

/// READ THE EXIT CODE of `mod remote-logs`, do not just inherit it.
///
/// `remote-log-grep` is a FOUR-outcome check and this script is the consumer that pinned `2`:
///
/// ```text
/// 0 HEALTHY  ·  1 FAIL  ·  2 PARTIAL (booted, nobody joined yet)  ·  3 ENVIRONMENT
/// ```
///
/// Were this the last statement in the file, under `set -e` the deploy would simply exit with
/// whatever it returned. `2` is the NORMAL state immediately after a deploy — nobody has had time
/// to join — so every healthy deploy reported failure to any caller reading `!= 0`, and the fix
/// people reach for when a green run keeps "failing" is to stop believing the gate. `3` is the
/// opposite hazard and must never be soft: it means no log was examined at all, so it says nothing
/// about the mod and cannot be allowed to read as success.
///
/// The same contract applies to `cargo xtask mcp wb-logs` and `cargo xtask mod spawn-verify`: an
/// inverted reading of either passes only on a stale build. Do not build a staging check on a
/// `!= 0` reading of any of the three.
pub fn v6_verdict(code: i32) -> u8 {
    match code {
        0 => {
            println!(
                "V6 HEALTHY — current build, mission loaded, reached LOBBY, a player was seated."
            );
            0
        }
        2 => {
            println!(
                "V6 PARTIAL — boot is healthy, no player has joined yet. This is the expected result"
            );
            println!("   for a fresh deploy and is NOT a failure.");
            0
        }
        1 => {
            eprintln!(
                "V6 FAIL — a required structural line is missing, or an error class is present."
            );
            1
        }
        3 => {
            eprintln!(
                "V6 ENVIRONMENT — the log could not be obtained, so nothing was examined. This says"
            );
            eprintln!("   NOTHING about the mod, and is not a pass.");
            1
        }
        other => {
            eprintln!(
                "V6 returned an unexpected status {other} — treating as failure rather than guessing."
            );
            1
        }
    }
}
