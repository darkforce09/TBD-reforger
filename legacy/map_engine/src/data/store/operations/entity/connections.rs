//! Role: connections.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::slot_rows;

/// Labels are best-effort (`"SL (s0)"` for a slot whose role is known, the bare id otherwise) and are display-only — every VERB here takes the `id`, so a label that cannot be resolved degrades the row's readability and nothing else. An unresolvable endpoint is exactly the `CONN-DANGLING` case the findings list flags by id, which is why the label does not try to hide it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionListRow {
    /// Id.
    pub id: String,

    /// Kind.
    pub kind: String,

    /// From.
    pub from: String,

    /// To.
    pub to: String,

    /// From label.
    pub from_label: String,

    /// To label.
    pub to_label: String,
}

/// Domain representation of connection finding row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionFindingRow {
    /// Code.
    pub code: String,

    /// Connection id.
    pub connection_id: String,

    /// Detail.
    pub detail: String,
}

/// Asked over `connection_rows_json`, the SAME stable listing the panel renders and the map lane is built from, so "present" means one thing to the verb, to the reconcile and to the operator's eyes. The core's `remove_connection` returns unit and so cannot answer "was it there?"; this takes the answer BEFORE the write instead of inferring it from a count afterwards.
pub fn connection_id_in_doc(core: &MissionDocCore, id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json()) else {
        return false;
    };
    rows.as_array().is_some_and(|a| {
        a.iter()
            .any(|r| r.get("id").and_then(serde_json::Value::as_str) == Some(id))
    })
}

/// Mint connection id using the supplied domain data.
pub fn mint_connection_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.connections_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("conn-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

/// Apply delete_connection to explicit document state.
pub fn delete_connection(core: &MissionDocCore, id: &str) -> bool {
    if !connection_id_in_doc(core, id) {
        return false;
    }
    core.remove_connection(id);
    true
}

/// Apply connection_list to explicit document state.
pub fn connection_list(core: &MissionDocCore) -> Vec<ConnectionListRow> {
    let labels: std::collections::HashMap<String, String> = slot_rows(core)
        .into_iter()
        .map(|r| {
            let label = if r.role.is_empty() {
                r.id.clone()
            } else {
                format!("{} ({})", r.role, r.id)
            };
            (r.id, label)
        })
        .collect();
    let label_of = |id: &str| labels.get(id).cloned().unwrap_or_else(|| id.to_string());
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json()) else {
        return Vec::new();
    };
    rows.as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    let s = |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let (from, to) = (s("from"), s("to"));
                    ConnectionListRow {
                        from_label: label_of(&from),
                        to_label: label_of(&to),
                        id: s("id"),
                        kind: s("kind"),
                        from,
                        to,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Apply connection_findings to explicit document state.
pub fn connection_findings(core: &MissionDocCore) -> Vec<ConnectionFindingRow> {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_findings_json())
    else {
        return Vec::new();
    };
    rows.as_array()
        .map(|a| {
            a.iter()
                .map(|f| {
                    let s = |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    ConnectionFindingRow {
                        code: s("code"),
                        connection_id: s("connectionId"),
                        detail: s("detail"),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Apply complete_connect to explicit document state.
pub fn complete_connect(core: &MissionDocCore, to_id: &str, kind: String, from_id: String) -> bool {
    let id = mint_connection_id(core);
    core.add_connection(&id, &kind, &from_id, to_id)
}
