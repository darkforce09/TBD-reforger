//! The fleet waves W1–W14: the fleet servers they address, and the steps the fleet plan runs.
//!
//! **Role:** declares the wave modules, the [`WaveTargets`] every wave reads (database
//! container, fleet folder, the five servers, the operator's account, the relay's instance) and
//! [`steps`], the waves in the order they run.
//!
//! **Position:** called by `fleet_procedure/mod.rs` for the plan; `operator_lists.rs` reads the
//! wave table and the plan's host actions; the steps run on `procedure_runner/`.
//!
//! **Signals & state:** none; values and the boxed judges of the steps.
//!
//! **Invariants:** W1–W8 are one step each, which the orchestrator performs in Server Control on
//! all five servers at once; W9–W14 drive one server each, over one to three steps; the fleet has
//! exactly five instances, a relay instance and an operator account, or no step is built; a
//! server's W1–W7 effects decide only that server's cases, and W8–W14 decide the fleet-wide ones.

pub(crate) mod console_waves;
pub(crate) mod credential_judges;
pub(crate) mod credential_waves;
pub(crate) mod deployment_waves;
pub(crate) mod identity_waves;
pub(crate) mod lost_acknowledgement_waves;
pub(crate) mod process_waves;
pub(crate) mod shared_probes;
pub(crate) mod single_server_probes;
pub(crate) mod wave_table;

use anyhow::{Context, Result, ensure};

use self::wave_table::WaveServer;
use super::fleet_cases::FLEET_SERVER_COUNT;
use crate::commands::staging::procedure_runner::step::Step;
use crate::commands::staging::staging_settings::StagingSettings;

/// One fleet server as the waves address it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FleetServer {
    /// The staging instance, 1 to 5.
    pub instance: u16,
    /// The registered name, `TBD Staging <instance>`.
    pub name: String,
    /// The game server unit, `tbd-reforger@<instance>.service`.
    pub unit: String,
    /// The host agent unit, `fleet_host_agent@<instance>.service`.
    pub host_agent_unit: String,
}

/// Where the waves read: the database container, the fleet folder and the five servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaveTargets {
    /// The staging Postgres container the committed queries run in.
    pub database_container: String,
    /// `<home>/tbd/fleet` on the host, the parent of every instance folder.
    pub fleet_root: String,
    /// The fleet servers in instance order.
    pub servers: Vec<FleetServer>,
    /// The operator's Discord id: the account W9 links.
    pub operator_discord_id: String,
    /// The instance whose host agent polls through the acknowledgement-dropping relay.
    pub relay_instance: u16,
}

impl WaveTargets {
    /// The targets of `settings`; the fleet must have exactly [`FLEET_SERVER_COUNT`] instances,
    /// a relay instance and an operator account.
    pub(crate) fn from_settings(settings: &StagingSettings) -> Result<Self> {
        let instances = settings.fleet.instances();
        ensure!(
            instances.len() == usize::from(FLEET_SERVER_COUNT),
            "the fleet procedure drives {FLEET_SERVER_COUNT} servers, but deploy.env declares {} \
             (TBD_FLEET_INSTANCES)",
            instances.len()
        );
        Ok(Self {
            database_container: settings.database_container.clone(),
            fleet_root: settings.fleet_root(),
            servers: instances
                .iter()
                .map(|instance| FleetServer {
                    instance: instance.number,
                    name: instance.server_name(),
                    unit: instance.game_server_unit(),
                    host_agent_unit: instance.host_agent_unit(),
                })
                .collect(),
            operator_discord_id: settings.operator()?.to_string(),
            relay_instance: settings
                .relay_unit()
                .map(|(instance, _)| instance)
                .context(
                    "the lost-acknowledgement waves need the relay instance, which deploy.env \
                     does not set (TBD_FLEET_RELAY_INSTANCE)",
                )?,
        })
    }

    /// The fleet server `which` names.
    pub(crate) fn server(&self, which: WaveServer) -> Result<&FleetServer> {
        let instance = match which {
            WaveServer::First => 1,
            WaveServer::Second => 2,
            WaveServer::Relay => self.relay_instance,
        };
        self.servers
            .iter()
            .find(|server| server.instance == instance)
            .with_context(|| format!("the fleet has no instance {instance}"))
    }

    /// The servers as one phrase: `TBD Staging 1, 2, 3, 4 and 5`.
    pub(crate) fn server_list(&self) -> String {
        let numbers: Vec<String> = self
            .servers
            .iter()
            .map(|server| server.instance.to_string())
            .collect();
        match numbers.split_last() {
            None => "no server".to_string(),
            Some((last, [])) => format!("TBD Staging {last}"),
            Some((last, rest)) => format!("TBD Staging {} and {last}", rest.join(", ")),
        }
    }
}

/// The steps of W1–W14, in the order they run; the rotations' host actions come from
/// `settings`.
pub(crate) fn steps(targets: &WaveTargets, settings: &StagingSettings) -> Result<Vec<Step>> {
    let mut steps = process_waves::steps(targets)?;
    steps.extend(console_waves::steps(targets)?);
    steps.extend(deployment_waves::steps(targets)?);
    steps.extend(identity_waves::steps(targets)?);
    steps.extend(credential_waves::steps(targets, settings)?);
    steps.extend(lost_acknowledgement_waves::steps(targets)?);
    Ok(steps)
}
