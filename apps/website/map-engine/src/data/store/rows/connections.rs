//! Role: connections.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::ConnectionKind;
use super::ConnectionRow;
use super::HashSet;
use super::Map;
use super::MissionDocCore;
use super::comment_str;
use super::connection_row;
use super::read_connection_map;
use super::validate_connection_rows;
use yrs::Transact;
use yrs::types::ToJson;

impl MissionDocCore {
    /// Connection rows json using the supplied domain data.
    #[must_use]
    pub fn connection_rows_json(&self) -> String {
        let rows = self.connection_rows();
        let out: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "kind": r.kind,
                    "from": r.from,
                    "to": r.to,
                })
            })
            .collect();
        serde_json::Value::Array(out).to_string()
    }
}

impl MissionDocCore {
    /// The four rules, and why each one is a rule:.
    #[must_use]
    pub fn connection_findings_json(&self) -> String {
        let rows = self.connection_rows();
        let known = self.known_endpoint_ids();
        let out: Vec<serde_json::Value> = validate_connection_rows(&rows, &known)
            .into_iter()
            .map(|f| {
                serde_json::json!({
                    "code": f.code,
                    "connectionId": f.connection_id,
                    "detail": f.detail,
                })
            })
            .collect();
        serde_json::Value::Array(out).to_string()
    }
}

impl MissionDocCore {
    /// Connections json using the supplied domain data.
    #[must_use]
    pub fn connections_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.connections.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// Connection count using the supplied domain data.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.connections.len(&self.doc.transact()) as usize
    }
}

impl MissionDocCore {
    /// * `id`, `from` or `to` empty — an unaddressable or endpoint-less edge. * `kind` not one of `sync` / `group` / `triggerOwner` ([`ConnectionKind::parse`]). * `from == to` — a self-link (`CONN-SELF`). * an existing row already has the same `(kind, from, to)` after normalisation (`CONN-DUPLICATE`). Drawing the same relation twice is not an edit; it is a second row the operator now has to find and delete.
    pub fn add_connection(&self, id: &str, kind: &str, from: &str, to: &str) -> bool {
        if id.is_empty() || from.is_empty() || to.is_empty() {
            return false;
        }
        let Some(kind) = ConnectionKind::parse(kind) else {
            return false;
        };
        if from == to {
            return false;
        }
        let (from, to) = kind.normalise(from, to);
        if self
            .connection_rows()
            .iter()
            .any(|r| r.id != id && r.kind == kind.as_str() && r.from == from && r.to == to)
        {
            return false;
        }
        let mut txn = self.begin();
        self.connections.insert(
            &mut txn,
            id,
            Any::Map(Arc::new(connection_row(id, kind.as_str(), &from, &to))),
        );
        true
    }
}

impl MissionDocCore {
    /// Remove connection using the supplied domain data.
    pub fn remove_connection(&self, id: &str) {
        let mut txn = self.begin();
        self.connections.remove(&mut txn, id);
    }
}

impl MissionDocCore {
    /// Remove connections touching using the supplied domain data.
    pub fn remove_connections_touching(&self, entity_id: &str) -> Vec<String> {
        if entity_id.is_empty() {
            return Vec::new();
        }
        let doomed: Vec<String> = self
            .connection_rows()
            .into_iter()
            .filter(|r| r.from == entity_id || r.to == entity_id)
            .map(|r| r.id)
            .collect();
        if doomed.is_empty() {
            return Vec::new();
        }
        let mut txn = self.begin();
        for id in &doomed {
            self.connections.remove(&mut txn, id.as_str());
        }
        doomed
    }
}

impl MissionDocCore {
    /// Connection rows using the supplied domain data.
    #[must_use]
    pub(super) fn connection_rows(&self) -> Vec<ConnectionRow> {
        let txn = self.doc.transact();
        let mut rows: Vec<ConnectionRow> = self
            .connections
            .iter(&txn)
            .filter_map(|(key, _)| {
                let row = read_connection_map(&txn, &self.connections, key)?;
                Some(ConnectionRow {
                    id: key.to_string(),
                    kind: comment_str(&row, "kind"),
                    from: comment_str(&row, "from"),
                    to: comment_str(&row, "to"),
                })
            })
            .collect();
        rows.sort_by(|a, b| {
            (&a.kind, &a.from, &a.to, &a.id).cmp(&(&b.kind, &b.from, &b.to, &b.id))
        });
        rows
    }
}

impl MissionDocCore {
    /// Known endpoint ids using the supplied domain data.
    #[must_use]
    pub(super) fn known_endpoint_ids(&self) -> HashSet<String> {
        let txn = self.doc.transact();
        let mut out = HashSet::new();
        for map in [
            &self.slots,
            &self.entities,
            &self.vehicles,
            &self.zones,
            &self.triggers,
        ] {
            for (k, _) in map.iter(&txn) {
                out.insert(k.to_string());
            }
        }
        out
    }
}
