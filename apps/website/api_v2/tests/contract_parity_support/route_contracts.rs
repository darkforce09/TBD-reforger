//! Route contracts: which schema a golden answers to, and every way a golden breaks it.
//!
//! **Role:** resolves a golden's method and path to its [`RouteContract`] and validates the
//! golden's JSON body, envelope rows or event-stream frames against the `contracts_v2`
//! definitions that contract names.
//!
//! **Position:** reads the table in [`super::route_contract_table`] and the schema files through
//! `tests/contract_support`; consumed by the validation and decoding cases of
//! `tests/contract_parity_goldens.rs` and by [`super::generated_type_decoders`].
//!
//! **Signals & state:** none; pure functions over the table and the committed schema files.
//!
//! **Invariants:** a golden with no matching row, or with two equally specific rows that disagree,
//! is a violation, never a silent pass; every violation names the JSON pointer it concerns.

use jsonschema::Validator;
use serde_json::Value;

use super::event_stream_frames;
use super::route_contract_table::route_contracts;
use crate::contract_support;

/// Where a schema lives inside its `contracts_v2/definitions` file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaLocation {
    /// The file's root schema.
    Root,
    /// A draft-07 `#/definitions/<name>` entry.
    Definition(&'static str),
}

/// One schema: a `contracts_v2/definitions` file and the location inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaRef {
    /// Path relative to `contracts_v2/definitions`.
    pub file: &'static str,
    /// The schema inside that file.
    pub location: SchemaLocation,
}

impl SchemaRef {
    /// The root schema of `file`.
    pub const fn root(file: &'static str) -> Self {
        Self {
            file,
            location: SchemaLocation::Root,
        }
    }

    /// The draft-07 definition `name` of `file`.
    pub const fn definition(file: &'static str, name: &'static str) -> Self {
        Self {
            file,
            location: SchemaLocation::Definition(name),
        }
    }

    /// The schema's `file#pointer` spelling, for reports.
    pub fn describe(&self) -> String {
        match self.location {
            SchemaLocation::Root => format!("{}#", self.file),
            SchemaLocation::Definition(name) => format!("{}#/definitions/{name}", self.file),
        }
    }

    /// A format-checking validator for this schema.
    pub fn validator(&self) -> Validator {
        match self.location {
            SchemaLocation::Root => contract_support::validator(self.file, None),
            SchemaLocation::Definition(name) => contract_support::validator(self.file, Some(name)),
        }
    }

    /// Every violation of this schema by `value`, each prefixed with `at`.
    pub fn violations(&self, at: &str, value: &Value) -> Vec<String> {
        self.validator()
            .iter_errors(value)
            .map(|error| {
                format!(
                    "{at}{}: {error} ({})",
                    error.instance_path(),
                    self.describe()
                )
            })
            .collect()
    }
}

/// The shape a route's success answer takes.
#[derive(Clone, Copy, Debug)]
pub enum ResponseContract {
    /// A JSON body validated as a whole.
    Body(SchemaRef),
    /// A top-level JSON array whose every element answers to the schema.
    ArrayOf(SchemaRef),
    /// An event stream: the first frame's data answers to `first`; each later frame's data
    /// answers to the schema paired with its `event:` name (`""` for an unnamed frame).
    EventStream {
        first: SchemaRef,
        later: &'static [(&'static str, SchemaRef)],
    },
}

/// The schema each `data` row of an envelope answers to separately.
#[derive(Clone, Copy, Debug)]
pub struct RowContract {
    /// The member of each row that carries the document, or `None` for the whole row.
    pub member: Option<&'static str>,
    /// The row schema.
    pub schema: SchemaRef,
}

/// One route's success contract.
#[derive(Clone, Debug)]
pub struct RouteContract {
    /// HTTP method, upper case.
    pub method: &'static str,
    /// Path in the `@route` spelling.
    pub path: &'static str,
    /// A `(name, value)` query pair the request must carry for this row to apply.
    pub query: Option<(&'static str, &'static str)>,
    /// The success answer's shape.
    pub response: ResponseContract,
    /// The separately validated envelope rows, when the envelope has them.
    pub rows: Option<RowContract>,
}

impl RouteContract {
    /// This row, applying only when the request carries `name=value`.
    pub fn when_query(mut self, name: &'static str, value: &'static str) -> Self {
        self.query = Some((name, value));
        self
    }

    /// This row, with each `data` row (or its `member`) validated against `schema`.
    pub fn with_rows(mut self, member: Option<&'static str>, schema: SchemaRef) -> Self {
        self.rows = Some(RowContract { member, schema });
        self
    }

    /// How specific this row is for `(method, path, query)`, or `None` when it does not apply.
    fn specificity(&self, method: &str, path: &str, query: &str) -> Option<(bool, usize)> {
        if self.method != method {
            return None;
        }
        let pattern: Vec<&str> = self.path.split('/').collect();
        let actual: Vec<&str> = path.split('/').collect();
        if pattern.len() != actual.len() {
            return None;
        }
        let mut literals = 0;
        for (expected, segment) in pattern.iter().zip(&actual) {
            let parameter = expected.starts_with(':') || expected.starts_with('{');
            if parameter {
                if segment.is_empty() {
                    return None;
                }
            } else if expected != segment {
                return None;
            } else {
                literals += 1;
            }
        }
        match self.query {
            None => Some((false, literals)),
            Some((name, value)) => query
                .split('&')
                .any(|pair| pair == format!("{name}={value}"))
                .then_some((true, literals)),
        }
    }
}

fn contract(method: &'static str, path: &'static str, response: ResponseContract) -> RouteContract {
    RouteContract {
        method,
        path,
        query: None,
        response,
        rows: None,
    }
}

/// A route answering the draft-07 definition `name` of `file`.
pub fn definition(
    method: &'static str,
    path: &'static str,
    file: &'static str,
    name: &'static str,
) -> RouteContract {
    contract(
        method,
        path,
        ResponseContract::Body(SchemaRef::definition(file, name)),
    )
}

/// A route answering the root schema of `file`.
pub fn root(method: &'static str, path: &'static str, file: &'static str) -> RouteContract {
    contract(method, path, ResponseContract::Body(SchemaRef::root(file)))
}

/// A route answering a top-level array of the definition `name` of `file`.
pub fn array_of(
    method: &'static str,
    path: &'static str,
    file: &'static str,
    name: &'static str,
) -> RouteContract {
    contract(
        method,
        path,
        ResponseContract::ArrayOf(SchemaRef::definition(file, name)),
    )
}

/// A route answering an event stream.
pub fn event_stream(
    method: &'static str,
    path: &'static str,
    first: SchemaRef,
    later: &'static [(&'static str, SchemaRef)],
) -> RouteContract {
    contract(method, path, ResponseContract::EventStream { first, later })
}

/// The contract of the route `method path_and_query` answers on.
///
/// # Errors
/// Names the request when no row matches, or when two equally specific rows disagree.
pub fn contract_for(method: &str, path_and_query: &str) -> Result<RouteContract, String> {
    let (path, query) = path_and_query
        .split_once('?')
        .unwrap_or((path_and_query, ""));
    let mut matches: Vec<((bool, usize), RouteContract)> = route_contracts()
        .into_iter()
        .filter_map(|row| row.specificity(method, path, query).map(|rank| (rank, row)))
        .collect();
    matches.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    match matches.as_slice() {
        [] => Err(format!(
            "no route contract answers {method} {path_and_query}; add its row to \
             tests/contract_parity_support/route_contract_table.rs"
        )),
        [(first_rank, first), (second_rank, second), ..]
            if first_rank == second_rank && first.path != second.path =>
        {
            Err(format!(
                "{method} {path_and_query} matches both {} and {} equally",
                first.path, second.path
            ))
        }
        [(_, best), ..] => Ok(best.clone()),
    }
}

/// One part of a golden and the schema it answers to.
#[derive(Clone, Debug)]
pub struct ContractPart {
    /// Where the part sits in the golden (a JSON pointer, or `frame <n> data`).
    pub at: String,
    /// The schema the part answers to.
    pub schema: SchemaRef,
    /// The part itself.
    pub value: Value,
}

/// A golden split into the parts its contract validates, plus every structural break (a missing
/// row array, a frame that is not JSON, an event the route never sends).
#[derive(Debug, Default)]
pub struct ContractParts {
    /// The parts, in golden order.
    pub parts: Vec<ContractPart>,
    /// Structural breaks found while splitting.
    pub breaks: Vec<String>,
}

impl ContractParts {
    fn part(&mut self, at: String, schema: SchemaRef, value: &Value) {
        self.parts.push(ContractPart {
            at,
            schema,
            value: value.clone(),
        });
    }

    /// Every structural break and schema violation, as pointer-named lines.
    pub fn violations(&self) -> Vec<String> {
        let mut violations = self.breaks.clone();
        for part in &self.parts {
            violations.extend(part.schema.violations(&part.at, &part.value));
        }
        violations
    }
}

/// A JSON golden split into the parts `contract` validates.
pub fn json_parts(contract: &RouteContract, body: &Value) -> ContractParts {
    let mut split = ContractParts::default();
    match contract.response {
        ResponseContract::Body(schema) => split.part(String::new(), schema, body),
        ResponseContract::ArrayOf(schema) => match body.as_array() {
            Some(elements) => {
                for (index, element) in elements.iter().enumerate() {
                    split.part(format!("/{index}"), schema, element);
                }
            }
            None => split.breaks.push(format!(
                "/: the body is not an array of {}",
                schema.describe()
            )),
        },
        ResponseContract::EventStream { .. } => split
            .breaks
            .push("a JSON golden on an event-stream route".to_string()),
    }
    if let Some(rows) = contract.rows {
        split_rows(rows, body, &mut split);
    }
    split
}

/// Each `data` row (or its `member`) of an envelope, as a part of its own.
fn split_rows(rows: RowContract, body: &Value, split: &mut ContractParts) {
    let Some(data) = body["data"].as_array() else {
        split
            .breaks
            .push("/data: the envelope carries no row array".to_string());
        return;
    };
    for (index, row) in data.iter().enumerate() {
        match rows.member {
            None => split.part(format!("/data/{index}"), rows.schema, row),
            Some(member) => match row.get(member) {
                Some(document) => {
                    split.part(format!("/data/{index}/{member}"), rows.schema, document);
                }
                None => split
                    .breaks
                    .push(format!("/data/{index}/{member}: missing")),
            },
        }
    }
}

/// An event-stream golden split into its frames' data, each with the schema its event names.
pub fn stream_parts(contract: &RouteContract, golden: &[u8]) -> ContractParts {
    let mut split = ContractParts::default();
    let ResponseContract::EventStream { first, later } = contract.response else {
        split
            .breaks
            .push("an event-stream golden on a JSON route".to_string());
        return split;
    };
    let frames = event_stream_frames::complete_frames(golden);
    if frames.is_empty() {
        split
            .breaks
            .push("the golden holds no complete frame".to_string());
    }
    for (index, frame) in frames.iter().enumerate() {
        let at = format!("frame {index} data");
        let event = event_stream_frames::frame_event(frame);
        let schema = if index == 0 {
            Some(first)
        } else {
            later
                .iter()
                .find(|(name, _)| *name == event)
                .map(|(_, schema)| *schema)
        };
        let Some(schema) = schema else {
            split
                .breaks
                .push(format!("frame {index}: the route sends no `{event}` event"));
            continue;
        };
        match event_stream_frames::frame_json(frame) {
            Ok(data) => split.part(at, schema, &data),
            Err(why) => split.breaks.push(format!("frame {index}: {why}")),
        }
    }
    split
}
