//! Parking.

use super::*;

/// Corpus-wide snapshots: every nonempty `owns` / `depends_on`, every `pack_last = true`.
/// Corpus-wide (not lock-member-scoped) so the writer and the checker cannot disagree about
/// which tickets are in scope.
pub(super) type Snapshots = (
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, Vec<String>>,
    Vec<String>,
);

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
    last.sort();
    (owns, deps, last)
}

/// Wave 0 = baseline ∪ parked − dispatchable, sorted by id (wave 0 ignores `order`).
///
/// The baseline union is what keeps wave 0 a LEDGER: an id whose ticket file later disappears
/// stays parked instead of silently vanishing from the plan. The dispatchable subtraction is
/// the one legal exit — a reopened ticket is open work and must pack, and listing it in both
/// halves would make the lock disagree with itself.
pub(super) fn wave_zero(views: &[TicketView], baseline: &BTreeSet<String>) -> Vec<String> {
    let mut set: BTreeSet<String> = baseline.clone();
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
    set.into_iter().collect()
}

/// T-925 — the `[[emptied]]` carry: which pending close targets does the new lock hold?
///
/// Same previous-lock carry class as the wave-0 baseline ([`repack_quiet`] feeds the committed
/// lock in; a lockless tree carries nothing):
///
///   * every pending entry of the previous lock survives UNTIL ITS MARKER LANDS — an entry
///     whose label is at or below the fresh ledger base drops, because that label's
///     `wave N CLOSED` marker is now in history (the post-close repack is exactly this call);
///   * every previous OPEN wave above the base whose ticket set is now entirely
///     shipped/cancelled EMPTIES: its label and its exact ticket set freeze into an entry.
///     Shipped-or-cancelled mirrors `Registry::is_shipped` — the same "done" the close
///     ceremony's all-shipped validation re-checks against the tree — so the repack can never
///     record a target close would refuse on status grounds. A wave with any live ticket
///     records NOTHING (partial ships leave no trace), and a listed id with no ticket file is
///     not shipped — fail-closed, the wave stays un-emptied.
///
/// The frozen set is never recomputed or filtered afterwards: it is the ledger of what the
/// wave WAS when it emptied, which is what its close marker will name. Labels ascend in the
/// output; `check_as_errors` re-verifies that via the union numbering check.
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
    let landed = |id: &String| {
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
    // T-946 — RELABEL FROM THE LEDGER, KEEP THE SET FROZEN. The frozen ticket set is still never
    // recomputed; only the LABEL is, because the label is the half that drifted. A pending entry
    // is a promise that a `wave N CLOSED` marker will be written for this set, and the ceremony's
    // oracle accepts exactly `ledger_floor + 1` — so a label above that is a promise the ceremony
    // is structurally unable to keep. Measured 2026-09-05: entries ratcheted one label per
    // emptied wave (a wave that empties reserves its label, the next open wave numbers past it)
    // while no close ever landed to spend one, reaching 247 against a ledger whose highest claim
    // was 235. `wave --close` refused forever, in both directions.
    //
    // Ledger ORDER is preserved (the queue still drains oldest-first, which is the only order the
    // oracle's +1 window admits); only the numbers are re-seated onto the ledger. On a healthy
    // ledger `ledger_floor == wave_base` and every label is already its own new value, so this
    // renumbers nothing.
    for (i, e) in out.iter_mut().enumerate() {
        e.n = ledger_floor + 1 + i as u32;
    }
    out
}
