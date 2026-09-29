//! The Discord outage: an `HTTPS_PROXY` drop-in on the API's user unit that sends every outbound
//! HTTPS request to a closed loopback port.
//!
//! **Role:** builds the install and the removal of the drop-in (each with a daemon reload and an
//! API restart) and the read of whether it is present.
//!
//! **Position:** used by the Discord procedure's outage and recovery steps, its recovery action
//! list, and `staging status`.
//!
//! **Signals & state:** none; pure builders.
//!
//! **Invariants:** the drop-in is one named file under the API unit's `.d` folder, so removing it
//! restores the unit exactly; the proxy is loopback, so no request leaves the host while it is
//! installed.

use crate::commands::staging::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::commands::staging::staging_settings::StagingSettings;

/// The drop-in's file name.
pub(crate) const OUTAGE_DROPIN_FILE: &str = "staging-discord-outage.conf";

/// The proxy every outbound HTTPS request goes to while the drop-in is installed: a loopback port
/// nothing listens on, so each request fails at once as unavailable.
pub(crate) const BLACKHOLE_PROXY: &str = "http://127.0.0.1:9";

/// The drop-in's path on the host.
pub(crate) fn dropin_path(settings: &StagingSettings) -> String {
    format!(
        "{}/.config/systemd/user/{}.d/{OUTAGE_DROPIN_FILE}",
        settings.home, settings.api_unit
    )
}

/// Installs the drop-in and restarts the API.
pub(crate) fn install(settings: &StagingSettings) -> RemoteCommand {
    let path = dropin_path(settings);
    let folder = shell_quote(path.rsplit_once('/').map_or(".", |(folder, _)| folder));
    RemoteCommand::change_script(
        "outage drop-in",
        format!(
            "set -euo pipefail\n\
             mkdir -p {folder}\n\
             printf '[Service]\\nEnvironment=HTTPS_PROXY={BLACKHOLE_PROXY}\\n' > {path}\n\
             systemctl --user daemon-reload\n\
             systemctl --user restart {unit}\n\
             echo \"installed {OUTAGE_DROPIN_FILE}; {unit} restarted\"\n",
            path = shell_quote(&path),
            unit = shell_quote(&settings.api_unit),
        ),
    )
}

/// Removes the drop-in and restarts the API.
pub(crate) fn remove(settings: &StagingSettings) -> RemoteCommand {
    RemoteCommand::change_script(
        "outage drop-in",
        format!(
            "set -euo pipefail\n\
             rm -f {path}\n\
             systemctl --user daemon-reload\n\
             systemctl --user restart {unit}\n\
             echo \"removed {OUTAGE_DROPIN_FILE}; {unit} restarted\"\n",
            path = shell_quote(&dropin_path(settings)),
            unit = shell_quote(&settings.api_unit),
        ),
    )
}

/// Prints `present` or `absent`.
pub(crate) fn state(settings: &StagingSettings) -> RemoteCommand {
    RemoteCommand::read(
        "outage drop-in",
        format!(
            "if [ -e {} ]; then echo present; else echo absent; fi",
            shell_quote(&dropin_path(settings))
        ),
    )
}
