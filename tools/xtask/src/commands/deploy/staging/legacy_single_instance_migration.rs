//! `--migrate-single-instance`: retire a host's single-instance game server for the fleet.
//!
//! **Role:** builds the check that refuses a fleet deploy while the single-instance units are still
//! installed ([`single_instance_units_absent_payload`]), and the migration that stops and disables
//! them and archives their unit files, the single host agent's configuration folder and binary,
//! the profile and the server config ([`migration_payload`]).
//!
//! **Position:** called by the deploy pipeline in `super::remote`: the check runs before the rsync
//! when the flag is absent, the migration after the rsync and before any instance file is written
//! when it is present. `TBD_PROFILE_DIR` names the single-instance profile.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** every name this module retires is spelled as the single-instance install wrote
//! it on the host, in kebab-case (`fleet-host-agent.service`, `~/.config/fleet-host-agent/`,
//! `~/.local/bin/fleet-host-agent`), while everything the fleet installs carries the package's
//! snake_case name `fleet_host_agent`; so the old configuration folder holds nothing of the fleet
//! and is retired whole, and this module names no snake_case agent path. The migration moves and
//! never deletes: everything it retires lands in one new folder
//! `~/tbd/retired/single-instance-<UTC time>/` under a fixed archive name; it refuses a profile
//! inside the fleet root; with nothing left to retire it creates no folder and succeeds; it ends
//! by proving neither single-instance unit is loaded, because instance 1 takes the ports the
//! single server held.

use super::fleet_instances::FLEET_ROOT_UNDER_HOME;

/// The units of the single-instance server, as the host holds them.
pub const SINGLE_INSTANCE_UNITS: [&str; 2] = ["tbd-reforger.service", "fleet-host-agent.service"];
/// The single host agent's configuration folder under `$HOME`, retired whole.
pub const SINGLE_INSTANCE_AGENT_CONFIGURATION: &str = ".config/fleet-host-agent";
/// The single host agent's binary under `$HOME`.
pub const SINGLE_INSTANCE_AGENT_BINARY: &str = ".local/bin/fleet-host-agent";
/// The archive names of the configuration folder and the binary, which share a file name.
const ARCHIVED_AGENT_CONFIGURATION: &str = "fleet-host-agent-configuration";
const ARCHIVED_AGENT_BINARY: &str = "fleet-host-agent-binary";

const UNITS_ABSENT: &str = r#"set -uo pipefail
installed=""
for unit in @UNITS@; do
  if [ "$(systemctl --user show -p LoadState --value "$unit" 2>/dev/null || true)" = "loaded" ]; then
    installed="$installed $unit"
  fi
done
if [ -n "$installed" ]; then
  echo "FAIL: the single-instance units are still installed:$installed. Instance 1 takes their ports; rerun with --migrate-single-instance to stop, disable and archive them." >&2
  exit 1
fi
echo "  no single-instance unit is installed"
"#;

/// Exit 1, naming them, while either single-instance unit is still installed.
pub fn single_instance_units_absent_payload() -> String {
    UNITS_ABSENT.replace("@UNITS@", &SINGLE_INSTANCE_UNITS.join(" "))
}

const MIGRATION: &str = r#"set -euo pipefail
umask 077
PROFILE='@PROFILE@'
SERVER_CONFIG='@SERVER_CONFIG@'
FLEET="$HOME/@FLEET@"
UNITS="$HOME/.config/systemd/user"
case "$PROFILE/" in
  "$FLEET"/*) echo "FAIL: TBD_PROFILE_DIR $PROFILE is inside the fleet root $FLEET; refusing to archive it" >&2; exit 1 ;;
esac
for unit in @UNITS@; do
  if [ "$(systemctl --user show -p LoadState --value "$unit" 2>/dev/null || true)" = "loaded" ]; then
    systemctl --user stop "$unit" || true
    systemctl --user disable "$unit" 2>/dev/null || true
    echo "  stopped and disabled $unit"
  fi
done
retire=()
archived_as=()
retire_if_present() {
  if [ -e "$1" ]; then retire+=("$1"); archived_as+=("$2"); fi
}
for unit in @UNITS@; do
  retire_if_present "$UNITS/$unit" "$unit"
done
retire_if_present "$HOME/@AGENT_CONFIGURATION@" @ARCHIVED_AGENT_CONFIGURATION@
retire_if_present "$HOME/@AGENT_BINARY@" @ARCHIVED_AGENT_BINARY@
retire_if_present "$PROFILE" "$(basename "$PROFILE")"
retire_if_present "$SERVER_CONFIG" "$(basename "$SERVER_CONFIG")"
if [ "${#retire[@]}" -eq 0 ]; then
  echo "  nothing of the single-instance server is left to archive"
else
  ARCHIVE="$HOME/tbd/retired/single-instance-$(date -u +%Y%m%dT%H%M%SZ)"
  mkdir -p "$HOME/tbd/retired"
  mkdir "$ARCHIVE"
  for i in "${!retire[@]}"; do
    mv "${retire[$i]}" "$ARCHIVE/${archived_as[$i]}"
    echo "  archived ${retire[$i]} as ${archived_as[$i]}"
  done
  echo "  single-instance files archived in $ARCHIVE"
fi
systemctl --user daemon-reload
for unit in @UNITS@; do
  if [ "$(systemctl --user show -p LoadState --value "$unit" 2>/dev/null || true)" = "loaded" ]; then
    echo "FAIL: $unit is still loaded after the migration" >&2
    exit 1
  fi
done
"#;

/// Stop and disable the single-instance units, then move their unit files, the single agent's
/// configuration folder and binary, `profile_dir` and `server_config` into one timestamped folder
/// under `~/tbd/retired/`.
pub fn migration_payload(profile_dir: &str, server_config: &str) -> String {
    MIGRATION
        .replace("@PROFILE@", profile_dir)
        .replace("@SERVER_CONFIG@", server_config)
        .replace("@FLEET@", FLEET_ROOT_UNDER_HOME)
        .replace("@UNITS@", &SINGLE_INSTANCE_UNITS.join(" "))
        .replace("@AGENT_CONFIGURATION@", SINGLE_INSTANCE_AGENT_CONFIGURATION)
        .replace(
            "@ARCHIVED_AGENT_CONFIGURATION@",
            ARCHIVED_AGENT_CONFIGURATION,
        )
        .replace("@AGENT_BINARY@", SINGLE_INSTANCE_AGENT_BINARY)
        .replace("@ARCHIVED_AGENT_BINARY@", ARCHIVED_AGENT_BINARY)
}

/// The dry-run line for the check without the flag.
pub fn units_absent_plan_line() -> String {
    format!(
        "[dry-run] refuse while {} is installed",
        SINGLE_INSTANCE_UNITS.join(" or ")
    )
}

/// The dry-run line for the migration with the flag.
pub fn migration_plan_line(profile_dir: &str, server_config: &str) -> String {
    format!(
        "[dry-run] migrate: stop and disable {}; archive their unit files, \
         ~/{SINGLE_INSTANCE_AGENT_CONFIGURATION}/, ~/{SINGLE_INSTANCE_AGENT_BINARY}, {profile_dir} \
         and {server_config} under ~/tbd/retired/single-instance-<UTC time>/",
        SINGLE_INSTANCE_UNITS.join(" and ")
    )
}

#[cfg(test)]
#[path = "tests/legacy_single_instance_migration/tests.rs"]
mod tests;
