//! Role: connection types.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::HashMap;
use super::HashSet;
use super::Map;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use yrs::types::ToJson;

/// Domain representation of connection kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// `Sync to` — a symmetric peer relation between two placed things. UNDIRECTED.
    Sync,

    /// `Group to` — `from` joins `to`'s group. Directed; the graph must stay acyclic.
    Group,

    /// `Set Trigger Owner` — `to` owns `from`. Directed; the graph must stay acyclic.
    TriggerOwner,
}

impl ConnectionKind {
    /// Parse the stored/wire token. `None` for anything else — an unknown kind is refused rather than coerced, because coercing would silently turn a typo into a relation the author did not ask for and cannot see the difference of.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sync" => Some(Self::Sync),
            "group" => Some(Self::Group),
            "triggerOwner" => Some(Self::TriggerOwner),
            _ => None,
        }
    }
}

impl ConnectionKind {
    /// The stored token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Group => "group",
            Self::TriggerOwner => "triggerOwner",
        }
    }
}

impl ConnectionKind {
    /// Whether `from`→`to` has a direction. `sync` does not: it is a peer relation, which is why it is normalised at write and excluded from the `CONN-CYCLE` rule (a "cycle" of peers is just a connected component, and flagging it would be noise on a correct graph).
    #[must_use]
    pub const fn is_directed(self) -> bool {
        !matches!(self, Self::Sync)
    }
}

impl ConnectionKind {
    /// Normalise using the supplied domain data.
    #[must_use]
    pub(super) fn normalise(self, from: &str, to: &str) -> (String, String) {
        if self.is_directed() || from <= to {
            (from.to_string(), to.to_string())
        } else {
            (to.to_string(), from.to_string())
        }
    }
}

/// Domain representation of connection row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionRow {
    /// Id.
    pub id: String,

    /// Kind.
    pub kind: String,

    /// From.
    pub from: String,

    /// To.
    pub to: String,
}

/// Domain representation of connection finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionFinding {
    /// Code.
    pub code: &'static str,

    /// Connection id.
    pub connection_id: String,

    /// Detail.
    pub detail: String,
}

/// Pure and taking its inputs explicitly so the rules are tested against hand-built graphs with no yrs document in the way — the FNF v4 warning is specifically that this mechanism's defects hide, and a checker that can only be exercised through a live document is a checker nobody exercises. [`MissionDocCore::connection_findings_json`] is a thin adapter over this.
#[must_use]
pub fn validate_connection_rows(
    rows: &[ConnectionRow],
    known_ids: &HashSet<String>,
) -> Vec<ConnectionFinding> {
    let mut out: Vec<ConnectionFinding> = Vec::new();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();

    for r in rows {
        if ConnectionKind::parse(&r.kind).is_none() {
            out.push(ConnectionFinding {
                code: "CONN-KIND",
                connection_id: r.id.clone(),
                detail: format!("unknown connection kind `{}`", r.kind),
            });
        }
        if !r.from.is_empty() && r.from == r.to {
            out.push(ConnectionFinding {
                code: "CONN-SELF",
                connection_id: r.id.clone(),
                detail: format!("`{}` is connected to itself", r.from),
            });
        }
        for (end, id) in [("from", &r.from), ("to", &r.to)] {
            if id.is_empty() || !known_ids.contains(id) {
                out.push(ConnectionFinding {
                    code: "CONN-DANGLING",
                    connection_id: r.id.clone(),
                    detail: format!("{end} endpoint `{id}` is not a placed entity"),
                });
            }
        }
        let key = (r.kind.clone(), r.from.clone(), r.to.clone());
        if !seen.insert(key) {
            out.push(ConnectionFinding {
                code: "CONN-DUPLICATE",
                connection_id: r.id.clone(),
                detail: format!(
                    "`{}` → `{}` is already connected ({})",
                    r.from, r.to, r.kind
                ),
            });
        }
    }

    out.extend(cycle_findings(rows));
    out.sort_by(|a, b| (a.code, &a.connection_id).cmp(&(b.code, &b.connection_id)));
    out
}

/// Cycle findings using the supplied domain data.
pub(super) fn cycle_findings(rows: &[ConnectionRow]) -> Vec<ConnectionFinding> {
    let mut adj: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for r in rows {
        let directed = ConnectionKind::parse(&r.kind).is_some_and(ConnectionKind::is_directed);
        if !directed || r.from == r.to || r.from.is_empty() || r.to.is_empty() {
            continue;
        }
        adj.entry(r.from.as_str())
            .or_default()
            .push((r.to.as_str(), r.id.as_str()));
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Colour {
        /// Domain representation of grey.
        Grey,

        /// Domain representation of black.
        Black,
    }
    let mut colour: HashMap<&str, Colour> = HashMap::new();
    let mut out: Vec<ConnectionFinding> = Vec::new();
    let mut roots: Vec<&str> = adj.keys().copied().collect();
    roots.sort_unstable();

    for root in roots {
        if colour.contains_key(root) {
            continue;
        }

        let mut stack: Vec<(&str, usize)> = vec![(root, 0)];
        colour.insert(root, Colour::Grey);
        while let Some((node, edge_idx)) = stack.pop() {
            let edges = adj.get(node).map_or(&[][..], Vec::as_slice);
            if edge_idx >= edges.len() {
                colour.insert(node, Colour::Black);
                continue;
            }
            stack.push((node, edge_idx + 1));
            let (target, conn_id) = edges[edge_idx];
            match colour.get(target) {
                Some(Colour::Grey) => out.push(ConnectionFinding {
                    code: "CONN-CYCLE",
                    connection_id: conn_id.to_string(),
                    detail: format!("`{node}` → `{target}` closes an ownership cycle"),
                }),
                Some(Colour::Black) => {}
                None => {
                    colour.insert(target, Colour::Grey);
                    stack.push((target, 0));
                }
            }
        }
    }
    out
}

/// Connection row using the supplied domain data.
pub(super) fn connection_row(id: &str, kind: &str, from: &str, to: &str) -> HashMap<String, Any> {
    let mut row: HashMap<String, Any> = HashMap::new();
    row.insert("id".to_string(), Any::String(id.into()));
    row.insert("kind".to_string(), Any::String(kind.into()));
    row.insert("from".to_string(), Any::String(from.into()));
    row.insert("to".to_string(), Any::String(to.into()));
    row
}

/// Read connection map using the supplied domain data.
pub(super) fn read_connection_map<T: ReadTxn>(
    txn: &T,
    connections: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match connections.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),
                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
    }
}
