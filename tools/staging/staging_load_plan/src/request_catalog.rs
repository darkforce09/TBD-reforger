//! The request mix, compiled: weighted picks, per-account placeholders and resolved requests.
//!
//! - **Role:** checks every template of the workload's mix once (ids, weights, classes, methods,
//!   statuses, placeholders), picks a template from a uniform draw by weight, and resolves one
//!   template step into a concrete request for one account.
//! - **Position:** compiled by the load generator's `run` from the workload's request mix; used
//!   by the virtual clients at every paced slot and by the account rotation for the refresh's
//!   shape.
//! - **Signals & state:** none; a compiled catalog never changes.
//! - **Invariants:**
//!   - A placeholder is `{name}` with a name from [`PLACEHOLDERS`]; any other brace in a path or a
//!     body string is refused when the catalog compiles, so no request is resolved from an
//!     unchecked template. Object keys are never substituted.
//!   - Bound values are ids of ASCII letters, digits, `-` and `_`, so a path needs no escaping.
//!   - A step is conditional exactly when it expects 304, and only a JSON read may expect 304.
//!   - A request is guarded by the stricter authentication ceiling exactly when its path starts
//!     with [`AUTH_PATH_PREFIX`].
//!   - No template reaches the game traffic ([`GAME_TRAFFIC_ROUTES`]): the member load is JSON
//!     reads, JSON writes and refreshes only.

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{Error, Result, refuse_unless};
use crate::identifiers::{DiscordId, EventId, EventMissionId, MissionId, SlotId};
use crate::workload_plan::{FixtureEvent, HttpMethod, RequestClass, RequestStep, RequestTemplate};

/// The placeholders a path or a body string may name, each bound per account.
pub const PLACEHOLDERS: [&str; 6] = [
    "account_index",
    "discord_id",
    "event_id",
    "event_mission_id",
    "mission_id",
    "slot_id",
];

/// Prefix of the API's authentication routes, which the per-address auth ceiling guards.
pub const AUTH_PATH_PREFIX: &str = "/api/v1/auth/";

/// The status that makes a step conditional.
const NOT_MODIFIED: u16 = 304;

/// The route groups under `/api/v1/` that carry game traffic: the game runtime, the fleet executor
/// and match ingest. The member load never sends to them; game operations are measured apart.
pub const GAME_TRAFFIC_ROUTES: [&str; 3] = ["game-runtime", "fleet-executor", "ingest"];

/// The values one account's requests bind their placeholders to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBinding {
    /// The account's position in the account file.
    pub account_index: usize,
    /// The account's Discord id.
    pub discord_id: DiscordId,
    /// The fixture event the account writes to.
    pub event_id: EventId,
    /// That event's mission attachment, which the account registers on.
    pub event_mission_id: EventMissionId,
    /// That event's mission, which the account bookmarks.
    pub mission_id: MissionId,
    /// The slot of the attachment the account claims.
    pub slot_id: SlotId,
}

impl AccountBinding {
    /// Account `index` takes event `index mod events` and that event's slot `index div events`.
    ///
    /// The plan's checks guarantee a non-empty event list with enough slots for every account.
    pub fn for_account(index: usize, discord_id: &DiscordId, events: &[FixtureEvent]) -> Self {
        let event = &events[index % events.len()];
        Self {
            account_index: index,
            discord_id: discord_id.clone(),
            event_id: event.event_id.clone(),
            event_mission_id: event.event_mission_id.clone(),
            mission_id: event.mission_id.clone(),
            slot_id: event.slot_ids[index / events.len()].clone(),
        }
    }

    fn value(&self, name: &str) -> Option<String> {
        Some(match name {
            "account_index" => self.account_index.to_string(),
            "discord_id" => self.discord_id.to_string(),
            "event_id" => self.event_id.to_string(),
            "event_mission_id" => self.event_mission_id.to_string(),
            "mission_id" => self.mission_id.to_string(),
            "slot_id" => self.slot_id.to_string(),
            _ => return None,
        })
    }
}

/// A request bound to one account, ready to send. Its body may carry a refresh token, so it has
/// no `Debug` rendering.
pub struct ResolvedRequest {
    /// The method.
    pub method: HttpMethod,
    /// The path and query under the target origin, every placeholder bound.
    pub path: String,
    /// The JSON body as bytes, every placeholder bound; `None` for no body.
    pub body: Option<Vec<u8>>,
    /// Every status that counts as expected.
    pub expected_statuses: Vec<u16>,
    /// The step expects 304, so the request carries the account's entity tag.
    pub conditional: bool,
    /// The path is under [`AUTH_PATH_PREFIX`], so the stricter auth ceiling guards it.
    pub auth: bool,
}

/// The checked mix with its cumulative weights.
#[derive(Debug)]
pub struct RequestCatalog {
    templates: Vec<RequestTemplate>,
    cumulative_weights: Vec<u64>,
}

impl RequestCatalog {
    /// Check every template and build the weighted catalog.
    ///
    /// # Errors
    ///
    /// The mix is empty, a template is declared twice, or a template's id, weight, class,
    /// methods, paths, placeholders or statuses are refused.
    pub fn compile(mix: &[RequestTemplate]) -> Result<Self> {
        refuse_unless!(!mix.is_empty(), "the request mix is empty");
        let mut ids = HashSet::new();
        let mut cumulative_weights = Vec::with_capacity(mix.len());
        let mut total = 0u64;
        for template in mix {
            check_template(template).map_err(|problem| {
                Error::refused(format!(
                    "request template {:?}: {problem}",
                    template.id.as_str()
                ))
            })?;
            refuse_unless!(
                ids.insert(template.id.as_str()),
                "request template {:?} is declared twice",
                template.id.as_str()
            );
            total += u64::from(template.weight);
            cumulative_weights.push(total);
        }
        Ok(Self {
            templates: mix.to_vec(),
            cumulative_weights,
        })
    }

    /// The number of templates in the mix.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// The template at `index`, as [`RequestCatalog::pick`] returns it.
    pub fn template(&self, index: usize) -> &RequestTemplate {
        &self.templates[index]
    }

    /// The template a uniform 64-bit draw lands on, in proportion to the weights.
    pub fn pick(&self, draw: u64) -> usize {
        let total = self.cumulative_weights.last().copied().unwrap_or(1);
        let target = ((u128::from(draw) * u128::from(total)) >> 64) as u64;
        self.cumulative_weights
            .partition_point(|&edge| edge <= target)
    }
}

/// Bind `step` to one account.
pub fn resolve(step: &RequestStep, binding: &AccountBinding) -> ResolvedRequest {
    let path = substitute(&step.path, binding);
    ResolvedRequest {
        method: step.method,
        auth: path.starts_with(AUTH_PATH_PREFIX),
        path,
        body: step
            .body
            .as_ref()
            .map(|template| substitute_body(template, binding).to_string().into_bytes()),
        expected_statuses: step.expected_statuses.clone(),
        conditional: step.expected_statuses.contains(&NOT_MODIFIED),
    }
}

/// Whether `value` can stand in a path unescaped: ASCII letters, digits, `-` and `_`.
pub fn is_path_safe(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn check_template(template: &RequestTemplate) -> std::result::Result<(), String> {
    let id = template.id.as_str();
    let id_ok = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !id_ok {
        return Err("the id must be lowercase letters, digits and '_'".into());
    }
    if template.weight == 0 {
        return Err("the weight must be at least 1".into());
    }
    if template.steps.is_empty() {
        return Err("a template needs at least one step".into());
    }
    for (index, step) in template.steps.iter().enumerate() {
        check_step(template.class, step).map_err(|problem| format!("step {index}: {problem}"))?;
    }
    Ok(())
}

fn check_step(class: RequestClass, step: &RequestStep) -> std::result::Result<(), String> {
    match (class, step.method) {
        (RequestClass::JsonRead, HttpMethod::Get)
        | (RequestClass::JsonWrite, HttpMethod::Post | HttpMethod::Delete) => {}
        (RequestClass::Session, _) => {
            return Err(
                "the session class is the refresh, which the account rotation issues".into(),
            );
        }
        (class, method) => return Err(format!("a {class:?} step cannot use {method:?}")),
    }
    if step.method == HttpMethod::Get && step.body.is_some() {
        return Err("a GET step takes no body".into());
    }
    if !step.path.starts_with('/') || step.path.chars().any(char::is_whitespace) {
        return Err(format!(
            "the path {:?} must start with '/' and hold no whitespace",
            step.path
        ));
    }
    if is_game_traffic(&step.path) {
        return Err(format!("the path {:?} is game traffic", step.path));
    }
    scan(&step.path)?;
    if let Some(body) = &step.body {
        check_body(body)?;
    }
    if step.expected_statuses.is_empty() {
        return Err("a step needs at least one expected status".into());
    }
    let mut seen = HashSet::new();
    for &status in &step.expected_statuses {
        if !(100..=599).contains(&status) {
            return Err(format!("{status} is not an HTTP status"));
        }
        if !seen.insert(status) {
            return Err(format!("status {status} is listed twice"));
        }
        if status == NOT_MODIFIED && class != RequestClass::JsonRead {
            return Err("only a JSON read may expect 304".into());
        }
    }
    Ok(())
}

/// Whether `path` names a route group of [`GAME_TRAFFIC_ROUTES`].
fn is_game_traffic(path: &str) -> bool {
    path.strip_prefix("/api/v1/")
        .and_then(|rest| rest.split(['/', '?']).next())
        .is_some_and(|group| GAME_TRAFFIC_ROUTES.contains(&group))
}

fn check_body(value: &Value) -> std::result::Result<(), String> {
    match value {
        Value::String(text) => scan(text).map(drop),
        Value::Array(items) => items.iter().try_for_each(check_body),
        Value::Object(fields) => fields.values().try_for_each(check_body),
        _ => Ok(()),
    }
}

/// A run of literal text or one placeholder name.
enum Segment<'a> {
    Literal(&'a str),
    Placeholder(&'a str),
}

/// Split `text` into literal runs and known placeholders; refuse any other brace.
fn scan(text: &str) -> std::result::Result<Vec<Segment<'_>>, String> {
    let mut segments = Vec::new();
    let mut rest = text;
    while let Some(brace) = rest.find(['{', '}']) {
        if rest[brace..].starts_with('}') {
            return Err(format!("{text:?} holds a '}}' that closes nothing"));
        }
        let after = &rest[brace + 1..];
        let Some(close) = after.find('}') else {
            return Err(format!("{text:?} holds a '{{' that is never closed"));
        };
        let name = &after[..close];
        if !PLACEHOLDERS.contains(&name) {
            return Err(format!(
                "{text:?} names the unknown placeholder {{{name}}}; the known ones are {}",
                PLACEHOLDERS.join(", ")
            ));
        }
        if brace > 0 {
            segments.push(Segment::Literal(&rest[..brace]));
        }
        segments.push(Segment::Placeholder(name));
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        segments.push(Segment::Literal(rest));
    }
    Ok(segments)
}

/// `text` with every placeholder bound. A compiled catalog never holds a text `scan` refuses; such
/// a text would be returned unchanged.
fn substitute(text: &str, binding: &AccountBinding) -> String {
    let Ok(segments) = scan(text) else {
        return text.to_owned();
    };
    segments
        .into_iter()
        .map(|segment| match segment {
            Segment::Literal(literal) => literal.to_owned(),
            Segment::Placeholder(name) => binding.value(name).unwrap_or_default(),
        })
        .collect()
}

fn substitute_body(value: &Value, binding: &AccountBinding) -> Value {
    match value {
        Value::String(text) => Value::String(substitute(text, binding)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| substitute_body(item, binding))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, item)| (key.clone(), substitute_body(item, binding)))
                .collect(),
        ),
        other => other.clone(),
    }
}
