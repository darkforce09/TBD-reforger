//! T-936.2 — the authored `tasks[]` block: tiers, states, and the transition table.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! Until this slice a mission had no task vocabulary. `mission.schema.json` used `task` only as
//! an objective-type enum value; nothing in core, the editor or the mod tracked
//! primary / secondary / optional assignments or moved them through a state machine. Objectives
//! (T-212) fire `endOn` triggers; this block OBSERVES T-676 trigger completion and never fires
//! those triggers.
//!
//! ══ The transition table ════════════════════════════════════════════════════════════════════
//! `assigned → succeeded` and `assigned → failed` only. Anything else is refused here and, on the
//! mod side, logged and ignored. The illegal-transition test below is the perturbation target:
//! adding `succeeded → assigned` to [`LEGAL_TRANSITIONS`] must turn that test red.
//!
//! ══ The AUTHORED_BLOCKS row ═════════════════════════════════════════════════════════════════
//! [`validate`] is what `extensions.rs` registers. The block is OPTIONAL and rides
//! [`crate::mission::extensions::ExtensionBlocks`] — it is not document-modelled (unlike
//! `winConditions`), so an unauthored mission still emits exactly the bytes it emitted before
//! this module existed.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Closed `tasks[].tier` vocabulary — `$defs/task.tier`.
pub const TIERS: &[&str] = &["primary", "secondary", "optional"];

/// Closed `tasks[].state` vocabulary — `$defs/task.state`.
pub const STATES: &[&str] = &["assigned", "succeeded", "failed"];

/// One row of the runtime transition table.
///
/// Order is the order an operator reads the rule: from assigned, the two terminals. The table is
/// the ONE place the core and the tests agree; [`is_legal_transition`] is a lookup, not a second
/// `match`.
pub const LEGAL_TRANSITIONS: &[(TaskState, TaskState)] = &[
    (TaskState::Assigned, TaskState::Succeeded),
    (TaskState::Assigned, TaskState::Failed),
];

/// A task's place in the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    Assigned,
    Succeeded,
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
    Primary,
    Secondary,
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
    pub id: String,
    pub title: String,
    pub tier: TaskTier,
    pub state: TaskState,
    pub trigger_id: Option<String>,
    pub marker_id: Option<String>,
    pub description: Option<String>,
}

/// May `from` become `to`?
///
/// The table is [`LEGAL_TRANSITIONS`]. Identity is not a transition: restating `assigned` is a
/// no-op the runtime never asks this function about, and succeeding twice would hide a second
/// completion.
#[must_use]
pub fn is_legal_transition(from: TaskState, to: TaskState) -> bool {
    LEGAL_TRANSITIONS.contains(&(from, to))
}

/// Apply a transition, or say why it is illegal.
///
/// # Errors
/// Returns a sentence naming both states when the pair is not in [`LEGAL_TRANSITIONS`].
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
///
/// The error string is a whole sentence naming the value it refused, because it reaches the author
/// through the compile's refusal path and "invalid tasks" tells nobody which row and which field.
///
/// # Errors
/// Returns the refusal clause when the value is not an array, an item is not an object, a required
/// field is missing or outside its vocabulary, an optional string is blank, an unknown property is
/// present, or two rows share an `id`.
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

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn parse_item(value: &Value, index: usize) -> Result<AuthoredTask, String> {
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
    })
}

const KNOWN_KEYS: &[&str] = &[
    "id",
    "title",
    "tier",
    "state",
    "triggerId",
    "markerId",
    "description",
];

fn required_string(obj: &Map<String, Value>, index: usize, key: &str) -> Result<String, String> {
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

fn optional_string(
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

fn quote(s: &str) -> String {
    format!("{s:?}")
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::compile::compile_payload;
    use crate::mission::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};
    use serde_json::json;

    fn three_tiers() -> Value {
        json!([
            {
                "id": "t-pri",
                "title": "Seize the hill",
                "tier": "primary",
                "state": "assigned",
                "triggerId": "trg-hill",
                "markerId": "attack"
            },
            {
                "id": "t-sec",
                "title": "Find the cache",
                "tier": "secondary",
                "state": "assigned",
                "triggerId": "trg-cache"
            },
            {
                "id": "t-opt",
                "title": "Radio check",
                "tier": "optional",
                "state": "assigned",
                "description": "No trigger — stays assigned."
            }
        ])
    }

    fn compile_env_with_tasks(tasks: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "tasks": tasks }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

    #[test]
    fn a_three_tier_block_parses() {
        let got = parse(&three_tiers()).expect("parses");
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].tier, TaskTier::Primary);
        assert_eq!(got[1].tier, TaskTier::Secondary);
        assert_eq!(got[2].tier, TaskTier::Optional);
        assert!(got.iter().all(|t| t.state == TaskState::Assigned));
        assert_eq!(got[0].trigger_id.as_deref(), Some("trg-hill"));
        assert_eq!(got[0].marker_id.as_deref(), Some("attack"));
        assert_eq!(
            got[2].description.as_deref(),
            Some("No trigger — stays assigned.")
        );
        assert!(got[2].trigger_id.is_none());
    }

    #[test]
    fn assigned_to_succeeded_is_legal() {
        assert_eq!(
            transition(TaskState::Assigned, TaskState::Succeeded).expect("legal"),
            TaskState::Succeeded
        );
    }

    #[test]
    fn assigned_to_failed_is_legal() {
        assert_eq!(
            transition(TaskState::Assigned, TaskState::Failed).expect("legal"),
            TaskState::Failed
        );
    }

    /// **The perturbation target.** Adding `succeeded → assigned` to [`LEGAL_TRANSITIONS`] flips
    /// this test from red-when-legalised back to green-when-restored. That is the proof the table
    /// is what this test examines, not a comment next to it.
    #[test]
    fn succeeded_to_assigned_is_illegal() {
        let err = transition(TaskState::Succeeded, TaskState::Assigned)
            .expect_err("succeeded → assigned must be refused");
        assert!(
            err.contains("illegal task transition"),
            "the refusal must name the class of mistake: {err}"
        );
        assert!(
            err.contains("succeeded"),
            "the refusal must name the from-state: {err}"
        );
        assert!(
            err.contains("assigned"),
            "the refusal must name the to-state: {err}"
        );
        assert!(
            !is_legal_transition(TaskState::Succeeded, TaskState::Assigned),
            "the table itself must not list succeeded → assigned"
        );
    }

    #[test]
    fn every_other_pair_is_illegal() {
        let states = [TaskState::Assigned, TaskState::Succeeded, TaskState::Failed];
        for from in states {
            for to in states {
                let legal = is_legal_transition(from, to);
                if from == TaskState::Assigned
                    && matches!(to, TaskState::Succeeded | TaskState::Failed)
                {
                    assert!(legal, "{from:?} → {to:?} must be legal");
                } else {
                    assert!(!legal, "{from:?} → {to:?} must be illegal");
                    transition(from, to).expect_err("refused");
                }
            }
        }
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let err = parse(&json!([
            {"id": "t1", "title": "A", "tier": "primary", "state": "assigned"},
            {"id": "t1", "title": "B", "tier": "secondary", "state": "assigned"}
        ]))
        .expect_err("duplicate id");
        assert!(err.contains("unique"), "{err}");
        assert!(err.contains("t1"), "{err}");
    }

    #[test]
    fn an_unknown_property_is_refused() {
        let err = parse(&json!([{
            "id": "t1", "title": "A", "tier": "primary", "state": "assigned", "endOn": ["time_limit"]
        }]))
        .expect_err("endOn is not a task field");
        assert!(err.contains("endOn"), "{err}");
        assert!(err.contains("additionalProperties"), "{err}");
    }

    #[test]
    fn a_blank_optional_is_refused_rather_than_stored() {
        let err = parse(&json!([{
            "id": "t1", "title": "A", "tier": "primary", "state": "assigned", "triggerId": "  "
        }]))
        .expect_err("blank triggerId");
        assert!(err.contains("blank"), "{err}");
    }

    #[test]
    fn a_tier_outside_the_vocabulary_is_refused() {
        let err = parse(&json!([{
            "id": "t1", "title": "A", "tier": "main", "state": "assigned"
        }]))
        .expect_err("main is not a tier");
        assert!(err.contains("main"), "{err}");
        assert!(err.contains("primary"), "{err}");
    }

    #[test]
    fn not_an_array_is_refused() {
        let err = parse(&json!({"id": "t1"})).expect_err("object is not an array");
        assert!(err.contains("array"), "{err}");
    }

    #[test]
    fn tasks_is_registered_and_not_document_modelled() {
        assert!(
            is_authored_block("tasks"),
            "T-936.2's row must be in AUTHORED_BLOCKS or the carrier never emits it"
        );
        assert!(
            !crate::mission::extensions::DOCUMENT_OWNED_BLOCKS.contains(&"tasks"),
            "tasks is optional — it rides ExtensionBlocks, it does not get a typed ModMission field"
        );
    }

    #[test]
    fn copy_promotes_tasks_verbatim_from_the_environment_bag() {
        let tasks = three_tiers();
        let env = json!({"weather": "clear", "tasks": tasks});
        let mut dst = Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert!(
            copied.contains(&"tasks"),
            "tasks must leave the bag: {copied:?}"
        );
        assert_eq!(dst["tasks"], tasks, "verbatim");
        assert!(
            !dst.contains_key("weather"),
            "the bag's own keys stay in the bag"
        );
    }

    #[test]
    fn from_payload_carries_a_valid_block_and_refuses_a_malformed_one() {
        let (carried, refusals) = ExtensionBlocks::from_payload(&json!({"tasks": three_tiers()}));
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("tasks"), Some(&three_tiers()));

        let (carried, refusals) = ExtensionBlocks::from_payload(&json!({
            "tasks": [{"id": "t1", "title": "A", "tier": "primary", "state": "nope"}]
        }));
        assert!(carried.is_empty(), "a refused block must not ride the wire");
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0].0, "tasks");
        assert!(refusals[0].1.contains("nope"), "{}", refusals[0].1);
    }

    /// Acceptance half we own: `copy_authored_blocks` + `compile_payload` put a three-tier
    /// `tasks` array on the payload root. `flatten.rs` is T-682's file; `authored_blocks_root`
    /// currently copies only `winConditions`, so this slice must not teach flatten a `tasks`
    /// field. The carrier (`ExtensionBlocks::from_payload`) is what emits once that root is
    /// handed a `tasks` key.
    #[test]
    fn a_three_tier_mission_copies_to_the_payload_root() {
        let tasks = three_tiers();
        let p = compile_env_with_tasks(&tasks);
        assert_eq!(
            p["tasks"], tasks,
            "AUTHORED_BLOCKS must promote tasks out of the env bag: {p:#}"
        );
        assert_eq!(p["tasks"].as_array().expect("array").len(), 3);
        assert_eq!(p["tasks"][0]["tier"], "primary");
        assert_eq!(p["tasks"][1]["tier"], "secondary");
        assert_eq!(p["tasks"][2]["tier"], "optional");

        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("tasks"), Some(&tasks));
    }

    #[test]
    fn an_unauthored_payload_still_omits_the_tasks_key() {
        let p = compile_payload(
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("tasks").is_none(),
            "parity: no tasks authored ⇒ no tasks key: {p:#}"
        );
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(carried.get("tasks").is_none());
    }

    /// The unlisted-key witness T-936.1 left in compile.rs (`tasks` as the dummy) goes red
    /// once this slice registers the row. The assertion lives HERE now, using `audio`
    /// (T-936.5), so compile.rs can stay at merge-base.
    #[test]
    fn an_unlisted_environment_key_is_not_promoted() {
        let env = json!({"weather": "clear", "audio": {"emitters": []}});
        let mut dst = Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert!(!copied.contains(&"audio"), "{copied:?}");
        assert!(
            !dst.contains_key("audio"),
            "an unlisted key stays parked: {dst:?}"
        );
    }
}
