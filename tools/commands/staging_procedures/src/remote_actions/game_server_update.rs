//! The Experimental dedicated server update: steamcmd `app_update 1890870 validate` into the
//! install the fleet units run.
//!
//! **Role:** builds the update script and the read of the installed build id.
//!
//! **Position:** used by `staging_dispatch.rs` (`staging update-game-server`) and the build identity.
//!
//! **Signals & state:** none; pure builders.
//!
//! **Invariants:** the update refuses while any fleet game server unit is active, since it
//! rewrites the files they run; the build id is read from the app manifest steamcmd writes, and
//! the answer names it.

use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::staging_settings::StagingSettings;

/// The Experimental dedicated server's Steam app.
pub(crate) const EXPERIMENTAL_SERVER_APP: u32 = 1_890_870;

/// The answer line naming the installed build.
const BUILD_LINE_PREFIX: &str = "buildid: ";

fn manifest(settings: &StagingSettings) -> String {
    shell_quote(&format!(
        "{}/steamapps/appmanifest_{EXPERIMENTAL_SERVER_APP}.acf",
        settings.server_install
    ))
}

fn build_line(settings: &StagingSettings) -> String {
    format!(
        "echo \"{BUILD_LINE_PREFIX}$(sed -n 's/.*\"buildid\"[[:space:]]*\"\\([0-9]*\\)\".*/\\1/p' {} | head -n 1)\"\n",
        manifest(settings)
    )
}

/// Updates and validates the install at `TBD_SERVER_DIR`.
pub(crate) fn update(settings: &StagingSettings) -> RemoteCommand {
    let install = shell_quote(&settings.server_install);
    RemoteCommand::change_script(
        "game server update",
        format!(
            "set -euo pipefail\n\
             command -v steamcmd > /dev/null || {{ echo 'steamcmd is not on PATH' >&2; exit 127; }}\n\
             active=\"$(systemctl --user list-units 'tbd-reforger@*' --state=active --no-legend --plain | wc -l)\"\n\
             if [ \"$active\" -ne 0 ]; then echo \"$active fleet game server(s) active: stop them first\" >&2; exit 1; fi\n\
             steamcmd +force_install_dir {install} +login anonymous +app_update {EXPERIMENTAL_SERVER_APP} validate +quit\n\
             {}",
            build_line(settings)
        ),
    )
}

/// Reads the installed build id.
pub(crate) fn installed_build(settings: &StagingSettings) -> RemoteCommand {
    RemoteCommand::read_script(
        "game server build",
        format!("set -euo pipefail\n{}", build_line(settings)),
    )
}

/// The build id of an update's or a read's answer, when steamcmd recorded one.
pub(crate) fn build_id(output: &str) -> Option<&str> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(BUILD_LINE_PREFIX))
        .filter(|id| !id.is_empty())
}
