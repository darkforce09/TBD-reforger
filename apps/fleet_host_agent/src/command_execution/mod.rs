//! Commands as this host performs them: the re-validation of each claimed command's action and
//! arguments ([`HostCommand`], [`MissionDeployment`], [`ConsoleLine`], [`CommandRefusal`]), and
//! the executor that performs a validated command ([`FleetActionExecutor`], implemented for the
//! game host by [`HostActionExecutor`]; `mission_restart` performs `restart_with_mission`, and
//! [`ConsoleResponseCapture`] bounds the reply to a `console_command`).

mod command_refusal;
mod console_line;
mod console_response_capture;
mod host_action_executor;
mod host_command;
mod mission_deployment;
mod mission_restart;

pub use command_refusal::CommandRefusal;
pub use console_line::{CONSOLE_LINE_MAX_BYTES, ConsoleLine};
pub use console_response_capture::{CONSOLE_RESPONSE_MAX_BYTES, ConsoleResponseCapture};
pub use host_action_executor::{FleetActionExecutor, HostActionExecutor};
pub use host_command::HostCommand;
pub use mission_deployment::{MissionDeployment, ScenarioId};
