//! `--migrate-host-agent-name`: move a host's game server host agents from the retired
//! `fleet_host_agent` names to the `game_server_host_agent` names the fleet installs.
//!
//! **Role:** the decision table of one renamed path ([`rename_decision`]); the migration script
//! ([`migration_payload`]), which stops and disables every `fleet_host_agent@N.service`, moves the
//! binary and the configuration folder to their current names, removes the retired unit template,
//! installs the current one and enables and starts `game_server_host_agent@N.service` for the same
//! N; the check that refuses a deploy while a retired name is left
//! ([`retired_names_absent_payload`]); and the dry-run lines of both.
//!
//! **Position:** called by the deploy pipeline in `super::remote`: with the flag, the migration
//! runs alone over ssh (no rsync, no deploy) and the operator deploys afterwards; without it, the
//! check runs before the rsync. The current names come from `super::host_agent`, the unit template
//! from `super::fleet_units`.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** the retired spellings live in this module alone, as the host holds them; every
//! refusal ([`RenameDecision::Refuse`]: both names of the configuration folder, or of the binary)
//! exits 3 before anything on the host changes; the binary and the folder are moved with `mv`,
//! which keeps every file, its mode and its owner, and the move never overwrites; the only removal
//! is the retired unit template, a copy of a tracked file, after every earlier step held; a host
//! that carries no retired name is left untouched, so a second run is a no-op; the started units
//! are read back and one that is not active fails the run.
//!
//! The binary is moved rather than left for the next deploy: the current units start before that
//! deploy runs, and they run whatever sits at the current binary path. The next deploy then builds
//! and installs a fresh `game_server_host_agent` over it. The single-instance migration of
//! `super::legacy_single_instance_migration` is independent: it retires the kebab-case single
//! install, which carries none of the names this module moves.

use super::fleet_instances::MAXIMUM_FLEET_INSTANCES;
use super::fleet_units::HOST_AGENT_TEMPLATE;
use super::host_agent::{
    HOST_AGENT_BINARY_UNDER_HOME, HOST_AGENT_CONFIGURATION_UNDER_HOME, HOST_AGENT_PACKAGE,
    HOST_AGENT_UNIT_TEMPLATE,
};

/// The retired name of the host agent's package, binary, configuration folder and units.
pub(super) const RETIRED_HOST_AGENT_NAME: &str = "fleet_host_agent";
/// The retired unit template's file name.
pub(super) const RETIRED_UNIT_TEMPLATE: &str = "fleet_host_agent@.service";
/// The retired binary, relative to the deploy user's home.
pub(super) const RETIRED_BINARY_UNDER_HOME: &str = ".local/bin/fleet_host_agent";
/// The retired configuration folder, relative to the deploy user's home.
pub(super) const RETIRED_CONFIGURATION_UNDER_HOME: &str = ".config/fleet_host_agent";

/// The exit status of a refusal, distinct from a failed step's.
pub(super) const REFUSED: i32 = 3;

/// What the migration does with one renamed path, from which of its two names exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenameDecision {
    /// Only the retired name exists: move it to the current name.
    Move,
    /// Only the current name exists: an earlier run or a fresh install put it there.
    AlreadyMigrated,
    /// Neither name exists: nothing to move.
    NothingToMove,
    /// Both names exist: refuse before any change, because only the operator can tell which copy
    /// the running agents use.
    Refuse,
}

/// The decision for one path whose retired and current names are present or not.
pub(super) fn rename_decision(retired_present: bool, current_present: bool) -> RenameDecision {
    match (retired_present, current_present) {
        (true, false) => RenameDecision::Move,
        (false, true) => RenameDecision::AlreadyMigrated,
        (false, false) => RenameDecision::NothingToMove,
        (true, true) => RenameDecision::Refuse,
    }
}

/// The four states of a path, in the order the script's `case` lists them.
const PRESENCE_STATES: [(bool, bool); 4] =
    [(true, false), (false, true), (false, false), (true, true)];

/// One path the migration renames, as the script names it.
struct RenamedPath {
    /// What the operator reads, such as `configuration folder`.
    noun: &'static str,
    /// The shell variable that records whether the path moves (`yes` or `no`).
    move_variable: &'static str,
    retired_variable: &'static str,
    current_variable: &'static str,
    retired_under_home: &'static str,
    current_under_home: &'static str,
}

/// The `case` that applies [`rename_decision`] to `path` on the host: one arm per state of
/// [`PRESENCE_STATES`].
fn decision_case(path: &RenamedPath) -> String {
    let retired = format!("~/{}", path.retired_under_home);
    let current = format!("~/{}", path.current_under_home);
    let mut case = format!(
        "case \"$(present \"${}\"):$(present \"${}\")\" in\n",
        path.retired_variable, path.current_variable
    );
    for (retired_present, current_present) in PRESENCE_STATES {
        let state = format!(
            "{}:{}",
            if retired_present { "yes" } else { "no" },
            if current_present { "yes" } else { "no" }
        );
        let body = match rename_decision(retired_present, current_present) {
            RenameDecision::Move => format!("{}=yes", path.move_variable),
            RenameDecision::AlreadyMigrated => format!(
                "{}=no; echo \"  the {} is already at {current}\"",
                path.move_variable, path.noun
            ),
            RenameDecision::NothingToMove => format!("{}=no", path.move_variable),
            RenameDecision::Refuse => format!(
                "refuse \"both {retired} and {current} exist; keep the {} the running agents use, \
                 move the other out of the way\"",
                path.noun
            ),
        };
        case.push_str(&format!("  {state}) {body} ;;\n"));
    }
    case.push_str("esac\n");
    case
}

const CONFIGURATION_FOLDER: RenamedPath = RenamedPath {
    noun: "configuration folder",
    move_variable: "move_configuration",
    retired_variable: "RETIRED_CONFIGURATION",
    current_variable: "CURRENT_CONFIGURATION",
    retired_under_home: RETIRED_CONFIGURATION_UNDER_HOME,
    current_under_home: HOST_AGENT_CONFIGURATION_UNDER_HOME,
};

const BINARY: RenamedPath = RenamedPath {
    noun: "binary",
    move_variable: "move_binary",
    retired_variable: "RETIRED_BINARY",
    current_variable: "CURRENT_BINARY",
    retired_under_home: RETIRED_BINARY_UNDER_HOME,
    current_under_home: HOST_AGENT_BINARY_UNDER_HOME,
};

const PREAMBLE: &str = r#"set -euo pipefail
RETIRED_BINARY="$HOME/@RETIRED_BINARY@"
CURRENT_BINARY="$HOME/@CURRENT_BINARY@"
RETIRED_CONFIGURATION="$HOME/@RETIRED_CONFIGURATION@"
CURRENT_CONFIGURATION="$HOME/@CURRENT_CONFIGURATION@"
UNITS="$HOME/.config/systemd/user"
RETIRED_TEMPLATE="$UNITS/@RETIRED_TEMPLATE@"
CURRENT_TEMPLATE="$UNITS/@CURRENT_TEMPLATE@"
present() { if [ -e "$1" ] || [ -L "$1" ]; then echo yes; else echo no; fi; }
refuse() {
  echo "REFUSED: $1." >&2
  echo "  Nothing on the host was changed; decide which copy stays, then rerun --migrate-host-agent-name." >&2
  exit @REFUSED@
}
"#;

const MIGRATION: &str = r#"if ! systemctl --user show-environment >/dev/null 2>&1; then
  echo "FAIL: the deploy user's systemd manager does not answer systemctl --user; nothing on the host was changed." >&2
  exit 1
fi
retired_instances=""
for n in @INSTANCES@; do
  unit="@RETIRED@@$n.service"
  enabled="$(systemctl --user is-enabled "$unit" 2>/dev/null || true)"
  active="$(systemctl --user is-active "$unit" 2>/dev/null || true)"
  if [ "$enabled" = enabled ] || [ "$active" = active ] || [ "$active" = activating ]; then
    retired_instances="$retired_instances $n"
  fi
done
if [ "$move_configuration" = no ] && [ "$move_binary" = no ] && [ -z "$retired_instances" ] && [ "$(present "$RETIRED_TEMPLATE")" = no ]; then
  echo "  nothing to migrate: the host carries no @RETIRED@ name"
  exit 0
fi
trap 'echo "FAIL: the host agent name migration stopped part way (instances whose retired unit was running:${retired_instances:- none}); fix the error above, then rerun --migrate-host-agent-name." >&2' ERR
for n in $retired_instances; do
  systemctl --user disable --now "@RETIRED@@$n.service"
  echo "  stopped and disabled @RETIRED@@$n.service"
done
if [ "$move_binary" = yes ]; then
  mv -T "$RETIRED_BINARY" "$CURRENT_BINARY"
  echo "  moved ~/@RETIRED_BINARY@ to ~/@CURRENT_BINARY@"
fi
if [ "$move_configuration" = yes ]; then
  mv -T "$RETIRED_CONFIGURATION" "$CURRENT_CONFIGURATION"
  echo "  moved ~/@RETIRED_CONFIGURATION@/ to ~/@CURRENT_CONFIGURATION@/"
fi
if [ "$(present "$RETIRED_TEMPLATE")" = yes ]; then
  rm -f -- "$RETIRED_TEMPLATE"
  echo "  removed $RETIRED_TEMPLATE"
fi
mkdir -p "$UNITS"
cat > "$CURRENT_TEMPLATE" <<'UNITEOF'
@TEMPLATE@UNITEOF
echo "  installed $CURRENT_TEMPLATE"
systemctl --user daemon-reload
for n in $retired_instances; do
  systemctl --user enable --now "@CURRENT@@$n.service"
  echo "  enabled and started @CURRENT@@$n.service"
done
trap - ERR
if [ -n "$retired_instances" ]; then sleep 3; fi
failed=0
for n in $retired_instances; do
  unit="@CURRENT@@$n.service"
  state="$(systemctl --user is-active "$unit" 2>/dev/null || true)"
  if [ "$state" = active ]; then
    echo "  $unit active"
  else
    echo "FAIL: $unit is '$state', not active." >&2
    journalctl --user -u "$unit" -n 20 --no-pager >&2 || true
    failed=1
  fi
done
exit "$failed"
"#;

/// The script `--migrate-host-agent-name` sends to the host through `ssh <host> bash -s`.
pub(super) fn migration_payload() -> String {
    let instances = (1..=MAXIMUM_FLEET_INSTANCES)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let preamble = PREAMBLE
        .replace("@RETIRED_BINARY@", RETIRED_BINARY_UNDER_HOME)
        .replace("@CURRENT_BINARY@", HOST_AGENT_BINARY_UNDER_HOME)
        .replace("@RETIRED_CONFIGURATION@", RETIRED_CONFIGURATION_UNDER_HOME)
        .replace(
            "@CURRENT_CONFIGURATION@",
            HOST_AGENT_CONFIGURATION_UNDER_HOME,
        )
        .replace("@RETIRED_TEMPLATE@", RETIRED_UNIT_TEMPLATE)
        .replace("@CURRENT_TEMPLATE@", HOST_AGENT_UNIT_TEMPLATE)
        .replace("@REFUSED@", &REFUSED.to_string());
    let migration = MIGRATION
        .replace("@INSTANCES@", &instances)
        .replace("@RETIRED_BINARY@", RETIRED_BINARY_UNDER_HOME)
        .replace("@CURRENT_BINARY@", HOST_AGENT_BINARY_UNDER_HOME)
        .replace("@RETIRED_CONFIGURATION@", RETIRED_CONFIGURATION_UNDER_HOME)
        .replace(
            "@CURRENT_CONFIGURATION@",
            HOST_AGENT_CONFIGURATION_UNDER_HOME,
        )
        .replace("@RETIRED@", RETIRED_HOST_AGENT_NAME)
        .replace("@CURRENT@", HOST_AGENT_PACKAGE)
        .replace("@TEMPLATE@", HOST_AGENT_TEMPLATE);
    format!(
        "{preamble}{}{}{migration}",
        decision_case(&CONFIGURATION_FOLDER),
        decision_case(&BINARY)
    )
}

/// The dry-run lines of the migration: what it does, then the exact script it sends.
pub(super) fn migration_plan_lines(host: &str) -> Vec<String> {
    let mut lines = vec![
        format!(
            "[dry-run] migrate the host agent names: stop and disable every \
             {RETIRED_HOST_AGENT_NAME}@N.service; move ~/{RETIRED_BINARY_UNDER_HOME} to \
             ~/{HOST_AGENT_BINARY_UNDER_HOME} and ~/{RETIRED_CONFIGURATION_UNDER_HOME}/ to \
             ~/{HOST_AGENT_CONFIGURATION_UNDER_HOME}/; remove {RETIRED_UNIT_TEMPLATE}; install \
             {HOST_AGENT_UNIT_TEMPLATE}; enable and start {HOST_AGENT_PACKAGE}@N.service for the \
             same N; refuse with nothing changed while both names of the folder or of the binary \
             exist; no rsync and no deploy"
        ),
        format!("[dry-run] ssh {host} bash -s, with this script on stdin:"),
    ];
    lines.extend(migration_payload().lines().map(str::to_string));
    lines
}

const RETIRED_NAMES_ABSENT: &str = r#"set -uo pipefail
left=""
for path in "$HOME/.config/systemd/user/@RETIRED_TEMPLATE@" "$HOME/@RETIRED_CONFIGURATION@" "$HOME/@RETIRED_BINARY@"; do
  if [ -e "$path" ] || [ -L "$path" ]; then
    left="$left $path"
  fi
done
if [ -n "$left" ]; then
  echo "FAIL: the host still carries the @RETIRED@ names:$left. Run cargo xtask deploy staging --migrate-host-agent-name first, then deploy." >&2
  exit 1
fi
echo "  no @RETIRED@ name is left on the host"
"#;

/// Exit 1, naming them, while the host still holds the retired unit template, configuration
/// folder or binary: a deploy then would start a second agent beside each retired one.
pub(super) fn retired_names_absent_payload() -> String {
    RETIRED_NAMES_ABSENT
        .replace("@RETIRED_TEMPLATE@", RETIRED_UNIT_TEMPLATE)
        .replace("@RETIRED_CONFIGURATION@", RETIRED_CONFIGURATION_UNDER_HOME)
        .replace("@RETIRED_BINARY@", RETIRED_BINARY_UNDER_HOME)
        .replace("@RETIRED@", RETIRED_HOST_AGENT_NAME)
}

/// The dry-run line of the check a deploy runs without the flag.
pub(super) fn retired_names_absent_plan_line() -> String {
    format!(
        "[dry-run] check on the host: no {RETIRED_HOST_AGENT_NAME} name is left \
         (~/.config/systemd/user/{RETIRED_UNIT_TEMPLATE}, ~/{RETIRED_CONFIGURATION_UNDER_HOME}/, \
         ~/{RETIRED_BINARY_UNDER_HOME}); the deploy stops while one is, and \
         --migrate-host-agent-name moves them"
    )
}

#[cfg(test)]
#[path = "tests/host_agent_name_migration/tests.rs"]
mod tests;
