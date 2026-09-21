//! The API's user-systemd unit ships in the repository and is installed by hand.
//!
//! The deploy restarts the unit by name and never installs it. That is deliberate while the
//! platform is in alpha: the unit's shape is still moving, and an installer that overwrote the
//! host's copy on every deploy would make the repository template the only place an operator could
//! safely change anything. What the deploy owes the operator instead is the exact install command,
//! printed where it is actionable — when the restart fails because the unit is not there.

/// The template, relative to the repository root; the rsync carries it to the server.
pub const UNIT_TEMPLATE: &str = "scripts/deploy/tbd-website-api.service";

/// The one-time install: render the template's `TBD_REPO_DIR_PLACEHOLDER` for this remote
/// directory, drop it into the user unit directory, reload, enable.
///
/// The placeholder is substituted **without** its leading slash — the template already spells
/// `/TBD_REPO_DIR_PLACEHOLDER/…`, so a value with a slash would double it.
pub fn install_command(remote_dir: &str, unit: &str) -> String {
    let placeholder_value = remote_dir.trim_start_matches('/');
    format!(
        "cd '{remote_dir}' && mkdir -p ~/.config/systemd/user && \
         sed 's|TBD_REPO_DIR_PLACEHOLDER|{placeholder_value}|g' {UNIT_TEMPLATE} \
         > ~/.config/systemd/user/{unit} && \
         systemctl --user daemon-reload && systemctl --user enable --now {unit}"
    )
}
