//! `--migrate-single-instance`: retire a host's single-instance game server for the fleet.
//!
//! **Role:** builds the check that refuses a fleet deploy while the single-instance units are still
//! installed ([`single_instance_units_absent_payload`]), and the migration that stops and disables
//! them and archives their unit files, profile, server config and host agent files
//! ([`migration_payload`]).
//!
//! **Position:** called by the deploy pipeline in `super::remote`: the check runs before the rsync
//! when the flag is absent, the migration after the rsync and before any instance file is written
//! when it is present. `TBD_PROFILE_DIR` names the single-instance profile.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the migration moves and never deletes: everything it retires lands in one new
//! folder `~/tbd/retired/single-instance-<UTC time>/`; it refuses a profile inside the fleet root;
//! with nothing left to retire it creates no folder and succeeds; it ends by proving neither
//! single-instance unit is loaded, because instance 1 takes the ports the single server held.

use super::fleet_instances::FLEET_ROOT_UNDER_HOME;

/// The units of the single-instance server.
pub const SINGLE_INSTANCE_UNITS: [&str; 2] = ["tbd-reforger.service", "fleet-host-agent.service"];
/// The single host agent's files under `~/.config/fleet-host-agent/`.
pub const SINGLE_INSTANCE_AGENT_FILES: [&str; 3] =
    ["agent.toml", "machine-credential", "rcon-password"];

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
AGENT="$HOME/.config/fleet-host-agent"
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
for unit in @UNITS@; do
  if [ -e "$UNITS/$unit" ]; then retire+=("$UNITS/$unit"); fi
done
for file in @AGENT_FILES@; do
  if [ -e "$AGENT/$file" ]; then retire+=("$AGENT/$file"); fi
done
if [ -e "$PROFILE" ]; then retire+=("$PROFILE"); fi
if [ -e "$SERVER_CONFIG" ]; then retire+=("$SERVER_CONFIG"); fi
if [ "${#retire[@]}" -eq 0 ]; then
  echo "  nothing of the single-instance server is left to archive"
else
  ARCHIVE="$HOME/tbd/retired/single-instance-$(date -u +%Y%m%dT%H%M%SZ)"
  mkdir -p "$HOME/tbd/retired"
  mkdir "$ARCHIVE"
  for item in "${retire[@]}"; do
    mv "$item" "$ARCHIVE/"
    echo "  archived $item"
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
/// files, `profile_dir` and `server_config` into one timestamped folder under `~/tbd/retired/`.
pub fn migration_payload(profile_dir: &str, server_config: &str) -> String {
    MIGRATION
        .replace("@PROFILE@", profile_dir)
        .replace("@SERVER_CONFIG@", server_config)
        .replace("@FLEET@", FLEET_ROOT_UNDER_HOME)
        .replace("@UNITS@", &SINGLE_INSTANCE_UNITS.join(" "))
        .replace("@AGENT_FILES@", &SINGLE_INSTANCE_AGENT_FILES.join(" "))
}

#[cfg(test)]
#[path = "tests/legacy_single_instance_migration/tests.rs"]
mod tests;
