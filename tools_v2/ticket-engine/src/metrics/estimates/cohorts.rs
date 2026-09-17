//! Cohorts.

use super::*;

// ── Cohorts ────────────────────────────────────────────────────────────────────────────
/// A diff_loc-estimated ticket as cohort material.
#[derive(Debug, Clone)]
pub(super) struct Member {
    pub(super) tokens: u64,
    pub(super) class: Option<String>,
    pub(super) domain: Option<String>,
    pub(super) layer: Option<String>,
}

pub(super) fn attrs_of(t: Option<&Ticket>) -> (Option<String>, Option<String>, Option<String>) {
    match t {
        Some(Ticket::Work(w)) => (
            w.class.clone(),
            Some(w.scope.domain.as_str().to_string()),
            Some(w.scope.layer.clone()),
        ),
        Some(Ticket::Program(p)) => (p.class.clone(), None, None),
        None => (None, None, None),
    }
}

pub(super) fn estimated_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Work(w) => &w.estimated,
        Ticket::Program(p) => &p.estimated,
    }
}

pub(super) fn estimated_mut(t: &mut Ticket) -> &mut Vec<String> {
    match t {
        Ticket::Work(w) => &mut w.estimated,
        Ticket::Program(p) => &mut p.estimated,
    }
}

/// Median over a nonempty set: sort ascending; odd n → middle; even n → floor of
/// the mean of the two middles. Deterministic by construction.
pub(super) fn median(mut vals: Vec<u64>) -> u64 {
    vals.sort_unstable();
    let n = vals.len();
    if n % 2 == 1 {
        vals[n / 2]
    } else {
        (vals[n / 2 - 1] + vals[n / 2]) / 2
    }
}

/// Resolve the cohort for one target: walk the widening ladder from the most
/// specific key the target can express down to all-diff_loc; the first level with
/// ≥3 members wins, and the terminal all-level is used with whatever it has (≥1).
/// Returns the WIDENED key actually used plus the member values; `None` when no
/// diff_loc estimate exists anywhere.
pub(super) fn cohort_for(
    class: Option<&str>,
    domain: Option<&str>,
    layer: Option<&str>,
    members: &[Member],
) -> Option<(CohortKey, Vec<u64>)> {
    type Level<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);
    let levels: Vec<Level> = match (class, domain, layer) {
        (Some(c), Some(d), Some(l)) => vec![
            (Some(c), Some(d), Some(l)),
            (Some(c), Some(d), None),
            (Some(c), None, None),
            (None, None, None),
        ],
        (Some(c), Some(d), None) => vec![
            (Some(c), Some(d), None),
            (Some(c), None, None),
            (None, None, None),
        ],
        (Some(c), None, _) => vec![(Some(c), None, None), (None, None, None)],
        (None, _, _) => vec![(None, None, None)],
    };
    let last = levels.len() - 1;
    for (i, (c, d, l)) in levels.into_iter().enumerate() {
        let vals: Vec<u64> = members
            .iter()
            .filter(|m| {
                c.is_none_or(|c| m.class.as_deref() == Some(c))
                    && d.is_none_or(|d| m.domain.as_deref() == Some(d))
                    && l.is_none_or(|l| m.layer.as_deref() == Some(l))
            })
            .map(|m| m.tokens)
            .collect();
        if vals.len() >= 3 || (i == last && !vals.is_empty()) {
            return Some((
                CohortKey {
                    class: c.map(str::to_string),
                    domain: d.map(str::to_string),
                    layer: l.map(str::to_string),
                },
                vals,
            ));
        }
    }
    None
}
