//! Probes: one request departing from a world fixture and what its response must show.
//!
//! **Role:** [`Probe`], its request [`Change`]s and its [`Expect`]ation ([`Outcome`], envelope,
//! contract, `Location` and error details), built with chained calls.
//!
//! **Position:** test support; re-exported by [`super::spec`], so a part spec imports probes
//! from there; [`super::derived_probes`] builds the derived ones and the dimension runner sends
//! them.
//!
//! **Signals & state:** none; plain values.
//!
//! **Invariants:** a probe expects the spec's success until told otherwise; a status of 400 or
//! more is checked against the `{error, details?}` envelope unless the expectation names why
//! the refusal carries none.

use serde_json::Value;

use super::spec::{Actor, Contract};

/// How a probe's request departs from its fixture.
#[derive(Debug, Clone)]
pub enum Change {
    /// Replace a path parameter.
    Param(&'static str, String),
    /// Replace the query string (without `?`).
    Query(String),
    /// Replace the JSON body.
    Body(Value),
    /// Merge these top-level fields into the JSON body.
    MergeBody(Value),
    /// Remove a top-level field from the JSON body.
    RemoveField(&'static str),
    /// Send these bytes with this `Content-Type` (none when `None`) instead of the JSON body.
    RawBody(Vec<u8>, Option<&'static str>),
    /// Send a JSON body one byte over the route's body limit.
    OverLimitBody,
    /// Add a request header.
    Header(&'static str, String),
}

/// The expected status of a probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The spec's success status, with its contract checked.
    Success,
    /// This status.
    Status(u16),
}

/// What a probe's response must show.
#[derive(Debug, Clone)]
pub struct Expect {
    pub outcome: Outcome,
    /// A contract checked instead of (or, for a status, in addition to) the spec's own.
    pub contract: Option<Contract>,
    /// A substring of the envelope's `error`.
    pub error_contains: Option<&'static str>,
    /// The envelope's `details.code`.
    pub details_code: Option<&'static str>,
    /// A substring of the `Location` header.
    pub location_contains: Option<&'static str>,
    /// Why this refusal carries no `{error, details?}` envelope; `None` checks the envelope.
    pub without_envelope: Option<&'static str>,
    /// JSON pointers into the body and the value each must hold.
    pub json_equals: Vec<(&'static str, Value)>,
    /// A JSON pointer to an array and the most items it may hold (a paging clamp).
    pub max_items: Option<(&'static str, usize)>,
}

impl Expect {
    /// The spec's success status and contract.
    pub fn success() -> Expect {
        Expect::with_outcome(Outcome::Success)
    }

    /// This status; a status of 400 or more is checked for the envelope.
    pub fn status(status: u16) -> Expect {
        Expect::with_outcome(Outcome::Status(status))
    }

    fn with_outcome(outcome: Outcome) -> Expect {
        Expect {
            outcome,
            contract: None,
            error_contains: None,
            details_code: None,
            location_contains: None,
            without_envelope: None,
            json_equals: Vec::new(),
            max_items: None,
        }
    }

    /// The body's value at JSON pointer `pointer` (e.g. `/data/0/id`) equals `value`.
    pub fn json_at(mut self, pointer: &'static str, value: Value) -> Expect {
        self.json_equals.push((pointer, value));
        self
    }

    /// The array at JSON pointer `pointer` holds at most `limit` items.
    pub fn max_items(mut self, pointer: &'static str, limit: usize) -> Expect {
        self.max_items = Some((pointer, limit));
        self
    }

    /// Also check the body against `contract`.
    pub fn contract(mut self, contract: Contract) -> Expect {
        self.contract = Some(contract);
        self
    }

    /// The envelope's `error` contains `text`.
    pub fn error_contains(mut self, text: &'static str) -> Expect {
        self.error_contains = Some(text);
        self
    }

    /// The envelope's `details.code` equals `code`.
    pub fn details_code(mut self, code: &'static str) -> Expect {
        self.details_code = Some(code);
        self
    }

    /// The `Location` header contains `text`.
    pub fn location_contains(mut self, text: &'static str) -> Expect {
        self.location_contains = Some(text);
        self
    }

    /// The refusal carries no envelope, for the stated reason.
    pub fn without_envelope(mut self, reason: &'static str) -> Expect {
        self.without_envelope = Some(reason);
        self
    }
}

/// One request and what its response must show.
#[derive(Debug, Clone)]
pub struct Probe {
    pub name: String,
    pub actor: Actor,
    /// The world fixture to start from; `None` uses the spec's fixture.
    pub fixture: Option<&'static str>,
    pub changes: Vec<Change>,
    pub expect: Expect,
}

impl Probe {
    /// A probe sent by `actor` from the spec's fixture, expecting the success outcome until
    /// [`Probe::expect`] or [`Probe::expect_with`] says otherwise.
    pub fn new(name: impl Into<String>, actor: Actor) -> Probe {
        Probe {
            name: name.into(),
            actor,
            fixture: None,
            changes: Vec::new(),
            expect: Expect::success(),
        }
    }

    /// Start from the world fixture `key` instead of the spec's.
    pub fn fixture(mut self, key: &'static str) -> Probe {
        self.fixture = Some(key);
        self
    }

    /// Apply `change` to the fixture's request.
    pub fn change(mut self, change: Change) -> Probe {
        self.changes.push(change);
        self
    }

    /// Replace path parameter `name` with `value`.
    pub fn param(self, name: &'static str, value: impl Into<String>) -> Probe {
        self.change(Change::Param(name, value.into()))
    }

    /// Replace the query string.
    pub fn query(self, query: impl Into<String>) -> Probe {
        self.change(Change::Query(query.into()))
    }

    /// Replace the JSON body.
    pub fn body(self, body: Value) -> Probe {
        self.change(Change::Body(body))
    }

    /// Merge fields into the JSON body.
    pub fn merge_body(self, fields: Value) -> Probe {
        self.change(Change::MergeBody(fields))
    }

    /// Expect `status`.
    pub fn expect(mut self, status: u16) -> Probe {
        self.expect = Expect::status(status);
        self
    }

    /// Expect exactly `expect`.
    pub fn expect_with(mut self, expect: Expect) -> Probe {
        self.expect = expect;
        self
    }
}
