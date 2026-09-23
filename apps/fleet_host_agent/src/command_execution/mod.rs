//! Commands as this host performs them: the re-validation of each claimed command's action and
//! arguments ([`HostCommand`], [`MissionDeployment`], [`CommandRefusal`]), and the executor that
//! performs a validated command ([`FleetActionExecutor`], implemented for the game host by
//! [`HostActionExecutor`]; `mission_restart` performs `restart_with_mission`).

mod command_refusal;
mod host_action_executor;
mod host_command;
mod mission_deployment;
mod mission_restart;

pub use command_refusal::CommandRefusal;
pub use host_action_executor::{FleetActionExecutor, HostActionExecutor};
pub use host_command::HostCommand;
pub use mission_deployment::{MissionDeployment, ScenarioId};
