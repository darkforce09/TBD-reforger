//! Executing a part's probes, one dimension at a time.
//!
//! **Role:** builds a [`World`], expands each spec's [`Plan`] for one [`Dimension`], turns each
//! probe into a request from its world fixture, sends it, checks status, envelope, contract and
//! headers, and fails once with every failing probe listed.
//!
//! **Position:** test support; the binary's test functions call [`run_dimension`] with a
//! part's specs and world.
//!
//! **Signals & state:** the world built per call; failures accumulate in a local list.
//!
//! **Invariants:** a dimension whose every spec is `NotApplicable` passes only by printing each
//! reason, and a dimension with probes passes only when every probe ran and held; a fixture
//! missing a path parameter fails the probe instead of sending a malformed URI; each (part,
//! dimension) pair builds its world in its own namespace, so worlds sharing the binary's
//! database never share an account.

use serde_json::Value;

use super::contracts::json_violations;
use super::derived_probes::{Plan, plan};
use super::requests::{Outgoing, Received, envelope_problem, send};
use super::spec::{Change, Contract, Dimension, Outcome, Probe, RouteSpec};
use super::world::{Fixture, PartWorld, World};

async fn build_request<W: PartWorld>(
    world: &World<W>,
    spec: &RouteSpec,
    probe: &Probe,
) -> Result<Outgoing, String> {
    let key = probe.fixture.or(spec.fixture).unwrap_or(spec.key);
    let mut fixture = world
        .part
        .fixture(&world.core, key, probe.actor)
        .await
        .unwrap_or_else(Fixture::new);
    for change in &probe.changes {
        match change {
            Change::Param(name, value) => {
                fixture.params.retain(|(n, _)| n != name);
                fixture.params.push(((*name).to_string(), value.clone()));
            }
            Change::Body(body) => fixture.body = Some(body.clone()),
            Change::MergeBody(fields) => {
                let mut body = fixture.body.take().unwrap_or_else(|| serde_json::json!({}));
                if let (Some(target), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
                    target.extend(fields.clone());
                }
                fixture.body = Some(body);
            }
        }
    }
    let mut path = String::new();
    for segment in spec.path().split('/').skip(1) {
        path.push('/');
        match segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            Some(name) => {
                let value = fixture
                    .params
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| {
                        format!("fixture `{key}` supplies no path parameter `{name}`")
                    })?;
                path.push_str(&value);
            }
            None => path.push_str(segment),
        }
    }
    if let Some(query) = &fixture.query {
        path = format!("{path}?{query}");
    }
    let mut outgoing = Outgoing::new(spec.method(), path);
    outgoing.bearer = world.core.actors.bearer(probe.actor);
    outgoing.headers = fixture.headers;
    let fixture_raw = fixture
        .raw_body
        .map(|(bytes, content_type)| (bytes, Some(content_type)));
    outgoing.body = fixture_raw.or_else(|| {
        fixture.body.map(|body| {
            let bytes = serde_json::to_vec(&body).expect("serialise fixture body");
            (bytes, Some("application/json".to_string()))
        })
    });
    Ok(outgoing)
}

/// Every way `received` contradicts `contract`.
fn contract_problems(contract: &Contract, received: &Received) -> Vec<String> {
    match contract {
        Contract::Schema { .. }
        | Contract::SchemaItems { .. }
        | Contract::RefusalEnvelope { .. } => match received.json() {
            Some(value) => json_violations(contract, &value),
            None => vec![format!("the body is not JSON: {}", received.excerpt())],
        },
        Contract::EventStream { .. } => {
            if !received.content_type().starts_with("text/event-stream") {
                return vec![format!(
                    "Content-Type `{}` is not an event stream",
                    received.content_type()
                )];
            }
            match received.first_event_data() {
                Some(value) => json_violations(contract, &value),
                None => vec![format!("no JSON first event: {}", received.excerpt())],
            }
        }
        Contract::NoBody if received.body.is_empty() => Vec::new(),
        Contract::NoBody => vec![format!("expected no body, got: {}", received.excerpt())],
        Contract::Binary(content_type) => {
            let mut problems = Vec::new();
            if !received.content_type().starts_with(content_type) {
                problems.push(format!(
                    "Content-Type `{}` does not start with `{content_type}`",
                    received.content_type()
                ));
            }
            if received.body.is_empty() {
                problems.push("the binary body is empty".into());
            }
            problems
        }
    }
}

/// Send `probe` and check its response; `Err` describes every contradiction.
async fn run_probe<W: PartWorld>(
    world: &World<W>,
    spec: &RouteSpec,
    probe: &Probe,
) -> Result<(), String> {
    let outgoing = build_request(world, spec, probe).await?;
    let received = send(&world.core.app, &outgoing).await;
    let expect = &probe.expect;
    let success = spec.success.as_ref();
    let status = match expect.outcome {
        Outcome::Success => success
            .map(|(status, _)| *status)
            .ok_or("the spec has no `.ok`")?,
        Outcome::Status(status) => status,
    };
    let mut problems = Vec::new();
    if received.status.as_u16() != status {
        problems.push(format!(
            "expected {status}, got {}",
            received.status.as_u16()
        ));
    } else {
        if status >= 400 && expect.without_envelope.is_none() {
            problems.extend(envelope_problem(&received));
        }
        let body = received.json().unwrap_or(Value::Null);
        if let Some(text) = expect.error_contains
            && !body["error"]
                .as_str()
                .is_some_and(|error| error.contains(text))
        {
            problems.push(format!("`error` does not contain `{text}`"));
        }
        if let Some(code) = expect.details_code
            && body["details"]["code"].as_str() != Some(code)
        {
            problems.push(format!("`details.code` is not `{code}`"));
        }
        for (pointer, value) in &expect.json_equals {
            if body.pointer(pointer) != Some(value) {
                problems.push(format!(
                    "`{pointer}` is {:?}, not {value}",
                    body.pointer(pointer)
                ));
            }
        }
        if let Some((pointer, limit)) = expect.max_items {
            match body.pointer(pointer).and_then(Value::as_array) {
                Some(items) if items.len() <= limit => {}
                Some(items) => {
                    problems.push(format!("`{pointer}` holds {} > {limit}", items.len()))
                }
                None => problems.push(format!("`{pointer}` is not an array")),
            }
        }
        let location = match expect.outcome {
            Outcome::Success => expect.location_contains.or(spec.redirect),
            Outcome::Status(_) => expect.location_contains,
        };
        if let Some(text) = location
            && !received
                .header("location")
                .is_some_and(|l| l.contains(text))
        {
            problems.push(format!(
                "`Location` does not contain `{text}`: {:?}",
                received.header("location")
            ));
        }
        let contract = match (&expect.contract, expect.outcome) {
            (Some(contract), _) => Some(contract),
            (None, Outcome::Success) => success.map(|(_, contract)| contract),
            (None, Outcome::Status(_)) => None,
        };
        if let Some(contract) = contract {
            problems.extend(contract_problems(contract, &received));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} {}: {}\n      response: {}",
            outgoing.method,
            outgoing.uri,
            problems.join("; "),
            received.excerpt()
        ))
    }
}

/// Run every probe of `dimension` over `specs` against a fresh world of `W`; `part` (0–48)
/// numbers the part within the binary.
pub(crate) async fn run_dimension<W: PartWorld>(
    suite: &'static str,
    part: u8,
    specs: Vec<RouteSpec>,
    dimension: Dimension,
) {
    assert!(!specs.is_empty(), "{suite}: no spec to run");
    let index = Dimension::ALL
        .iter()
        .position(|d| *d == dimension)
        .expect("a dimension");
    let namespace = usize::from(part) * Dimension::ALL.len() + index;
    let namespace = u8::try_from(namespace).expect("a namespace below 100");
    let world = World::<W>::build(suite, namespace).await;
    let mut failures = Vec::new();
    let mut ran = 0;
    for spec in &specs {
        match plan(spec)
            .remove(&dimension)
            .expect("every dimension is planned")
        {
            Plan::Probes(probes) => {
                for probe in probes {
                    ran += 1;
                    if let Err(problem) = run_probe(&world, spec, &probe).await {
                        failures.push(format!("{} [{}]: {problem}", spec.key, probe.name));
                    }
                }
            }
            Plan::NotApplicable(reason) => {
                println!(
                    "{} [{}] not applicable: {reason}",
                    spec.key,
                    dimension.name()
                );
            }
            Plan::Undeclared => failures.push(format!(
                "{}: dimension `{}` is undeclared",
                spec.key,
                dimension.name()
            )),
        }
    }
    println!(
        "{suite} {}: {ran} probe(s) over {} spec(s)",
        dimension.name(),
        specs.len()
    );
    assert!(
        failures.is_empty(),
        "{suite} {}: {} of {ran} probe(s) failed:\n  {}",
        dimension.name(),
        failures.len(),
        failures.join("\n  ")
    );
}
