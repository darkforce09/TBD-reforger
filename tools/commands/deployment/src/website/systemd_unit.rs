//! The API's user-systemd unit ships in the repository and is installed by hand.
//!
//! **Role:** the API's user-systemd unit name, its repository template and the install command the
//! deploy prints.
//! **Position:** a child of [`crate::website`], whose restart step names the unit and prints the
//! install command when the unit is missing.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the default unit name is the file name of the shipped template; the deploy never
//! installs the unit.
//!
//! The deploy restarts the unit by name and never installs it. That is deliberate while the
//! platform is in alpha: the unit's shape is still moving, and an installer that overwrote the
//! host's copy on every deploy would make the repository template the only place an operator could
//! safely change anything. What the deploy owes the operator instead is the exact install command,
//! printed where it is actionable — when the restart fails because the unit is not there.

/// The unit `cargo xtask deploy website` restarts when `TBD_WEBSITE_SYSTEMD_UNIT` is unset: the
/// file name of the API template this repository ships, so the default and the template can
/// never name two different units.
pub(crate) fn default_unit_name() -> &'static str {
    let template = repository_layout::WEBSITE_API_UNIT;
    template.rsplit('/').next().unwrap_or(template)
}

/// The template for one unit, relative to the repository root.
///
/// Derived from the unit's own file name rather than fixed, so a deploy pointed at a different
/// unit through `TBD_WEBSITE_SYSTEMD_UNIT` installs that unit's template and not another's.
pub(crate) fn template_for(unit: &str) -> String {
    format!("{}/{unit}", repository_layout::SYSTEMD_UNITS_DIR)
}

/// The one-time install: render the template's `TBD_REPO_DIR_PLACEHOLDER` for this remote
/// directory, drop it into the user unit directory, reload, enable.
///
/// The placeholder is substituted **without** its leading slash — the template already spells
/// `/TBD_REPO_DIR_PLACEHOLDER/…`, so a value with a slash would double it.
pub(crate) fn install_command(remote_dir: &str, unit: &str) -> String {
    let placeholder_value = remote_dir.trim_start_matches('/');
    let template = template_for(unit);
    format!(
        "cd '{remote_dir}' && mkdir -p ~/.config/systemd/user && \
         sed 's|TBD_REPO_DIR_PLACEHOLDER|{placeholder_value}|g' {template} \
         > ~/.config/systemd/user/{unit} && \
         systemctl --user daemon-reload && systemctl --user enable --now {unit}"
    )
}
