//! The load run's inputs: the committed workload and the bindings of one staging run.
//!
//! - **Role:** declares [`WorkloadPlan`], the shape of the committed load workload (decoded with
//!   `deny_unknown_fields`), and [`LoadRunPlan`], the workload plus the target origin, the source
//!   addresses, the account file and the fixture events of one run; checks both before any request
//!   leaves the machine.
//! - **Position:** the xtask load procedure reads the committed workload and fills in the run
//!   bindings; the load generator's `run` consumes the plan through `checked_settings` and
//!   `normalized_origin`.
//! - **Signals & state:** none; plain data and pure checks.
//! - **Invariants:**
//!   - Every duration and rate is finite and above zero, and the jitter stays below half a
//!     period, so a client's paced slots never reorder.
//!   - The ramp is at least [`WorkloadPlan::minimum_ramp_seconds`]: the clients' sign-ins keep
//!     every source address at or under 80 % of its auth ceiling.
//!   - The target is a plain-HTTP origin without a path; the source addresses are distinct unicast
//!     addresses, exactly as many as the workload declares.
//!   - Every fixture event has a slot for each account that writes to it: account `k` takes event
//!     `k mod events` and slot `k div events`.

use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, refuse_unless};
use crate::identifiers::{EventId, EventMissionId, MissionId, SlotId, TemplateId};
use crate::request_catalog::{RequestCatalog, is_path_safe};
use crate::run_settings::{RunSettings, checked_ceiling, seconds};

/// The share of each address's auth ceiling the clients' sign-ins may take during the ramp.
const SIGN_IN_SHARE_OF_AUTH_CEILING: f64 = 0.8;

/// The committed workload: pacing, ceilings and the weighted request mix of the member load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadPlan {
    /// Seed of every random draw of a run: phases, jitter and template picks.
    pub seed: u64,
    /// Seconds over which the clients sign in, evenly spaced; nothing in the ramp is measured.
    /// At least [`WorkloadPlan::minimum_ramp_seconds`].
    pub ramp_seconds: f64,
    /// Seconds of measurement after the ramp.
    pub measured_seconds: f64,
    /// Virtual clients.
    pub clients: u32,
    /// Source addresses the clients spread over: client `c` sends from address `c mod count`.
    pub source_address_count: u32,
    /// Paced rate over all clients, requests per second: each client fires every
    /// `clients / requests_per_second` seconds.
    pub requests_per_second: f64,
    /// Accounts each client holds; the account file holds `clients × accounts_per_client`.
    pub accounts_per_client: u32,
    /// Seconds a client stays on one account before it refreshes into its next one.
    pub account_hold_seconds: f64,
    /// Jitter of every paced slot as a fraction of the period, both ways (0.05 is ±5 %).
    pub jitter_fraction: f64,
    /// Seconds after which a request whose body has not ended is a timeout.
    pub request_timeout_seconds: f64,
    /// Width of the concurrency census windows, seconds.
    pub census_window_seconds: f64,
    /// Ceilings every source address is held under.
    pub per_address_ceilings: PerAddressCeilings,
    /// The weighted request mix. The refresh is not part of it: the account rotation issues it.
    pub request_mix: Vec<RequestTemplate>,
}

/// The two ceilings of one source address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerAddressCeilings {
    /// Every request from the address, refreshes included.
    pub all_requests: WindowCeiling,
    /// Requests under the API's authentication routes, such as the refresh.
    pub auth_requests: WindowCeiling,
}

/// At most `max_requests` request starts inside any window of `window_seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowCeiling {
    /// Requests allowed inside one window.
    pub max_requests: u32,
    /// Length of the sliding window, seconds.
    pub window_seconds: f64,
}

impl WindowCeiling {
    /// The ceiling as a sustained rate, requests per second.
    pub fn requests_per_second(&self) -> f64 {
        f64::from(self.max_requests) / self.window_seconds
    }
}

/// What a request is for, which decides the latency sample it joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestClass {
    /// A GET answered with JSON, or with 304 where its step expects it.
    JsonRead,
    /// A POST or DELETE that changes the account's own state.
    JsonWrite,
    /// The refresh that switches a client into its next account.
    Session,
}

/// The HTTP methods the member load uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `DELETE`.
    Delete,
}

/// One weighted entry of the mix: a cycle of steps each account walks, one step per pick.
///
/// A plain read or save has one step. A toggle, such as a bookmark added then removed, or a
/// registration then its withdrawal, has two, so every write an account sends is valid for the
/// state its previous write left.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestTemplate {
    /// Name of the template in the report: lowercase letters, digits and `_`.
    pub id: TemplateId,
    /// The latency sample the template's requests join.
    pub class: RequestClass,
    /// Relative weight of the template in the mix, at least 1.
    pub weight: u32,
    /// The steps, walked in order and wrapped per account.
    pub steps: Vec<RequestStep>,
}

/// One request of a template, before its placeholders are bound to an account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestStep {
    /// The method.
    pub method: HttpMethod,
    /// The path and query under the target origin, with `{placeholder}` fields.
    pub path: String,
    /// The JSON body, whose string values may hold `{placeholder}` fields; absent for none.
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    /// Every status that counts as expected; a 304 makes the step conditional.
    pub expected_statuses: Vec<u16>,
}

/// One run: the committed workload plus this run's target, addresses, accounts and fixture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadRunPlan {
    /// The committed workload.
    pub workload: WorkloadPlan,
    /// The API origin under load, such as `http://192.0.2.10:3080`: plain HTTP, no path.
    pub target_origin: String,
    /// Local addresses the clients send from; client `c` uses entry `c mod len`.
    pub source_addresses: Vec<IpAddr>,
    /// The account file the population seeding wrote, read once when the run starts.
    pub account_file: PathBuf,
    /// Fixture events in fixture order; account `k` writes to event `k mod len`, slot `k div len`.
    pub fixture_events: Vec<FixtureEvent>,
}

/// One seeded fixture event and the ids the writes of its accounts name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureEvent {
    /// The event.
    pub event_id: EventId,
    /// The event's active mission attachment, which registrations name.
    pub event_mission_id: EventMissionId,
    /// The attached mission, which bookmarks name.
    pub mission_id: MissionId,
    /// The attachment's slots in order; account `k` claims slot `k div events`.
    pub slot_ids: Vec<SlotId>,
}

impl WorkloadPlan {
    /// Decode a workload document, refusing unknown fields at every level.
    ///
    /// # Errors
    ///
    /// The document is not JSON of the workload's shape.
    pub fn from_json_str(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|error| Error::WorkloadUndecodable { error })
    }

    /// Read and decode a workload file.
    ///
    /// # Errors
    ///
    /// The file cannot be read or is not JSON of the workload's shape.
    pub fn from_json_file(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).map_err(|error| Error::WorkloadFileUnreadable {
                path: path.to_path_buf(),
                error,
            })?;
        serde_json::from_str(&text).map_err(|error| Error::WorkloadFileUndecodable {
            path: path.to_path_buf(),
            error,
        })
    }

    /// Check every number and every template of the workload.
    ///
    /// # Errors
    ///
    /// Names the first number out of range or the first template that does not compile.
    pub fn validate(&self) -> Result<()> {
        self.checked_settings()?;
        RequestCatalog::compile(&self.request_mix)?;
        Ok(())
    }

    /// The shortest ramp over which the clients sign in at or under 80 % of each address's auth
    /// ceiling: `clients / (source_address_count × auth ceiling × 0.8)` seconds, the ceiling taken
    /// as requests per second. A client joins the member load only once its sign-in has handed
    /// it a token, so a ramp any shorter makes the harness's own auth pacing hold clients out.
    pub fn minimum_ramp_seconds(&self) -> f64 {
        f64::from(self.clients)
            / (f64::from(self.source_address_count)
                * self
                    .per_address_ceilings
                    .auth_requests
                    .requests_per_second()
                * SIGN_IN_SHARE_OF_AUTH_CEILING)
    }

    /// The workload's numbers, checked and converted once.
    ///
    /// # Errors
    ///
    /// Names the first number out of range; see [`WorkloadPlan::validate`].
    pub fn checked_settings(&self) -> Result<RunSettings> {
        refuse_unless!(self.clients >= 1, "clients must be at least 1");
        refuse_unless!(
            (1..=self.clients).contains(&self.source_address_count),
            "source_address_count must be between 1 and clients ({})",
            self.clients
        );
        refuse_unless!(
            self.accounts_per_client >= 1,
            "accounts_per_client must be at least 1"
        );
        refuse_unless!(
            self.requests_per_second.is_finite() && self.requests_per_second > 0.0,
            "requests_per_second must be a finite number above 0"
        );
        refuse_unless!(
            self.jitter_fraction.is_finite() && (0.0..0.5).contains(&self.jitter_fraction),
            "jitter_fraction must be at least 0 and below 0.5"
        );
        let period = seconds(
            "the period (clients / requests_per_second)",
            f64::from(self.clients) / self.requests_per_second,
            false,
        )?;
        let measured = seconds("measured_seconds", self.measured_seconds, false)?;
        let census_window = seconds("census_window_seconds", self.census_window_seconds, false)?;
        refuse_unless!(
            census_window <= measured,
            "census_window_seconds must not exceed measured_seconds"
        );
        let accounts =
            usize::try_from(u64::from(self.clients) * u64::from(self.accounts_per_client))
                .map_err(|error| {
                    Error::refused(format!(
                        "clients × accounts_per_client does not fit this machine: {error}"
                    ))
                })?;
        let all_requests = checked_ceiling("all_requests", self.per_address_ceilings.all_requests)?;
        let auth_requests =
            checked_ceiling("auth_requests", self.per_address_ceilings.auth_requests)?;
        let ramp = seconds("ramp_seconds", self.ramp_seconds, true)?;
        self.check_sign_in_ramp()?;
        Ok(RunSettings {
            clients: self.clients,
            accounts,
            period,
            ramp,
            measured,
            hold: seconds("account_hold_seconds", self.account_hold_seconds, false)?,
            jitter_fraction: self.jitter_fraction,
            request_timeout: seconds(
                "request_timeout_seconds",
                self.request_timeout_seconds,
                false,
            )?,
            census_window,
            all_requests,
            auth_requests,
        })
    }

    /// Refuse a ramp shorter than [`WorkloadPlan::minimum_ramp_seconds`], naming the minimum
    /// rounded up to the millisecond, so the value it names is itself accepted.
    fn check_sign_in_ramp(&self) -> Result<()> {
        let minimum = self.minimum_ramp_seconds();
        refuse_unless!(
            self.ramp_seconds >= minimum,
            "ramp_seconds {} is too short: {} clients signing in from {} source addresses need a \
             ramp of at least {} s to keep each address at or under 80 % of its auth ceiling of \
             {} requests per second",
            self.ramp_seconds,
            self.clients,
            self.source_address_count,
            (minimum * 1000.0).ceil() / 1000.0,
            self.per_address_ceilings
                .auth_requests
                .requests_per_second()
        );
        Ok(())
    }
}

impl LoadRunPlan {
    /// Check the workload, its templates and every binding of the run.
    ///
    /// # Errors
    ///
    /// Names the first number, template, origin, address or fixture event that is refused.
    pub fn validate(&self) -> Result<()> {
        self.checked_settings()?;
        RequestCatalog::compile(&self.workload.request_mix)?;
        Ok(())
    }

    /// The workload's numbers, checked and converted once, after every binding of the run is
    /// checked.
    ///
    /// # Errors
    ///
    /// Names the first number, origin, address or fixture event that is refused; see
    /// [`LoadRunPlan::validate`].
    pub fn checked_settings(&self) -> Result<RunSettings> {
        let settings = self.workload.checked_settings()?;
        self.normalized_origin()?;
        refuse_unless!(
            self.source_addresses.len() == self.workload.source_address_count as usize,
            "the run names {} source addresses; the workload declares {}",
            self.source_addresses.len(),
            self.workload.source_address_count
        );
        let mut seen = HashSet::new();
        for address in &self.source_addresses {
            refuse_unless!(
                !address.is_unspecified() && !address.is_multicast(),
                "source address {address} is not a unicast address"
            );
            refuse_unless!(
                seen.insert(*address),
                "source address {address} is named twice"
            );
        }
        refuse_unless!(
            !self.fixture_events.is_empty(),
            "the run names no fixture event"
        );
        let rows = settings.accounts.div_ceil(self.fixture_events.len());
        for (index, event) in self.fixture_events.iter().enumerate() {
            for (field, value) in [
                ("event_id", event.event_id.as_str()),
                ("event_mission_id", event.event_mission_id.as_str()),
                ("mission_id", event.mission_id.as_str()),
            ] {
                refuse_unless!(
                    is_path_safe(value),
                    "fixture event {index}: {field} {value:?} is not an id of ASCII letters, digits, '-' or '_'"
                );
            }
            refuse_unless!(
                event.slot_ids.len() >= rows,
                "fixture event {index} has {} slots; {rows} accounts write to it",
                event.slot_ids.len()
            );
            if let Some(slot) = event
                .slot_ids
                .iter()
                .map(SlotId::as_str)
                .find(|slot| !is_path_safe(slot))
            {
                return Err(Error::refused(format!(
                    "fixture event {index}: slot id {slot:?} is not an id of ASCII letters, digits, '-' or '_'"
                )));
            }
        }
        Ok(settings)
    }

    /// The target origin as `http://host[:port]`, without a trailing slash.
    ///
    /// The origin is echoed in an error only once it is known to carry no credentials.
    ///
    /// # Errors
    ///
    /// The origin is not a URL, carries credentials, is not plain http, names no host, or holds
    /// a path, a query or a fragment.
    pub fn normalized_origin(&self) -> Result<String> {
        let url = url::Url::parse(&self.target_origin)
            .map_err(|error| Error::refused(format!("target_origin is not a URL: {error}")))?;
        refuse_unless!(
            url.username().is_empty() && url.password().is_none(),
            "target_origin must carry no credentials"
        );
        let origin = &self.target_origin;
        refuse_unless!(
            url.scheme() == "http",
            "target_origin {origin:?} must be plain http: the load target is a LAN listener, and \
             this crate's HTTP client carries no TLS"
        );
        refuse_unless!(
            url.host_str().is_some(),
            "target_origin {origin:?} must name a host"
        );
        refuse_unless!(
            url.path() == "/" && url.query().is_none() && url.fragment().is_none(),
            "target_origin {origin:?} must be an origin, without a path, query or fragment"
        );
        Ok(url.origin().ascii_serialization())
    }
}
