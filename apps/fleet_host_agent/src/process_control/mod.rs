//! Game-server process control through the systemd user manager, without a shell.
//!
//! Start, stop and restart run `systemctl --user <verb> <unit>`. The verdict is then read back
//! from systemd with `systemctl --user show --property=LoadState --value <unit>` and
//! `systemctl --user show --property=ActiveState --value <unit>`. Every invocation is a fixed
//! argument vector whose only variable element is the unit name validated at startup, run with
//! a cleared environment (`systemctl_runner`).
//!
//! The exit status of the verb does not decide the verdict: an Arma Reforger server that fails
//! to start exits with status 0 a few seconds after systemd reported the start as done. After
//! start and restart the agent therefore waits out a dwell and then reads the unit's state. An
//! action succeeds only when the unit is loaded and in the state the action intends: active
//! after start and restart, inactive after stop. `systemctl show` reports a unit that does not
//! exist as inactive, which is why the LoadState is read as well (`unit_state_verdict`).

mod systemctl_runner;
mod unit_state_verdict;

use std::path::PathBuf;
use std::time::Duration;

use tracing::info;

use systemctl_runner::run_systemctl;
pub use unit_state_verdict::{ObservedUnitState, ProcessActionReport, SystemctlObservation};

/// Longest systemd unit name.
const UNIT_NAME_MAX_BYTES: usize = 255;
/// systemd state values are short lowercase words such as `active` or `not-found`.
const STATE_VALUE_MAX_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAction {
    Start,
    Stop,
    Restart,
}

impl ProcessAction {
    pub fn systemctl_verb(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }

    /// The ActiveState that means the action worked.
    pub fn intended_active_state(self) -> &'static str {
        match self {
            Self::Start | Self::Restart => "active",
            Self::Stop => "inactive",
        }
    }

    /// Start and restart can report success over a server that dies seconds later, so their
    /// state is read after the dwell. `systemctl stop` returns once the unit has stopped.
    pub fn reads_state_after_dwell(self) -> bool {
        matches!(self, Self::Start | Self::Restart)
    }
}

/// A systemd service unit name: ASCII letters, digits and `:_.@\-`, ending in `.service`, at
/// most 255 bytes, never starting with `-` or `.`, so it cannot read as an option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemdUnitName(String);

impl SystemdUnitName {
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        if raw.is_empty() || raw.len() > UNIT_NAME_MAX_BYTES {
            return Err("must be 1 to 255 bytes long");
        }
        if raw.starts_with(['-', '.']) {
            return Err("must not start with '-' or '.'");
        }
        if !raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":_.@\\-".contains(&byte))
        {
            return Err("may contain only ASCII letters, digits and the characters :_.@\\-");
        }
        match raw.strip_suffix(".service") {
            Some(prefix) if !prefix.is_empty() => Ok(Self(raw.to_owned())),
            _ => Err("must name a .service unit"),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ProcessControlSettings {
    /// Absolute path of the systemctl program.
    pub systemctl_program: PathBuf,
    /// The game server's systemd user unit.
    pub unit: SystemdUnitName,
    /// The wait after start and restart before the unit's state is read.
    pub start_dwell: Duration,
    /// The limit on one start, stop or restart invocation.
    pub verb_timeout: Duration,
    /// The limit on one state read.
    pub state_read_timeout: Duration,
}

impl ProcessControlSettings {
    /// Longer than systemd's default 90 s stop timeout, so a stop that ends in SIGKILL still
    /// returns. With the dwell (at most 30 s) and two state reads, start and restart stay inside
    /// the ledger's 180 s execution window and stop inside its 120 s window.
    pub const VERB_TIMEOUT: Duration = Duration::from_secs(100);
    pub const STATE_READ_TIMEOUT: Duration = Duration::from_secs(5);
}

/// A property read back with `systemctl --user show --property=<name> --value <unit>`.
#[derive(Debug, Clone, Copy)]
enum UnitProperty {
    LoadState,
    ActiveState,
}

impl UnitProperty {
    fn argument(self) -> &'static str {
        match self {
            Self::LoadState => "--property=LoadState",
            Self::ActiveState => "--property=ActiveState",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessControl {
    settings: ProcessControlSettings,
}

impl ProcessControl {
    pub fn new(settings: ProcessControlSettings) -> Self {
        Self { settings }
    }

    pub fn unit(&self) -> &SystemdUnitName {
        &self.settings.unit
    }

    /// Runs the action's verb, waits out the dwell where the action has one, and reads the
    /// unit's state back.
    pub async fn perform(&self, action: ProcessAction) -> ProcessActionReport {
        let unit = self.settings.unit.as_str();
        let verb = action.systemctl_verb();
        info!(unit, verb, "running systemctl --user {verb}");
        let systemctl = SystemctlObservation::of(
            run_systemctl(
                &self.settings.systemctl_program,
                &["--user", verb, unit],
                self.settings.verb_timeout,
            )
            .await,
        );
        let dwell = if action.reads_state_after_dwell() {
            self.settings.start_dwell
        } else {
            Duration::ZERO
        };
        tokio::time::sleep(dwell).await;
        let unit_state = self.read_unit_state().await;
        info!(unit, verb, ?unit_state, "read the unit state back");
        ProcessActionReport {
            action,
            unit: unit.to_owned(),
            systemctl,
            dwell,
            unit_state,
        }
    }

    async fn read_unit_state(&self) -> Result<ObservedUnitState, String> {
        Ok(ObservedUnitState {
            load_state: self.read_property(UnitProperty::LoadState).await?,
            active_state: self.read_property(UnitProperty::ActiveState).await?,
        })
    }

    async fn read_property(&self, property: UnitProperty) -> Result<String, String> {
        let unit = self.settings.unit.as_str();
        let arguments = ["--user", "show", property.argument(), "--value", unit];
        let invocation = format!("systemctl --user show {}", property.argument());
        let output = run_systemctl(
            &self.settings.systemctl_program,
            &arguments,
            self.settings.state_read_timeout,
        )
        .await
        .map_err(|failure| format!("{invocation} {failure}"))?;
        if !output.status.success() {
            return Err(format!(
                "{invocation} {}",
                SystemctlObservation::of(Ok(output)).describe()
            ));
        }
        state_value(&output.stdout)
            .ok_or_else(|| format!("{invocation} printed no recognisable value"))
    }
}

/// The single lowercase word `systemctl show --value` prints for a state property.
fn state_value(stdout: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(stdout).ok()?.trim();
    let recognisable = !value.is_empty()
        && value.len() <= STATE_VALUE_MAX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-');
    recognisable.then(|| value.to_owned())
}

#[cfg(test)]
#[path = "tests/process_control.rs"]
mod tests;
