//! The probes the framework derives from a spec's access class.
//!
//! **Role:** expands a [`RouteSpec`] into one [`Plan`] per [`Dimension`]: the derived probes
//! (the authorized call; anonymous 401, rank-below 403, machine credential refusals,
//! observability 401) plus the spec's own probes, with its overrides applied.
//!
//! **Position:** test support between [`super::spec`] (input) and the dimension runner
//! (consumer).
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** derivation reads only the spec, never the world; an override replaces the
//! expectation of a derived probe and never removes it.

use std::collections::BTreeMap;

use super::spec::{Access, Actor, Dimension, Expect, Probe, Role, RouteSpec};

/// What one dimension of one spec runs.
#[derive(Debug, Clone)]
pub(crate) enum Plan {
    /// These probes, derived and declared.
    Probes(Vec<Probe>),
    /// Nothing, for this reason.
    NotApplicable(String),
    /// Nothing derived and nothing declared: a coverage failure.
    Undeclared,
}

/// The actor the authorized probe uses when the spec names none.
pub(crate) fn least_privileged_actor(access: Access) -> Actor {
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
pub(crate) fn derived(
    spec: &RouteSpec,
    dimension: Dimension,
) -> Result<Vec<Probe>, Option<&'static str>> {
    let access = spec.access;
    let authorized = spec
        .authorized_actor
        .unwrap_or_else(|| least_privileged_actor(access));
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
    };
    if probes.is_empty() {
        Err(None)
    } else {
        Ok(probes)
    }
}

/// The plan of every dimension of `spec`: derived probes (with overrides applied) followed by
/// the spec's own probes.
pub(crate) fn plan(spec: &RouteSpec) -> BTreeMap<Dimension, Plan> {
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
