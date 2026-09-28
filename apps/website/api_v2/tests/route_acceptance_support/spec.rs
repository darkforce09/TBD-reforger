//! Route acceptance specs as compact data: access classes, dimensions, contracts and probes.
//!
//! **Role:** the vocabulary a part spec file writes — [`RouteSpec`] (built with defaults),
//! [`Access`], the seven [`Dimension`]s, [`Contract`], [`Actor`] and [`Probe`] — and nothing
//! that executes.
//!
//! **Position:** test support; `specs/<part>.rs` builds specs with it,
//! [`super::derived_probes`] expands them into probes, the coverage binary checks them and the
//! dimension runner executes them.
//!
//! **Signals & state:** none; plain values.
//!
//! **Invariants:** a spec names its route as `METHOD /path` in axum spelling; every path
//! parameter is a UUID unless the spec lists it with [`RouteSpec::text_param`] (a `{*tail}`
//! wildcard is always text); a probe's expected status is either the spec's success status
//! ([`Outcome::Success`]) or an explicit one; a refusal is checked against the `{error,
//! details?}` envelope unless the probe names the reason it is exempt; a spec's success is a
//! refusal only when [`RouteSpec::refusal_only`] declares it with the module that documents it.

use serde_json::Value;

pub use super::probe::{Change, Expect, Outcome, Probe};

/// A member role, ordered by rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Guest,
    Enlisted,
    Leader,
    MissionMaker,
    Admin,
}

impl Role {
    /// Every role, lowest rank first.
    pub const ALL: [Role; 5] = [
        Role::Guest,
        Role::Enlisted,
        Role::Leader,
        Role::MissionMaker,
        Role::Admin,
    ];

    /// The database spelling (`user_role`).
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Guest => "guest",
            Role::Enlisted => "enlisted",
            Role::Leader => "leader",
            Role::MissionMaker => "mission_maker",
            Role::Admin => "admin",
        }
    }

    /// The role one rank below, or `None` for a guest.
    pub fn below(self) -> Option<Role> {
        Role::ALL.iter().rev().copied().find(|role| *role < self)
    }
}

/// The program a machine credential authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Executor {
    ModRuntime,
    HostAgent,
}

impl Executor {
    /// The other executor kind: the wrong-executor probe's credential.
    pub fn other(self) -> Executor {
        match self {
            Executor::ModRuntime => Executor::HostAgent,
            Executor::HostAgent => Executor::ModRuntime,
        }
    }
}

/// Who may call a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Every caller, with or without credentials.
    Public,
    /// Registered only in development; every caller there.
    DevelopmentOnly,
    /// Any live session (`AuthUser`), guests included.
    Authenticated,
    /// A live session whose role ranks at least this one.
    Role(Role),
    /// A machine credential of this executor (`MachineCaller`).
    Machine(Executor),
    /// The `OBSERVABILITY_TOKEN` bearer.
    Observability,
}

/// The seven acceptance dimensions every spec declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dimension {
    Authorized,
    Unauthorized,
    Ownership,
    Guest,
    Ban,
    Malformed,
    Boundary,
}

impl Dimension {
    /// Every dimension, in report order.
    pub const ALL: [Dimension; 7] = [
        Dimension::Authorized,
        Dimension::Unauthorized,
        Dimension::Ownership,
        Dimension::Guest,
        Dimension::Ban,
        Dimension::Malformed,
        Dimension::Boundary,
    ];

    /// The lower-case name used in test and report names.
    pub fn name(self) -> &'static str {
        match self {
            Dimension::Authorized => "authorized",
            Dimension::Unauthorized => "unauthorized",
            Dimension::Ownership => "ownership",
            Dimension::Guest => "guest",
            Dimension::Ban => "ban",
            Dimension::Malformed => "malformed",
            Dimension::Boundary => "boundary",
        }
    }
}

/// The shape of a success response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contract {
    /// A JSON body validated against a schema file of `contracts_v2/definitions/` — its root
    /// when `definition` is `None`, else that entry of `definitions` or `$defs`.
    Schema {
        file: &'static str,
        definition: Option<&'static str>,
    },
    /// No body (204, redirects).
    NoBody,
    /// A `text/event-stream` whose first frame's `data` validates like [`Contract::Schema`].
    EventStream {
        file: &'static str,
        definition: Option<&'static str>,
    },
    /// A non-JSON body whose `Content-Type` starts with this value.
    Binary(&'static str),
    /// A JSON array whose every element validates against the entry `definition` of `file`.
    SchemaItems {
        file: &'static str,
        definition: &'static str,
    },
    /// Exactly the envelope `{"error": error}` with no `details`: the answer of a route that
    /// refuses every well-formed request by documented intent ([`RouteSpec::refusal_only`]).
    RefusalEnvelope { error: &'static str },
}

impl Contract {
    /// A JSON body shaped by `file#/definitions/<definition>`.
    pub fn schema(file: &'static str, definition: &'static str) -> Contract {
        Contract::Schema {
            file,
            definition: Some(definition),
        }
    }

    /// A JSON body shaped by the root of `file`.
    pub fn schema_root(file: &'static str) -> Contract {
        Contract::Schema {
            file,
            definition: None,
        }
    }

    /// A JSON array whose every element is shaped by `file#/definitions/<definition>`.
    pub fn schema_items(file: &'static str, definition: &'static str) -> Contract {
        Contract::SchemaItems { file, definition }
    }

    /// A stream whose first frame is shaped by `file#/definitions/<definition>`.
    pub fn event_stream(file: &'static str, definition: &'static str) -> Contract {
        Contract::EventStream {
            file,
            definition: Some(definition),
        }
    }
}

/// Who sends a probe; resolved to credentials by [`super::actors::Actors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Actor {
    /// No `Authorization` header.
    Anonymous,
    /// A bearer that is no token at all.
    InvalidBearer,
    /// The world's account of this role.
    User(Role),
    /// A second account of this role: the non-owner of what `User(role)` owns.
    Peer(Role),
    /// An administrator whose account is banned after its token was issued.
    Banned,
    /// A live credential of this executor for the world's server.
    Machine(Executor),
    /// A live credential of this executor for another server.
    MachineOtherServer(Executor),
    /// A revoked credential of this executor for the world's server.
    MachineRevoked(Executor),
    /// A bearer that is not a well-formed machine credential.
    MachineMalformed,
    /// The configured observability token.
    Observability,
    /// A wrong observability token.
    WrongObservability,
}

/// Decodes a JSON body into a generated contract type and serialises it back.
pub type RoundTrip = fn(&Value) -> Result<Value, String>;

/// A derived probe whose documented outcome differs from the derived default.
#[derive(Debug, Clone)]
pub struct Override {
    pub probe: &'static str,
    pub expect: Expect,
    pub reason: &'static str,
}

/// The kind of request body a route reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    None,
    Json,
    Multipart,
}

/// Everything a route's acceptance needs that the framework cannot derive.
#[derive(Debug, Clone)]
pub struct RouteSpec {
    /// `METHOD /path`, as the route table spells it.
    pub key: &'static str,
    pub access: Access,
    /// The documented success status and contract; a refusal status only on a refusal-only
    /// route.
    pub success: Option<(u16, Contract)>,
    /// Why every well-formed request is refused, naming the module (relative to `src/`) that
    /// documents it; set only by [`RouteSpec::refusal_only`].
    pub refusal_reason: Option<&'static str>,
    /// A substring of every successful redirect's `Location`.
    pub redirect: Option<&'static str>,
    /// The generated type's round trip, where the contract has one.
    pub decode: Option<RoundTrip>,
    /// The request body's contract, where the schema has one.
    pub request_contract: Option<Contract>,
    pub body: BodyKind,
    /// Path parameters that are not UUIDs.
    pub text_params: Vec<&'static str>,
    /// The default world fixture; `None` uses [`RouteSpec::key`].
    pub fixture: Option<&'static str>,
    /// The authorized probe's actor; `None` uses the least-privileged admitted actor.
    pub authorized_actor: Option<Actor>,
    pub probes: Vec<(Dimension, Probe)>,
    pub not_applicable: Vec<(Dimension, &'static str)>,
    pub overrides: Vec<Override>,
}

impl RouteSpec {
    /// A spec for `key` with `access` and nothing else declared.
    pub fn new(key: &'static str, access: Access) -> RouteSpec {
        RouteSpec {
            key,
            access,
            success: None,
            refusal_reason: None,
            redirect: None,
            decode: None,
            request_contract: None,
            body: BodyKind::None,
            text_params: Vec::new(),
            fixture: None,
            authorized_actor: None,
            probes: Vec::new(),
            not_applicable: Vec::new(),
            overrides: Vec::new(),
        }
    }

    /// [`Access::Public`].
    pub fn public(key: &'static str) -> RouteSpec {
        RouteSpec::new(key, Access::Public)
    }
    /// [`Access::DevelopmentOnly`].
    pub fn development_only(key: &'static str) -> RouteSpec {
        RouteSpec::new(key, Access::DevelopmentOnly)
    }
    /// [`Access::Authenticated`].
    pub fn authenticated(key: &'static str) -> RouteSpec {
        RouteSpec::new(key, Access::Authenticated)
    }
    /// [`Access::Role`].
    pub fn role(key: &'static str, minimum: Role) -> RouteSpec {
        RouteSpec::new(key, Access::Role(minimum))
    }
    /// [`Access::Machine`].
    pub fn machine(key: &'static str, executor: Executor) -> RouteSpec {
        RouteSpec::new(key, Access::Machine(executor))
    }
    /// [`Access::Observability`].
    pub fn observability(key: &'static str) -> RouteSpec {
        RouteSpec::new(key, Access::Observability)
    }

    /// The documented success status and response contract.
    pub fn ok(mut self, status: u16, contract: Contract) -> RouteSpec {
        self.success = Some((status, contract));
        self
    }

    /// A documented redirect: `status`, no body, and a `Location` containing `location`.
    pub fn ok_redirect(mut self, status: u16, location: &'static str) -> RouteSpec {
        self.success = Some((status, Contract::NoBody));
        self.redirect = Some(location);
        self
    }

    /// A route that refuses every well-formed request by documented intent: the authorized probe
    /// expects `status` with exactly the envelope `{"error": error}`, and `reason` names the
    /// module (relative to `src/`) whose `@route` tag documents the refusal, which the coverage
    /// binary checks.
    pub fn refusal_only(
        mut self,
        status: u16,
        error: &'static str,
        reason: &'static str,
    ) -> RouteSpec {
        self.success = Some((status, Contract::RefusalEnvelope { error }));
        self.refusal_reason = Some(reason);
        self
    }

    /// Round-trip every authorized JSON body through the generated type.
    pub fn decodes(mut self, round_trip: RoundTrip) -> RouteSpec {
        self.decode = Some(round_trip);
        self
    }

    /// The route reads a JSON body (derives the invalid JSON, content type and size probes).
    pub fn json_body(mut self) -> RouteSpec {
        self.body = BodyKind::Json;
        self
    }

    /// The route reads a multipart body (derives the size probe).
    pub fn multipart_body(mut self) -> RouteSpec {
        self.body = BodyKind::Multipart;
        self
    }

    /// The JSON body's contract; implies [`RouteSpec::json_body`].
    pub fn request_contract(mut self, contract: Contract) -> RouteSpec {
        self.request_contract = Some(contract);
        self.body = BodyKind::Json;
        self
    }

    /// Path parameter `name` is not a UUID: no non-UUID or nonexistent-id probe is derived.
    pub fn text_param(mut self, name: &'static str) -> RouteSpec {
        self.text_params.push(name);
        self
    }

    /// Start every probe from the world fixture `key`.
    pub fn fixture(mut self, key: &'static str) -> RouteSpec {
        self.fixture = Some(key);
        self
    }

    /// The authorized probe's actor.
    pub fn authorized_as(mut self, actor: Actor) -> RouteSpec {
        self.authorized_actor = Some(actor);
        self
    }

    /// Add a probe to `dimension`.
    pub fn probe(mut self, dimension: Dimension, probe: Probe) -> RouteSpec {
        self.probes.push((dimension, probe));
        self
    }

    /// Declare `dimension` not applicable, for `reason`.
    pub fn not_applicable(mut self, dimension: Dimension, reason: &'static str) -> RouteSpec {
        self.not_applicable.push((dimension, reason));
        self
    }

    /// Replace the expectation of the derived probe named `probe`, for `reason`.
    pub fn override_derived(
        mut self,
        probe: &'static str,
        expect: Expect,
        reason: &'static str,
    ) -> RouteSpec {
        self.overrides.push(Override {
            probe,
            expect,
            reason,
        });
        self
    }

    /// Add an unauthorized probe.
    pub fn unauthorized(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Unauthorized, probe)
    }
    /// Add an ownership probe.
    pub fn ownership(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Ownership, probe)
    }
    /// Declare ownership not applicable.
    pub fn no_ownership(self, reason: &'static str) -> RouteSpec {
        self.not_applicable(Dimension::Ownership, reason)
    }
    /// Add a guest probe.
    pub fn guest(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Guest, probe)
    }
    /// Add a ban probe.
    pub fn ban(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Ban, probe)
    }
    /// Add a malformed-request probe.
    pub fn malformed(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Malformed, probe)
    }
    /// Add a boundary probe.
    pub fn boundary(self, probe: Probe) -> RouteSpec {
        self.probe(Dimension::Boundary, probe)
    }

    /// The method half of the key.
    pub fn method(&self) -> &'static str {
        self.key
            .split_once(' ')
            .map_or(self.key, |(method, _)| method)
    }

    /// The path half of the key.
    pub fn path(&self) -> &'static str {
        self.key.split_once(' ').map_or("", |(_, path)| path)
    }

    /// The `{name}` parameters of the path, in order (`*tail` for a wildcard).
    pub fn path_params(&self) -> Vec<&'static str> {
        self.path()
            .split('/')
            .filter_map(|segment| segment.strip_prefix('{')?.strip_suffix('}'))
            .collect()
    }

    /// The path parameters derived probes treat as UUIDs.
    pub fn uuid_params(&self) -> Vec<&'static str> {
        self.path_params()
            .into_iter()
            .filter(|name| !name.starts_with('*') && !self.text_params.contains(name))
            .collect()
    }
}
