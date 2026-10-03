//! Wave 0, the corpus-wide snapshots, and the carry of emptied waves.
//!
//! **Role:** computes wave 0, the parked ledger (`wave_zero`), the `owns`, `depends_on` and
//! `pack_last` snapshots (`snapshots`), and the `[[emptied]]` entries carried from the previous
//! lock (`carry_emptied`).
//! **Position:** called by the compiler when it assembles a lock, and by `verification`, which
//! re-derives the same values to check a committed lock.
//! **Signals & state:** none; pure functions over the views and the previous lock.
//! **Invariants:** wave 0 is the previous wave 0 plus every parked ticket minus every
//! dispatchable one, in id order; snapshots cover the whole corpus, so the writer and the check
//! agree on scope; a frozen emptied ticket set is never recomputed, only its label re-seated on
//! the ledger.

use crate::{LockWave, TicketView, WaveLock};
use std::collections::{BTreeMap, BTreeSet};
use ticket_model::{StatusName, TicketId};

/// The corpus-wide snapshots: every nonempty `owns` list, every nonempty `depends_on` list (both
/// keyed by id), and every `pack_last = true` id in id order. They cover the whole corpus rather
/// than the lock's members, so the writer and the check cannot disagree about scope.
pub(super) type Snapshots = (
    BTreeMap<TicketId, Vec<String>>,
    BTreeMap<TicketId, Vec<String>>,
    Vec<TicketId>,
);

/// Builds the [`Snapshots`] from the views.
pub(super) fn snapshots(views: &[TicketView]) -> Snapshots {
    let mut owns = BTreeMap::new();
    let mut deps = BTreeMap::new();
    let mut last = Vec::new();
    for v in views {
        if !v.owns.is_empty() {
            owns.insert(v.id.clone(), v.owns.clone());
        }
        if !v.depends_on.is_empty() {
            deps.insert(v.id.clone(), v.depends_on.clone());
        }
        if v.pack_last {
            last.push(v.id.clone());
        }
    }
    sort_ids(&mut last);
    (owns, deps, last)
}

/// Sorts ticket ids in [`ticket_model::store::ticket_id_order_key`] order.
fn sort_ids(ids: &mut [TicketId]) {
    ids.sort_by(|a, b| {
        ticket_model::store::ticket_id_order_key(a.as_str())
            .cmp(&ticket_model::store::ticket_id_order_key(b.as_str()))
    });
}

/// Wave 0: `baseline` plus every parked ticket minus every dispatchable one, in
/// [`ticket_model::store::ticket_id_order_key`] order (wave 0 ignores `order`).
///
/// The baseline keeps wave 0 a ledger: an id whose ticket file disappears stays parked instead
/// of vanishing from the plan. Becoming dispatchable is the one way out, because a reopened
/// ticket is open work that must pack, and listing it in both places would contradict itself.
pub(super) fn wave_zero(views: &[TicketView], baseline: &BTreeSet<TicketId>) -> Vec<TicketId> {
    let mut set: BTreeSet<TicketId> = baseline.clone();
    for v in views {
        if v.parked() {
            set.insert(v.id.clone());
        }
    }
    for v in views {
        if v.dispatchable() {
            set.remove(&v.id);
        }
    }
    let mut ids: Vec<TicketId> = set.into_iter().collect();
    sort_ids(&mut ids);
    ids
}

/// The `[[emptied]]` carry: which pending close targets does the new lock hold?
///
/// Carried from `prev`, the previous lock, like the wave 0 baseline ([`crate::repack_quiet`]
/// passes it in; `None` carries nothing):
///
///   * every pending entry of the previous lock survives until its marker lands: an entry whose
///     label is at or below the fresh `wave_base` drops, because its `wave N CLOSED` commit is
///     now in history (the repack after a close is exactly this call);
///   * every previous open wave above the base whose tickets are now all shipped or cancelled
///     empties: its label and exact ticket set freeze into an entry. Shipped or cancelled is the
///     same meaning the close ceremony re-checks, so the repack never records a target the
///     close would refuse on status grounds. A wave with any live ticket records nothing, and a
///     listed id with no ticket file counts as not shipped, so the wave stays open.
///
/// The frozen set is never recomputed or filtered afterwards: it records what the wave was when
/// it emptied, which is what its close marker will name. The entries come back in ascending
/// label order, labelled from `ledger_floor + 1` on.
pub(super) fn carry_emptied(
    prev: Option<&WaveLock>,
    views: &[TicketView],
    wave_base: u32,
    ledger_floor: u32,
) -> Vec<LockWave> {
    let Some(prev) = prev else {
        return Vec::new();
    };
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let landed = |id: &TicketId| {
        by_id
            .get(id.as_str())
            .map(|v| matches!(v.status, StatusName::Shipped | StatusName::Cancelled))
            .unwrap_or(false)
    };
    let mut out: Vec<LockWave> = prev
        .emptied
        .iter()
        .filter(|e| e.n > wave_base)
        .cloned()
        .collect();
    let pending: BTreeSet<u32> = out.iter().map(|e| e.n).collect();
    for w in &prev.waves {
        if w.n <= wave_base || pending.contains(&w.n) || w.tickets.is_empty() {
            continue;
        }
        if w.tickets.iter().all(landed) {
            out.push(w.clone());
        }
    }
    out.sort_by_key(|e| e.n);
    // Relabel from the ledger, keep the set frozen. A pending entry promises that a
    // `wave N CLOSED` marker will be written for its set, and the close check accepts exactly
    // `ledger_floor + 1` as the next number, so a label above that could never close. Without
    // this re-seat, labels would climb by one per emptied wave while no close lands.
    //
    // Order is preserved, so the queue drains oldest first, the only order the +1 window admits.
    // On a ledger closed in order `ledger_floor == wave_base` and every label already equals its
    // new value, so nothing renumbers.
    for (i, e) in out.iter_mut().enumerate() {
        e.n = ledger_floor + 1 + i as u32;
    }
    out
}
