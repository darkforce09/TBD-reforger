//! Role: hierarchy.
//! Position: `mission/extensions/objectives/tasks` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Deserialize, Map, Serialize, Value};

/// Closed `tasks[].tier` vocabulary — `$defs/task.tier`.
pub const TIERS: &[&str] = &["primary", "secondary", "optional"];

/// Closed `tasks[].state` vocabulary — `$defs/task.state`.
pub const STATES: &[&str] = &["assigned", "succeeded", "failed"];

/// One row of the runtime transition table.
pub const LEGAL_TRANSITIONS: &[(TaskState, TaskState)] = &[
    (TaskState::Assigned, TaskState::Succeeded),
    (TaskState::Assigned, TaskState::Failed),
];

/// A task's place in the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    /// Domain representation of assigned.
    Assigned,
    /// Domain representation of succeeded.
    Succeeded,
    /// Domain representation of failed.
    Failed,
}

impl TaskState {
    /// Parse a schema state string, or `None` when it is outside [`STATES`].
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "assigned" => Some(Self::Assigned),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// The schema spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// True when this state cannot leave — the HUD hides the marker here.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// A task's priority band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskTier {
    /// Domain representation of primary.
    Primary,
    /// Domain representation of secondary.
    Secondary,
    /// Domain representation of optional.
    Optional,
}

impl TaskTier {
    /// Parse a schema tier string, or `None` when it is outside [`TIERS`].
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "primary" => Some(Self::Primary),
            "secondary" => Some(Self::Secondary),
            "optional" => Some(Self::Optional),
            _ => None,
        }
    }

    /// The schema spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Optional => "optional",
        }
    }
}

/// One authored task, parsed and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredTask {
    /// Id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Tier.
    pub tier: TaskTier,
    /// State.
    pub state: TaskState,
    /// Trigger id.
    pub trigger_id: Option<String>,
    /// Marker id.
    pub marker_id: Option<String>,
    /// Description.
    pub description: Option<String>,

    /// Schedule.
    pub schedule: Option<Schedule>,
}

/// OFCR timing on one task. Wire keys are `startAfterS` / `windowS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    /// Start after s.
    pub start_after_s: i64,
    /// Window s.
    pub window_s: i64,
}

/// True when `windowS` may stand.
#[must_use]
pub fn window_is_legal(window_s: i64) -> bool {
    window_s > 0
}

/// Refuse an illegal schedule. `mission_length_s`: `None` skips the length rule (the AUTHORED_BLOCKS validator does not see `flow.timeLimitSeconds`); `Some(0)` is an authored no-limit; `Some(n)` with `n > 0` requires `0 <= startAfterS < n`.
pub fn validate_schedule(
    start_after_s: i64,
    window_s: i64,
    mission_length_s: Option<i64>,
) -> Result<(), String> {
    if !window_is_legal(window_s) {
        return Err(format!(
            "windowS must be > 0 (got {window_s}) — a timed task needs a window it can evaluate inside"
        ));
    }
    if start_after_s < 0 {
        return Err(format!(
            "startAfterS cannot be negative (got {start_after_s})"
        ));
    }
    if let Some(length) = mission_length_s
        && length > 0
        && start_after_s >= length
    {
        return Err(format!(
            "startAfterS {start_after_s} is not within mission length {length}s"
        ));
    }
    Ok(())
}

/// May `from` become `to`?.
#[must_use]
pub fn is_legal_transition(from: TaskState, to: TaskState) -> bool {
    LEGAL_TRANSITIONS.contains(&(from, to))
}

/// Apply a transition, or say why it is illegal.
pub fn transition(from: TaskState, to: TaskState) -> Result<TaskState, String> {
    if is_legal_transition(from, to) {
        return Ok(to);
    }
    Err(format!(
        "illegal task transition {} → {} — only assigned→succeeded and assigned→failed are legal",
        from.as_str(),
        to.as_str()
    ))
}

/// Parse and validate one authored `tasks` value — an array of [`AuthoredTask`].
pub fn parse(value: &Value) -> Result<Vec<AuthoredTask>, String> {
    let Some(arr) = value.as_array() else {
        return Err(format!(
            "`tasks` must be an array, not {}",
            type_name(value)
        ));
    };

    let mut out = Vec::with_capacity(arr.len());
    let mut seen: Vec<String> = Vec::with_capacity(arr.len());

    for (index, item) in arr.iter().enumerate() {
        let task = parse_item(item, index)?;
        if seen.iter().any(|id| id == &task.id) {
            return Err(format!(
                "`tasks[{index}]`.id is {} — each task id must be unique",
                quote(&task.id)
            ));
        }
        seen.push(task.id.clone());
        out.push(task);
    }

    Ok(out)
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Parse item using the supplied domain data.
pub(super) fn parse_item(value: &Value, index: usize) -> Result<AuthoredTask, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`tasks[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`tasks[{index}]` carries {key:?}, which `$defs/task` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let id = required_string(obj, index, "id")?;
    let title = required_string(obj, index, "title")?;

    let tier_raw = required_string(obj, index, "tier")?;
    let Some(tier) = TaskTier::parse(&tier_raw) else {
        return Err(format!(
            "`tasks[{index}].tier` is {} — the editor authors one of {}",
            quote(&tier_raw),
            TIERS.join(", ")
        ));
    };

    let state_raw = required_string(obj, index, "state")?;
    let Some(state) = TaskState::parse(&state_raw) else {
        return Err(format!(
            "`tasks[{index}].state` is {} — a task is assigned, succeeded or failed",
            quote(&state_raw)
        ));
    };

    Ok(AuthoredTask {
        id,
        title,
        tier,
        state,
        trigger_id: optional_string(obj, index, "triggerId")?,
        marker_id: optional_string(obj, index, "markerId")?,
        description: optional_string(obj, index, "description")?,
        schedule: parse_schedule(obj, index)?,
    })
}

/// Canonical known keys value.
pub(super) const KNOWN_KEYS: &[&str] = &[
    "id",
    "title",
    "tier",
    "state",
    "triggerId",
    "markerId",
    "description",
    "schedule",
];

/// Canonical schedule keys value.
pub(super) const SCHEDULE_KEYS: &[&str] = &["startAfterS", "windowS"];

/// Required string using the supplied domain data.
pub(super) fn required_string(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!("`tasks[{index}].{key}` is required and is missing"));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`tasks[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`tasks[{index}].{key}` is blank — a task needs a {key} the HUD and the runtime can name"
        ));
    }
    Ok(trimmed.to_string())
}

/// Optional string using the supplied domain data.
pub(super) fn optional_string(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`tasks[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`tasks[{index}].{key}` is blank — omit the key rather than authoring an empty {key}"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

/// Parse schedule using the supplied domain data.
pub(super) fn parse_schedule(
    obj: &Map<String, Value>,
    index: usize,
) -> Result<Option<Schedule>, String> {
    let Some(raw) = obj.get("schedule") else {
        return Ok(None);
    };
    let Some(sched) = raw.as_object() else {
        return Err(format!(
            "`tasks[{index}].schedule` must be an object, not {}",
            type_name(raw)
        ));
    };
    for key in sched.keys() {
        if !SCHEDULE_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`tasks[{index}].schedule` carries {key:?}, which the schedule object does not declare"
            ));
        }
    }
    let start_after_s = required_i64(sched, index, "startAfterS")?;
    let window_s = required_i64(sched, index, "windowS")?;
    validate_schedule(start_after_s, window_s, None)?;
    Ok(Some(Schedule {
        start_after_s,
        window_s,
    }))
}

/// Required i64 using the supplied domain data.
pub(super) fn required_i64(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<i64, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`tasks[{index}].schedule.{key}` is required and is missing"
        ));
    };
    let Some(n) = raw.as_i64() else {
        return Err(format!(
            "`tasks[{index}].schedule.{key}` must be an integer, not {}",
            type_name(raw)
        ));
    };
    Ok(n)
}

/// Quote using the supplied domain data.
pub(super) fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// Type name using the supplied domain data.
pub(super) fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
