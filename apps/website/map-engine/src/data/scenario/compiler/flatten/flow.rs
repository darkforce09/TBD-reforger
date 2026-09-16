//! Role: flow.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{HashSet, MOD_MAX_LABEL_CHARS, ModFlow, slug_key};

/// One side's contribution to the radio plan, harvested from the ORBAT as it is built. `callsigns` are the group callsigns in document order — the same strings that reach `orbat.*.groups[].callsign` and `slots[].groupCallsign`, so a net cannot name a squad the compiled document does not contain.
pub(super) struct RadioNetSource {
    /// Faction key.
    pub(super) faction_key: String,
    /// Display name.
    pub(super) display_name: String,
    /// Callsigns.
    pub(super) callsigns: Vec<String>,
}

/// Allocate a net id nothing else in this document has taken.
pub(super) fn unique_net_id(used: &mut HashSet<String>, faction_key: &str, source: &str) -> String {
    let base = format!("net:{faction_key}_{}", slug_key(source, "net"));
    if used.insert(base.clone()) {
        return base;
    }
    for n in 2usize.. {
        let candidate = format!("{base}_{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("the suffix search terminates — some n is always free")
}

/// `TBD_RadioPlan.CapLabel` in the compiler, on a char boundary.
pub(super) fn cap_net_label(label: &str) -> String {
    label.chars().take(MOD_MAX_LABEL_CHARS).collect()
}

/// What a mission that authors nothing runs with — `flow.briefingSeconds`, in **seconds**.
pub const FLOW_DEFAULT_BRIEFING_S: i64 = 600;

/// Unauthored `flow.safeStartSeconds`, in seconds. See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_SAFESTART_S: i64 = 300;

/// Unauthored `flow.timeLimitSeconds`, in seconds. See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_TIMELIMIT_S: i64 = 5400;

/// Unauthored `flow.jip`. See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_JIP: &str = "until_safestart_end";

/// The `jip` domain, pinned to `mission.schema.json#/$defs/flow/properties/jip` — three values.
pub(super) const JIP_VALUES: [&str; 3] = ["disabled", "until_safestart_end", "always"];

/// One authored duration out of the payload's top-level `environment` bag, or `default`.
pub(super) fn authored_flow_seconds(env: &serde_json::Value, key: &str, default: i64) -> i64 {
    env.get(key)
        .and_then(serde_json::Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(default)
}

/// Was `key` AUTHORED, as opposed to defaulted? Shares [`authored_flow_seconds`]'s filter on purpose: "absent", "not a number" and "negative" must mean the same thing to both, or a caller can be told a value it never wrote disagrees with it.
pub(super) fn flow_seconds_authored(env: &serde_json::Value, key: &str) -> bool {
    env.get(key)
        .and_then(serde_json::Value::as_i64)
        .filter(|n| *n >= 0)
        .is_some()
}

/// The authored `flow.jip`, or [`FLOW_DEFAULT_JIP`]. Mirrors `eden_env::read_flow_jip`'s filter: a value outside [`JIP_VALUES`] is treated as unauthored rather than forwarded.
pub(super) fn authored_flow_jip(env: &serde_json::Value) -> String {
    env.get("jip")
        .and_then(serde_json::Value::as_str)
        .filter(|v| JIP_VALUES.contains(v))
        .unwrap_or(FLOW_DEFAULT_JIP)
        .to_string()
}

/// Derive flow using the supplied domain data.
pub(super) fn derive_flow(env: &serde_json::Value) -> ModFlow {
    ModFlow {
        briefing_seconds: authored_flow_seconds(env, "briefingSeconds", FLOW_DEFAULT_BRIEFING_S),
        safe_start_seconds: authored_flow_seconds(
            env,
            "safeStartSeconds",
            FLOW_DEFAULT_SAFESTART_S,
        ),
        time_limit_seconds: authored_flow_seconds(
            env,
            "timeLimitSeconds",
            FLOW_DEFAULT_TIMELIMIT_S,
        ),
        jip: authored_flow_jip(env),
    }
}
