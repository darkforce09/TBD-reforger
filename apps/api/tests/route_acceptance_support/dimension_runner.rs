//! Executing a part's probes, one dimension or the contract parity pass at a time.
//!
//! **Role:** builds a [`World`], expands each spec's [`Plan`] for one [`Dimension`], turns each
//! probe into a request from its world fixture, sends it, checks status, envelope, contract and
//! headers, and fails once with every failing probe listed; the parity pass re-checks every
//! authorized JSON body against its schema and its generated type's round trip (compared
//! beside the schema, date-times as instants in the API's spelling), and every authorized
//! request body against its request contract.
//!
//! **Position:** test support; each part binary's test functions call [`run_dimension`] and
//! [`run_contract_parity`] with that part's specs and world.
//!
//! **Signals & state:** the world built per call; failures accumulate in a local list.
//!
//! **Invariants:** a spec whose key names no registered route fails; a dimension whose every
//! spec is `NotApplicable` passes only by printing each reason, and a dimension with probes
//! passes only when every probe ran and held; a fixture missing a path parameter fails the probe
//! instead of sending a malformed URI.

use api_http_layer::middleware::{MAX_JSON_BODY, MAX_MULTIPART_BODY};
use api_operations::handlers::ballistics_catalogs::upload::MAX_CATALOG_UPLOAD_BODY_BYTES;
use serde_json::Value;

use super::contracts::{json_violations, schema_of};
use super::derived_probes::{Plan, plan};
use super::requests::{Outgoing, Received, envelope_problem, send};
use super::round_trip_comparison::round_trip_differences;
use super::route_table::{RouteRow, row};
use super::spec::{BodyKind, Change, Contract, Dimension, Outcome, Probe, RouteSpec};
use super::world::{Fixture, PartWorld, World};

/// A sent probe: the request it sent and the response it read.
struct Exchange {
    outgoing: Outgoing,
    received: Received,
}

/// The body limit a row enforces under the world's configuration.
fn body_limit(row: &RouteRow, world_limit: i64) -> usize {
    match row.body_limit.as_deref() {
        None => MAX_JSON_BODY,
        Some(expr) if expr.ends_with("MAX_MULTIPART_BODY") => MAX_MULTIPART_BODY,
        Some(expr) if expr.ends_with("MAX_JSON_BODY") => MAX_JSON_BODY,
        Some(expr) if expr.ends_with("MAX_CATALOG_UPLOAD_BODY_BYTES") => {
            MAX_CATALOG_UPLOAD_BODY_BYTES
        }
        Some("version_limit") => usize::try_from(world_limit).expect("a positive body limit"),
        Some(other) => panic!("{}: unknown route body limit `{other}`", row.key()),
    }
}

async fn build_request<W: PartWorld>(
    world: &World<W>,
    spec: &RouteSpec,
    row: &RouteRow,
    probe: &Probe,
) -> Result<Outgoing, String> {
    let key = probe.fixture.or(spec.fixture).unwrap_or(spec.key);
    let mut fixture = world
        .part
        .fixture(&world.core, key, probe.actor)
        .await
        .unwrap_or_else(Fixture::new);
    let mut raw_body: Option<(Vec<u8>, Option<String>)> = None;
    for change in &probe.changes {
        match change {
            Change::Param(name, value) => {
                fixture.params.retain(|(n, _)| n != name);
                fixture.params.push(((*name).to_string(), value.clone()));
            }
            Change::Query(query) => fixture.query = Some(query.clone()),
            Change::Body(body) => fixture.body = Some(body.clone()),
            Change::MergeBody(fields) => {
                let mut body = fixture.body.take().unwrap_or_else(|| serde_json::json!({}));
                if let (Some(target), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
                    target.extend(fields.clone());
                }
                fixture.body = Some(body);
            }
            Change::RemoveField(name) => {
                if let Some(Value::Object(body)) = fixture.body.as_mut() {
                    body.remove(*name);
                }
            }
            Change::RawBody(bytes, content_type) => {
                raw_body = Some((bytes.clone(), content_type.map(str::to_string)));
            }
            Change::OverLimitBody => {
                let limit = body_limit(row, world.core.state.cfg.mission_version_body_limit());
                raw_body = Some(over_limit_body(spec.body, limit));
            }
            Change::Header(name, value) => {
                fixture.headers.push(((*name).to_string(), value.clone()))
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
    outgoing.body = raw_body.or(fixture_raw).or_else(|| {
        fixture.body.map(|body| {
            let bytes = serde_json::to_vec(&body).expect("serialise fixture body");
            (bytes, Some("application/json".to_string()))
        })
    });
    Ok(outgoing)
}

/// A body one byte over `limit`, shaped like the route's body so the size check, not the
/// content type, refuses it: a JSON object, or a multipart form with one file part.
fn over_limit_body(kind: BodyKind, limit: usize) -> (Vec<u8>, Option<String>) {
    let padding = "x".repeat(limit + 1);
    if kind == BodyKind::Multipart {
        let boundary = "route-acceptance-boundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
             filename=\"padding.png\"\r\nContent-Type: image/png\r\n\r\n{padding}\r\n\
             --{boundary}--\r\n"
        );
        let content_type = format!("multipart/form-data; boundary={boundary}");
        return (body.into_bytes(), Some(content_type));
    }
    let body = format!("{{\"padding\":\"{padding}\"}}");
    (body.into_bytes(), Some("application/json".into()))
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
    row: &RouteRow,
    probe: &Probe,
) -> Result<Exchange, String> {
    let outgoing = build_request(world, spec, row, probe).await?;
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
        Ok(Exchange { outgoing, received })
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

fn spec_row(spec: &RouteSpec) -> Result<&'static RouteRow, String> {
    row(spec.key).ok_or_else(|| format!("{}: no registered route has this key", spec.key))
}

/// Run every probe of `dimension` over `specs` against a fresh world of `W`.
pub async fn run_dimension<W: PartWorld>(
    suite: &'static str,
    specs: Vec<RouteSpec>,
    dimension: Dimension,
) {
    assert!(!specs.is_empty(), "{suite}: no spec to run");
    let namespace = Dimension::ALL
        .iter()
        .position(|d| *d == dimension)
        .expect("a dimension");
    let world = World::<W>::build(suite, namespace as u8).await;
    let mut failures = Vec::new();
    let mut ran = 0;
    for spec in &specs {
        let row = match spec_row(spec) {
            Ok(row) => row,
            Err(problem) => {
                failures.push(problem);
                continue;
            }
        };
        match plan(spec)
            .remove(&dimension)
            .expect("every dimension is planned")
        {
            Plan::Probes(probes) => {
                for probe in probes {
                    ran += 1;
                    if let Err(problem) = run_probe(&world, spec, row, &probe).await {
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

/// Re-check every authorized JSON response against its contract and generated type, and every
/// authorized request body against its request contract.
pub async fn run_contract_parity<W: PartWorld>(suite: &'static str, specs: Vec<RouteSpec>) {
    let world = World::<W>::build(suite, Dimension::ALL.len() as u8).await;
    let mut failures = Vec::new();
    let mut checked = 0;
    for spec in &specs {
        let Some((_, contract)) = &spec.success else {
            failures.push(format!("{}: no success contract", spec.key));
            continue;
        };
        if schema_of(contract).is_none() && spec.request_contract.is_none() {
            continue;
        }
        let row = match spec_row(spec) {
            Ok(row) => row,
            Err(problem) => {
                failures.push(problem);
                continue;
            }
        };
        let Plan::Probes(probes) = plan(spec).remove(&Dimension::Authorized).expect("planned")
        else {
            failures.push(format!("{}: no authorized probe", spec.key));
            continue;
        };
        for probe in probes {
            checked += 1;
            let exchange = match run_probe(&world, spec, row, &probe).await {
                Ok(exchange) => exchange,
                Err(problem) => {
                    failures.push(format!("{} [{}]: {problem}", spec.key, probe.name));
                    continue;
                }
            };
            if let (Some(request), Some((bytes, _))) =
                (&spec.request_contract, &exchange.outgoing.body)
                && schema_of(request).is_some()
            {
                let body: Value = serde_json::from_slice(bytes).unwrap_or(Value::Null);
                for violation in json_violations(request, &body) {
                    failures.push(format!("{} request body: {violation}", spec.key));
                }
            }
            let body = match contract {
                Contract::Schema { .. } | Contract::SchemaItems { .. } => exchange.received.json(),
                Contract::EventStream { .. } => exchange.received.first_event_data(),
                _ => None,
            };
            if let (Some(decode), Some(body)) = (spec.decode, body) {
                match decode(&body).map(|again| round_trip_differences(contract, &body, &again)) {
                    Ok(differences) if differences.is_empty() => {}
                    Ok(differences) => failures.push(format!(
                        "{}: the generated type's round trip differs:\n      {}",
                        spec.key,
                        differences.join("\n      ")
                    )),
                    Err(problem) => failures.push(format!("{}: {problem}", spec.key)),
                }
            }
        }
    }
    println!("{suite} contract parity: {checked} authorized exchange(s) checked");
    assert!(
        failures.is_empty(),
        "{suite} contract parity: {} problem(s):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
