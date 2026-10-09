//! The accounts a run signs in with: the account file, each client's share, and the refresh.
//!
//! - **Role:** reads the account file once, deals each client its accounts (client `c` holds
//!   accounts `c`, `c + clients`, `c + 2·clients`, …) as a ring of refresh tokens and a list of
//!   per-account request state, and builds and decodes the `POST /api/v1/auth/refresh` exchange
//!   that signs a client into an account.
//! - **Position:** [`crate::run`] reads the file and deals the accounts; a client's account
//!   switches refresh through [`AccountRing`] and its member requests keep their state in
//!   [`MemberAccount`].
//! - **Signals & state:** each [`AccountRing`] keeps its accounts' newest refresh tokens and which
//!   are retired, and each [`MemberAccount`] its step cursors and entity tags, in memory only;
//!   access tokens pass from the account switches to the member requests without being stored
//!   here; nothing here writes a file.
//! - **Invariants:**
//!   - A refresh token is presented at most once: a successful refresh replaces it with its
//!     successor at once, and a failed one retires the account for the rest of the run, because
//!     the API may have spent the token and a replay revokes the account's whole token family.
//!   - A token never reaches a `Debug` rendering, an error message or the report.
//!
//! The account file is JSON, `{"accounts": [{"discord_id": "…", "refresh_token": "…"}, …]}`, with
//! account `k` at position `k` and exactly `clients × accounts_per_client` entries.
//!
//! @contract session-token.schema.json#/definitions/SessionTokenRequest
//! @contract session-token.schema.json#/definitions/SessionTokenPair

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use serde::Deserialize;
use staging_load_plan::identifiers::DiscordId;
use staging_load_plan::request_catalog::{AUTH_PATH_PREFIX, AccountBinding, ResolvedRequest};
use staging_load_plan::workload_plan::{FixtureEvent, HttpMethod};

use crate::error::{Error, Result, refuse_account_file_unless};

/// The route a client refreshes into its next account through.
pub(crate) const REFRESH_PATH: &str = "/api/v1/auth/refresh";
/// The only status a refresh expects.
const REFRESH_EXPECTED_STATUS: u16 = 200;

/// A bearer or refresh token: printable only through [`SecretToken::expose`].
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SecretToken(String);

impl SecretToken {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretToken(redacted)")
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountFile {
    accounts: Vec<AccountFileEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountFileEntry {
    discord_id: String,
    refresh_token: String,
}

/// One account as the file names it.
#[derive(Debug)]
pub(crate) struct AccountSecret {
    discord_id: String,
    refresh_token: SecretToken,
}

/// Read the account file once and check it holds `expected` distinct accounts.
///
/// A decoding error reports its category, line and column only, so no token is echoed.
pub(crate) fn read_account_file(path: &Path, expected: usize) -> Result<Vec<AccountSecret>> {
    let bytes = std::fs::read(path).map_err(|error| Error::AccountFileUnreadable {
        path: path.to_path_buf(),
        error,
    })?;
    let file: AccountFile = serde_json::from_slice(&bytes).map_err(|error| {
        Error::account_file_refused(format!(
            "the account file {} is not an account list: {:?} error at line {}, column {}",
            path.display(),
            error.classify(),
            error.line(),
            error.column()
        ))
    })?;
    refuse_account_file_unless!(
        file.accounts.len() == expected,
        "the account file {} holds {} accounts; the workload needs {expected}",
        path.display(),
        file.accounts.len()
    );
    let mut seen = HashSet::new();
    for (index, entry) in file.accounts.iter().enumerate() {
        refuse_account_file_unless!(
            !entry.discord_id.is_empty() && entry.discord_id.bytes().all(|b| b.is_ascii_digit()),
            "account {index} of {} has a discord_id that is not a decimal id",
            path.display()
        );
        refuse_account_file_unless!(
            seen.insert(entry.discord_id.as_str()),
            "account {index} of {} repeats discord id {}",
            path.display(),
            entry.discord_id
        );
        refuse_account_file_unless!(
            !entry.refresh_token.is_empty(),
            "account {index} of {} has an empty refresh_token",
            path.display()
        );
    }
    Ok(file
        .accounts
        .into_iter()
        .map(|entry| AccountSecret {
            discord_id: entry.discord_id,
            refresh_token: SecretToken(entry.refresh_token),
        })
        .collect())
}

/// Deal account `k` to client `k mod clients`, binding it to its fixture event and slot.
pub(crate) fn deal_accounts(
    accounts: Vec<AccountSecret>,
    events: &[FixtureEvent],
    clients: u32,
    templates: usize,
) -> Vec<ClientAccounts> {
    let clients = clients as usize;
    let mut dealt: Vec<ClientAccounts> = (0..clients)
        .map(|_| ClientAccounts {
            ring: AccountRing::default(),
            members: Vec::new(),
        })
        .collect();
    for (index, secret) in accounts.into_iter().enumerate() {
        let share = &mut dealt[index % clients];
        share.ring.accounts.push(RingAccount {
            index,
            refresh_token: secret.refresh_token,
            retired: false,
        });
        share.members.push(MemberAccount {
            index,
            binding: AccountBinding::for_account(index, &DiscordId::new(secret.discord_id), events),
            step_cursors: vec![0; templates],
            entity_tags: HashMap::new(),
        });
    }
    dealt
}

/// A client's share of the accounts, by ring position: the ring its account switches refresh,
/// and the request state its member requests keep.
#[derive(Debug)]
pub(crate) struct ClientAccounts {
    pub(crate) ring: AccountRing,
    pub(crate) members: Vec<MemberAccount>,
}

/// One account as a client's account switches hold it.
#[derive(Debug)]
pub(crate) struct RingAccount {
    pub(crate) index: usize,
    refresh_token: SecretToken,
    retired: bool,
}

/// One account as a client's member requests use it: what its requests name, and where it is in
/// each template's cycle.
#[derive(Debug)]
pub(crate) struct MemberAccount {
    pub(crate) index: usize,
    pub(crate) binding: AccountBinding,
    step_cursors: Vec<usize>,
    entity_tags: HashMap<String, String>,
}

impl MemberAccount {
    /// How many expected answers the account has had from `template`, which selects its next step.
    pub(crate) fn step_cursor(&self, template: usize) -> usize {
        self.step_cursors[template]
    }

    pub(crate) fn advance_step(&mut self, template: usize) {
        self.step_cursors[template] += 1;
    }

    pub(crate) fn entity_tag(&self, path: &str) -> Option<&str> {
        self.entity_tags.get(path).map(String::as_str)
    }

    pub(crate) fn remember_entity_tag(&mut self, path: String, tag: String) {
        self.entity_tags.insert(path, tag);
    }
}

/// A client's accounts, the one it sends as, and the one it refreshes next.
#[derive(Debug, Default)]
pub(crate) struct AccountRing {
    accounts: Vec<RingAccount>,
    current: Option<usize>,
    next: usize,
}

impl AccountRing {
    /// The position of the account the client sends as, if it holds one.
    pub(crate) fn current(&self) -> Option<usize> {
        self.current
    }

    /// The first account from the next position on, wrapping, that is not retired.
    pub(crate) fn next_candidate(&self) -> Option<usize> {
        let count = self.accounts.len();
        (0..count)
            .map(|offset| (self.next + offset) % count)
            .find(|&position| !self.accounts[position].retired)
    }

    pub(crate) fn account(&self, position: usize) -> &RingAccount {
        &self.accounts[position]
    }

    /// Keep the refreshed pair's successor refresh token and hand back its access token; the
    /// next refresh moves past the account.
    pub(crate) fn take_refreshed(&mut self, position: usize, pair: RefreshedPair) -> SecretToken {
        self.accounts[position].refresh_token = pair.refresh_token;
        self.next = (position + 1) % self.accounts.len();
        pair.access_token
    }

    /// The client now sends as the account at `position`.
    pub(crate) fn switch_to(&mut self, position: usize) {
        self.current = Some(position);
    }

    /// Retire an account whose refresh failed; the next refresh moves past it.
    pub(crate) fn retire(&mut self, position: usize) {
        self.accounts[position].retired = true;
        if self.current == Some(position) {
            self.current = None;
        }
        self.next = (position + 1) % self.accounts.len();
    }
}

/// The request that refreshes `account`: its newest refresh token in a `SessionTokenRequest`.
///
/// @route POST /api/v1/auth/refresh
pub(crate) fn refresh_request(account: &RingAccount) -> ResolvedRequest {
    let body = serde_json::json!({ "refresh_token": account.refresh_token.expose() });
    ResolvedRequest {
        method: HttpMethod::Post,
        path: REFRESH_PATH.to_owned(),
        body: Some(body.to_string().into_bytes()),
        expected_statuses: vec![REFRESH_EXPECTED_STATUS],
        conditional: false,
        auth: REFRESH_PATH.starts_with(AUTH_PATH_PREFIX),
    }
}

/// The `SessionTokenPair` a successful refresh answers with.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionTokenPair {
    access_token: String,
    expires_at: String,
    refresh_token: String,
    token_type: String,
}

/// The tokens a successful refresh hands over.
#[derive(Debug)]
pub(crate) struct RefreshedPair {
    access_token: SecretToken,
    refresh_token: SecretToken,
}

/// Decode a refresh answer; `None` unless it is a complete `SessionTokenPair` of type `Bearer`.
pub(crate) fn decode_refresh_answer(body: &[u8]) -> Option<RefreshedPair> {
    let pair: SessionTokenPair = serde_json::from_slice(body).ok()?;
    let complete = pair.token_type == "Bearer"
        && !pair.access_token.is_empty()
        && !pair.refresh_token.is_empty()
        && !pair.expires_at.is_empty();
    complete.then_some(RefreshedPair {
        access_token: SecretToken(pair.access_token),
        refresh_token: SecretToken(pair.refresh_token),
    })
}
