//! Packing: the greedy that groups dispatchable tickets into open waves, and the reservation.
//!
//! **Role:** packs the dispatchable tickets into file-disjoint waves in priority order
//! (`greedy_waves`) and turns an explicit reservation into a pending-close entry
//! (`reserved_entry`).
//! **Position:** called by the compiler with the ticket views; its waves are labelled by the
//! compiler and its entry joins the emptied carry from `parking`.
//! **Signals & state:** none; pure functions over the views.
//! **Invariants:** two tickets whose `owns` collide never share a wave; a ticket packs strictly
//! after every dispatchable ticket it depends on; no wave exceeds the cap; `pack_last` tickets
//! trail in single-ticket waves; a dependency cycle among dispatchable tickets refuses the
//! compile.

use crate::error::{Error, Result};
use crate::{LockWave, TicketView, collides};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use ticket_model::{StatusName, TicketId};

/// The pending-close entry for a wave the emptied carry cannot see, or `None` when `reserve` is
/// empty.
///
/// The carry freezes an entry only when one repack sees a previous open wave whose whole ticket
/// set has landed. `ticket ship` repacks after every id, so a wave shipped one ticket at a time
/// never presents that picture: after the first ship the repack regroups the open waves and
/// gives the wave's label to the next batch. Without an entry, `wave --close --tickets` would
/// take a label the lock has reissued to unshipped tickets, and the close check at the marker's
/// parent would refuse it.
///
/// The reservation supplies that membership once, labelled after the `carried` entries above
/// `floor`. The entry then sustains itself: it is a pending entry above the base, so the next
/// carry keeps it and `check_as_errors` reproduces it. The reservation vouches for membership
/// only: every id must exist, appear once, be shipped or cancelled (the same meaning the close
/// re-checks) and not already be pending close, or the call refuses.
pub(super) fn reserved_entry(
    views: &[TicketView],
    reserve: &[TicketId],
    carried: &[LockWave],
    floor: u32,
) -> Result<Option<LockWave>> {
    if reserve.is_empty() {
        return Ok(None);
    }
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for id in reserve {
        if !seen.insert(id.as_str()) {
            return Err(Error::msg(format!(
                "wave repack --reserve: {id} named twice — a wave holds each ticket once"
            )));
        }
        let Some(v) = by_id.get(id.as_str()) else {
            return Err(Error::msg(format!(
                "wave repack --reserve: {id} is not a ticket in this tree — a reservation names \
                 the set a close marker will name, and a marker cannot name a ticket that does \
                 not exist"
            )));
        };
        if !matches!(v.status, StatusName::Shipped | StatusName::Cancelled) {
            return Err(Error::msg(format!(
                "wave repack --reserve: {id} is {:?}, not shipped or cancelled — reserving vouches \
                 for MEMBERSHIP, never for status, and `wave --close` re-checks this against the \
                 tree anyway",
                v.status
            )));
        }
        if let Some(e) = carried.iter().find(|e| e.tickets.iter().any(|t| t == id)) {
            return Err(Error::msg(format!(
                "wave repack --reserve: {id} is already pending close in wave {} — close that \
                 entry first",
                e.n
            )));
        }
    }
    Ok(Some(LockWave {
        n: floor + 1 + carried.len() as u32,
        tickets: reserve.to_vec(),
    }))
}

/// The greedy over dispatchable candidates sorted by `order` (unordered last), then by
/// [`ticket_model::store::ticket_id_order_key`], each wave holding at most `cap` tickets.
///
/// Returns the packed waves in order; the compiler labels them. A dependency whose target can
/// never pack (neither dispatchable nor shipped or cancelled) is reported in `warnings` and does
/// not gate the dependent. Fails on a dependency cycle among dispatchable tickets.
pub(super) fn greedy_waves(
    views: &[TicketView],
    cap: usize,
    warnings: &mut Vec<String>,
) -> Result<Vec<Vec<TicketId>>> {
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut cands: Vec<&TicketView> = views.iter().filter(|v| v.dispatchable()).collect();
    cands.sort_by(|a, b| {
        a.order
            .unwrap_or(i64::MAX)
            .cmp(&b.order.unwrap_or(i64::MAX))
            .then_with(|| {
                ticket_model::store::ticket_id_order_key(a.id.as_str())
                    .cmp(&ticket_model::store::ticket_id_order_key(b.id.as_str()))
            })
    });
    let disp_ids: HashSet<&str> = cands.iter().map(|v| v.id.as_str()).collect();

    for c in &cands {
        for d in &c.depends_on {
            if disp_ids.contains(d.as_str()) {
                continue; // schedulable — the packer orders around it
            }
            let landed = by_id
                .get(d.as_str())
                .map(|t| matches!(t.status, StatusName::Shipped | StatusName::Cancelled))
                .unwrap_or(false);
            if !landed {
                let what = by_id
                    .get(d.as_str())
                    .map(|t| t.status.as_str())
                    .unwrap_or("no ticket file");
                warnings.push(format!(
                    "{} depends_on {d} ({what}) — target can never pack, edge does not gate",
                    c.id
                ));
            }
        }
    }

    let last: Vec<&TicketView> = cands.iter().copied().filter(|v| v.pack_last).collect();
    let mut remaining: Vec<&TicketView> = cands.iter().copied().filter(|v| !v.pack_last).collect();
    // An edge blocks while its target is a dispatchable candidate not yet packed in an EARLIER
    // wave — `packed` updates between waves, so same-wave dependency is impossible.
    let mut packed: HashSet<TicketId> = HashSet::new();
    let blocked = |v: &TicketView, packed: &HashSet<TicketId>| {
        v.depends_on
            .iter()
            .any(|d| disp_ids.contains(d.as_str()) && !packed.contains(d.as_str()))
    };

    let mut waves: Vec<Vec<TicketId>> = Vec::new();
    while !remaining.is_empty() {
        let mut wave: Vec<TicketId> = Vec::new();
        let mut used: Vec<&[String]> = Vec::new();
        for c in &remaining {
            if blocked(c, &packed) {
                continue;
            }
            if used.iter().any(|u| collides(&c.owns, u)) {
                continue;
            }
            wave.push(c.id.clone());
            used.push(&c.owns);
            if wave.len() >= cap {
                break;
            }
        }
        if wave.is_empty() {
            // Every candidate left collides or waits on a dependency. With nothing free, that is
            // a cycle in the dispatchable graph and the compile refuses; otherwise the first
            // free candidate gets a wave of its own.
            let free: Vec<&TicketView> = remaining
                .iter()
                .copied()
                .filter(|v| !blocked(v, &packed))
                .collect();
            if free.is_empty() {
                let ids: Vec<&str> = remaining.iter().take(8).map(|v| v.id.as_str()).collect();
                return Err(Error::msg(format!(
                    "depends_on deadlock: {ids:?} — check those tickets' depends_on edges"
                )));
            }
            wave.push(free[0].id.clone());
        }
        let picked: HashSet<&str> = wave.iter().map(TicketId::as_str).collect();
        remaining.retain(|v| !picked.contains(v.id.as_str()));
        for id in &wave {
            packed.insert(id.clone());
        }
        waves.push(wave);
    }
    // pack_last tickets trail, one singleton wave each, in candidate order.
    for v in last {
        packed.insert(v.id.clone());
        waves.push(vec![v.id.clone()]);
    }
    Ok(waves)
}
