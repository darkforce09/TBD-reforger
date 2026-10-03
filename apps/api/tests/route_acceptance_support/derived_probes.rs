//! The probes the framework derives from a spec's access class, path and body kind.
//!
//! **Role:** expands a [`RouteSpec`] into one [`Plan`] per [`Dimension`]: the derived probes
//! (anonymous 401, rank-below 403, guest, ban 401, machine credential refusals, observability
//! 401, non-UUID path ids 400, invalid JSON 400, missing content type 415, over-limit body 413,
//! nonexistent path id 404) plus the spec's own probes, with its overrides applied; and lists
//! the declaration problems the coverage binary reports, including those of a refusal-only
//! spec.
//!
//! **Position:** test support between [`super::spec`] (input) and the dimension runner and the
//! coverage binary (consumers).
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** derivation reads only the spec, never the world, so the coverage binary sees
//! exactly the probes the part binaries run; an override replaces the expectation of a derived
//! probe and never removes it; a `NotApplicable` on a dimension the framework derives probes
//! for is a declaration problem, not a suppression; a refusal-only spec holds only with a 4xx
//! status, a non-empty envelope message and a reason naming the module of the route's `@route`
//! tag.

use std::collections::BTreeMap;

use super::route_tags::RouteTag;
use super::spec::{
    Access, Actor, BodyKind, Change, Contract, Dimension, Expect, Probe, Role, RouteSpec,
};

/// What one dimension of one spec runs.
#[derive(Debug, Clone)]
pub enum Plan {
    /// These probes, derived and declared.
    Probes(Vec<Probe>),
    /// Nothing, for this reason.
    NotApplicable(String),
    /// Nothing derived and nothing declared: a coverage failure.
    Undeclared,
}

/// The actor the authorized probe uses when the spec names none.
pub fn least_privileged_actor(access: Access) -> Actor {
    match access {
        Access::Public | Access::DevelopmentOnly => Actor::Anonymous,
        Access::Authenticated => Actor::User(Role::Guest),
        Access::Role(minimum) => Actor::User(minimum),
        Access::Machine(executor) => Actor::Machine(executor),
        Access::Observability => Actor::Observability,
    }
}

fn refusal(name: impl Into<String>, actor: Actor, status: u16) -> Probe {
    Probe::new(name, actor).expect(status)
}

/// The probes the framework derives for `dimension`, or the reason it derives none.
pub fn derived(spec: &RouteSpec, dimension: Dimension) -> Result<Vec<Probe>, Option<&'static str>> {
    let access = spec.access;
    let authorized = spec
        .authorized_actor
        .unwrap_or_else(|| least_privileged_actor(access));
    let session = matches!(access, Access::Authenticated | Access::Role(_));
    let probes = match dimension {
        Dimension::Authorized => match spec.success {
            Some(_) => vec![Probe::new("authorized", authorized)],
            None => Vec::new(),
        },
        Dimension::Unauthorized => match access {
            Access::Public => return Err(Some("public route: every caller is admitted")),
            Access::DevelopmentOnly => {
                return Err(Some(
                    "development-only route: every caller is admitted there",
                ));
            }
            Access::Authenticated | Access::Role(_) => {
                let mut probes = vec![
                    refusal("anonymous", Actor::Anonymous, 401),
                    refusal("invalid bearer", Actor::InvalidBearer, 401),
                ];
                if let Access::Role(minimum) = access
                    && let Some(below) = minimum.below()
                {
                    probes.push(
                        Probe::new("rank below", Actor::User(below))
                            .expect_with(Expect::status(403).error_contains("insufficient role")),
                    );
                }
                probes
            }
            Access::Machine(executor) => vec![
                refusal("no credential", Actor::Anonymous, 401),
                refusal("malformed credential", Actor::MachineMalformed, 401),
                refusal("revoked credential", Actor::MachineRevoked(executor), 401),
                refusal("wrong executor", Actor::Machine(executor.other()), 403),
            ],
            Access::Observability => vec![
                refusal("no token", Actor::Anonymous, 401),
                refusal("wrong token", Actor::WrongObservability, 401),
            ],
        },
        Dimension::Ownership => Vec::new(),
        Dimension::Guest => match access {
            Access::Public | Access::DevelopmentOnly => {
                return Err(Some("public route: a guest is admitted like any caller"));
            }
            Access::Authenticated | Access::Role(Role::Guest) => {
                vec![Probe::new("guest", Actor::User(Role::Guest))]
            }
            Access::Role(_) => vec![
                Probe::new("guest", Actor::User(Role::Guest))
                    .expect_with(Expect::status(403).error_contains("insufficient role")),
            ],
            Access::Machine(_) | Access::Observability => {
                vec![refusal("guest session", Actor::User(Role::Guest), 401)]
            }
        },
        Dimension::Ban if session => vec![refusal("banned", Actor::Banned, 401)],
        Dimension::Ban => return Err(Some("no user session is involved")),
        Dimension::Malformed => {
            let mut probes: Vec<Probe> = spec
                .uuid_params()
                .into_iter()
                .map(|name| {
                    Probe::new(format!("non-uuid {name}"), authorized)
                        .param(name, "not-a-uuid")
                        .expect(400)
                })
                .collect();
            if spec.body == BodyKind::Json {
                probes.push(
                    Probe::new("invalid json", authorized)
                        .change(Change::RawBody(
                            b"{\"unterminated".to_vec(),
                            Some("application/json"),
                        ))
                        .expect(400),
                );
                probes.push(
                    Probe::new("missing content type", authorized)
                        .change(Change::RawBody(b"{}".to_vec(), None))
                        .expect(415),
                );
            }
            probes
        }
        Dimension::Boundary => {
            let mut probes: Vec<Probe> = spec
                .uuid_params()
                .into_iter()
                .map(|name| {
                    Probe::new(format!("nonexistent {name}"), authorized)
                        .param(name, uuid::Uuid::new_v4().to_string())
                        .expect(404)
                })
                .collect();
            if spec.body != BodyKind::None {
                probes.push(
                    Probe::new("over-limit body", authorized)
                        .change(Change::OverLimitBody)
                        .expect_with(Expect::status(413).details_code("request_too_large")),
                );
            }
            probes
        }
    };
    if probes.is_empty() {
        Err(None)
    } else {
        Ok(probes)
    }
}

/// The plan of every dimension of `spec`: derived probes (with overrides applied) followed by
/// the spec's own probes.
pub fn plan(spec: &RouteSpec) -> BTreeMap<Dimension, Plan> {
    Dimension::ALL
        .iter()
        .map(|&dimension| (dimension, plan_dimension(spec, dimension)))
        .collect()
}

fn plan_dimension(spec: &RouteSpec, dimension: Dimension) -> Plan {
    let mut probes = derived(spec, dimension).unwrap_or_default();
    for probe in &mut probes {
        if let Some(found) = spec.overrides.iter().find(|o| o.probe == probe.name) {
            probe.expect = found.expect.clone();
        }
    }
    probes.extend(
        spec.probes
            .iter()
            .filter(|(d, _)| *d == dimension)
            .map(|(_, probe)| probe.clone()),
    );
    if !probes.is_empty() {
        return Plan::Probes(probes);
    }
    if let Some((_, reason)) = spec.not_applicable.iter().find(|(d, _)| *d == dimension) {
        return Plan::NotApplicable((*reason).to_string());
    }
    match derived(spec, dimension) {
        Err(Some(reason)) => Plan::NotApplicable(reason.to_string()),
        _ => Plan::Undeclared,
    }
}

/// Every way `spec` fails to declare its acceptance: a missing success contract, an undeclared
/// dimension, an empty reason, a `NotApplicable` that contradicts derived probes, a duplicate
/// probe name, and an override naming no derived probe or giving no reason.
pub fn declaration_problems(spec: &RouteSpec) -> Vec<String> {
    let key = spec.key;
    let mut problems = Vec::new();
    if spec.success.is_none() {
        problems.push(format!(
            "{key}: no documented success status and contract (`.ok` or `.refusal_only`)"
        ));
    }
    for (dimension, plan) in plan(spec) {
        match plan {
            Plan::Undeclared => problems.push(format!(
                "{key}: dimension `{}` declares no probe and no NotApplicable reason",
                dimension.name()
            )),
            Plan::NotApplicable(reason) if reason.trim().is_empty() => problems.push(format!(
                "{key}: dimension `{}` is NotApplicable with an empty reason",
                dimension.name()
            )),
            Plan::Probes(probes) => {
                let declared_na = spec.not_applicable.iter().any(|(d, _)| *d == dimension);
                if declared_na {
                    problems.push(format!(
                        "{key}: dimension `{}` is declared NotApplicable but runs {} probe(s)",
                        dimension.name(),
                        probes.len()
                    ));
                }
                let mut names: Vec<&str> = probes.iter().map(|p| p.name.as_str()).collect();
                names.sort_unstable();
                if let Some(pair) = names.windows(2).find(|pair| pair[0] == pair[1]) {
                    problems.push(format!(
                        "{key}: dimension `{}` runs two probes named `{}`",
                        dimension.name(),
                        pair[0]
                    ));
                }
            }
            Plan::NotApplicable(_) => {}
        }
    }
    for found in &spec.overrides {
        let derives_it = Dimension::ALL.iter().any(|&dimension| {
            derived(spec, dimension)
                .is_ok_and(|probes| probes.iter().any(|p| p.name == found.probe))
        });
        if !derives_it {
            problems.push(format!(
                "{key}: override names no derived probe `{}`",
                found.probe
            ));
        }
        if found.reason.trim().is_empty() {
            problems.push(format!(
                "{key}: override of `{}` gives no reason",
                found.probe
            ));
        }
    }
    if let Some(Contract::Binary(content_type)) = spec.success.as_ref().map(|(_, c)| c)
        && content_type.trim().is_empty()
    {
        problems.push(format!("{key}: Binary contract names no content type"));
    }
    problems
}

/// Every way a refusal-only spec ([`RouteSpec::refusal_only`]) fails its declaration: a status
/// outside 400–499, a success other than the refusal envelope, an empty envelope message, an
/// empty reason, or a reason naming none of the modules (relative to `src/`) whose `@route` tag
/// documents the route. A spec that is not refusal-only has none.
pub fn refusal_problems(spec: &RouteSpec, tags: &[RouteTag]) -> Vec<String> {
    let key = spec.key;
    let Some(reason) = spec.refusal_reason else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    match &spec.success {
        Some((status, Contract::RefusalEnvelope { error })) => {
            if !(400..500).contains(status) {
                problems.push(format!(
                    "{key}: refusal-only status {status} is not a 4xx refusal"
                ));
            }
            if error.trim().is_empty() {
                problems.push(format!(
                    "{key}: the refusal envelope names no error message"
                ));
            }
        }
        _ => problems.push(format!(
            "{key}: a refusal-only spec's success is not its refusal envelope"
        )),
    }
    if reason.trim().is_empty() {
        problems.push(format!("{key}: refusal-only with an empty reason"));
        return problems;
    }
    let modules: Vec<String> = tags
        .iter()
        .filter(|tag| tag.key() == key)
        .map(|tag| repository_relative_module(&tag.file))
        .collect();
    if modules.is_empty() {
        problems.push(format!(
            "{key}: no `@route` tag documents the refused route"
        ));
    } else if !modules
        .iter()
        .any(|module| reason.contains(module.as_str()))
    {
        problems.push(format!(
            "{key}: the refusal-only reason names none of the documenting modules {modules:?}"
        ));
    }
    problems
}

/// The module spelling a refusal-only reason names: a tag file under an API crate, which the
/// collector holds absolute, is spelled from the repository root
/// (`crates/api/<crate>/src/<module>.rs`); a file under this package's `src/` keeps its
/// `src/`-relative spelling.
fn repository_relative_module(file: &std::path::Path) -> String {
    let spelled = file.to_string_lossy().replace('\\', "/");
    if !file.is_absolute() {
        return spelled;
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    repository_layout::find_repository_root_from(manifest)
        .ok()
        .and_then(|root| {
            file.strip_prefix(root)
                .ok()
                .map(std::path::Path::to_path_buf)
        })
        .map_or(spelled, |relative| {
            relative.to_string_lossy().replace('\\', "/")
        })
}
