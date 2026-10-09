//! The committed inputs of the load procedure: the workload the engine drives and the population
//! the host tool seeds, and the digest that binds a receipt to both.
//!
//! **Role:** reads and decodes `tools/xtask/staging/load_workload.json` (a
//! [`WorkloadPlan`]) and `load_population.json` (a [`LoadPopulation`]), checks that they agree
//! with each other and with the reserved Discord id range, and computes `workload_sha256`.
//!
//! **Position:** read by the load procedure's plan, run, manifest and observations, by
//! `seed-load` and by the local rehearsal.
//!
//! **Signals & state:** none; pure values read once per command.
//!
//! **Invariants:** both documents refuse unknown fields; the population has exactly
//! `clients × accounts_per_client` accounts inside the reserved range, and every fixture event
//! has a slot for each account that writes to it; `workload_sha256` is the SHA-256 of the two
//! files' bytes, each preceded by its length as an 8-byte big-endian integer, workload first.

use std::path::Path;

use crate::error::{Result, ResultExt, ensure};
use content_digest::Sha256Hasher;
use serde::{Deserialize, Serialize};
use staging_load_plan::WorkloadPlan;

/// The committed workload, relative to the repository root.
pub(crate) const WORKLOAD_FILE: &str = repository_layout::tool_inputs::STAGING_LOAD_WORKLOAD;
/// The committed population, relative to the repository root.
pub(crate) const POPULATION_FILE: &str = repository_layout::tool_inputs::STAGING_LOAD_POPULATION;
/// The first Discord id reserved for synthetic staging accounts.
pub(crate) const FIRST_RESERVED_DISCORD_ID: u64 = 9_100_000_000_000_000_000;
/// The last Discord id reserved for synthetic staging accounts.
pub(crate) const LAST_RESERVED_DISCORD_ID: u64 = 9_100_000_000_000_099_999;

/// The synthetic members the host tool seeds and the fixture events their writes land on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoadPopulation {
    /// The Discord id of account 0, as decimal text.
    pub id_base: String,
    /// Accounts, numbered from the id base.
    pub accounts: u32,
    /// The Discord role name every account holds.
    pub discord_role: String,
    /// The `[Load fixture]` events.
    pub fixture_events: FixtureEventPlan,
}

/// The fixture events `staging-fixtures seed-load-fixture-events` creates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureEventPlan {
    /// Events, titled `<title prefix> 01` onwards.
    pub count: u32,
    /// The title every fixture event starts with.
    pub title_prefix: String,
    /// Factions of each event's ORBAT.
    pub factions: u32,
    /// Squads per faction.
    pub squads_per_faction: u32,
    /// Slots per squad.
    pub slots_per_squad: u32,
    /// Days between the seeding and the first event's start.
    pub start_offset_days: u32,
    /// The title of the live mission every event attaches.
    pub mission_title: String,
}

impl FixtureEventPlan {
    /// Slots of one event's ORBAT.
    pub(crate) fn slots_per_event(&self) -> u32 {
        self.factions * self.squads_per_faction * self.slots_per_squad
    }

    /// The title of fixture event `number` (from 1): `<prefix> 01`.
    pub(crate) fn title(&self, number: u32) -> String {
        format!("{} {number:02}", self.title_prefix)
    }
}

/// Both committed documents, decoded and checked, with their digest.
#[derive(Debug, Clone)]
pub(crate) struct CommittedLoadData {
    pub workload: WorkloadPlan,
    pub population: LoadPopulation,
    /// The receipt's `workload_sha256`.
    pub workload_sha256: String,
}

impl CommittedLoadData {
    /// Reads both files under `root`.
    pub(crate) fn read(root: &Path) -> Result<Self> {
        let workload = std::fs::read(root.join(WORKLOAD_FILE))
            .with_context(|| format!("reading {WORKLOAD_FILE}"))?;
        let population = std::fs::read(root.join(POPULATION_FILE))
            .with_context(|| format!("reading {POPULATION_FILE}"))?;
        Self::from_documents(&workload, &population)
    }

    /// Decodes and checks the two documents' bytes.
    pub(crate) fn from_documents(workload: &[u8], population: &[u8]) -> Result<Self> {
        let workload_plan = WorkloadPlan::from_json_str(
            std::str::from_utf8(workload).context("the load workload is not UTF-8")?,
        )?;
        workload_plan
            .validate()
            .context("the committed load workload is refused")?;
        let population_plan: LoadPopulation =
            serde_json::from_slice(population).context("decoding the load population")?;
        let data = Self {
            workload: workload_plan,
            population: population_plan,
            workload_sha256: workload_digest(workload, population),
        };
        data.check_agreement()?;
        Ok(data)
    }

    /// The Discord id of account 0.
    pub(crate) fn id_base(&self) -> Result<u64> {
        self.population
            .id_base
            .parse()
            .with_context(|| format!("id_base {:?} is not a Discord id", self.population.id_base))
    }

    /// The Discord id of the last account.
    pub(crate) fn last_account_id(&self) -> Result<u64> {
        Ok(self.id_base()? + u64::from(self.population.accounts) - 1)
    }

    fn check_agreement(&self) -> Result<()> {
        let population = &self.population;
        let events = &population.fixture_events;
        let base = self.id_base()?;
        ensure!(population.accounts >= 1, "the population has no account");
        ensure!(
            base >= FIRST_RESERVED_DISCORD_ID
                && base + u64::from(population.accounts) - 1 <= LAST_RESERVED_DISCORD_ID,
            "the population's ids leave the reserved range {FIRST_RESERVED_DISCORD_ID}..={LAST_RESERVED_DISCORD_ID}"
        );
        let expected =
            u64::from(self.workload.clients) * u64::from(self.workload.accounts_per_client);
        ensure!(
            u64::from(population.accounts) == expected,
            "the population holds {} accounts; the workload's clients × accounts per client is {expected}",
            population.accounts
        );
        ensure!(
            events.count >= 1 && !events.title_prefix.trim().is_empty(),
            "the fixture events need a count and a title prefix"
        );
        ensure!(
            population.accounts.div_ceil(events.count) <= events.slots_per_event(),
            "{} accounts over {} events need more than the {} slots of one event",
            population.accounts,
            events.count,
            events.slots_per_event()
        );
        ensure!(
            !population.discord_role.trim().is_empty() && !events.mission_title.trim().is_empty(),
            "the population needs a Discord role and the events a mission title"
        );
        Ok(())
    }
}

/// SHA-256 over each document's 8-byte big-endian length and bytes, workload first.
pub(crate) fn workload_digest(workload: &[u8], population: &[u8]) -> String {
    let mut hasher = Sha256Hasher::new();
    for document in [workload, population] {
        hasher.update((document.len() as u64).to_be_bytes());
        hasher.update(document);
    }
    hasher.finalize_hex()
}
