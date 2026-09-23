//! Packing.

use super::*;

/// RESERVE A LABEL FOR A WAVE THE DERIVED CARRY CANNOT SEE.
///
/// `carry_emptied` freezes a pending entry only when ONE repack sees a previous open wave whose
/// whole ticket set is landed. `ticket ship` repacks after every id, so a wave shipped one ticket
/// at a time never presents that picture: after the first ship the repack re-packs the open waves
/// and hands the wave's own label to the NEXT batch, so when the last id ships the label no longer
/// names the set that emptied. Measured 2026-09-06 on wave 240: the
/// batch broke mid-way on a missing `created_at`, the per-id path finished it, and the lock ended
/// with no pending entry and label 240 reissued to three unstarted tickets.
///
/// That is not a cosmetic loss. `wave --close --tickets` takes its label from the lock, the label
/// falls back to `wave_base + 1`, and oracle 2 then reads the plan at the marker's PARENT, finds
/// wave 240 assigned to UNSHIPPED tickets, and refuses every later gate — the shape that cost
/// wave 236 its marker and forced a disavowal.
///
/// So the membership the derived rule lost is supplied once, explicitly, and frozen the same way
/// the carry freezes one. The entry is self-sustaining afterwards: it is a pending entry above the
/// base, so the next `carry_emptied` keeps it and `check_as_errors`' re-derivation reproduces
/// it. THE VOUCHING IS FOR MEMBERSHIP ONLY — every id is still checked shipped-or-cancelled here,
/// against the same `Registry::is_shipped` meaning the close ceremony re-checks, so a reservation
/// can never record a target the close would refuse on status grounds.
pub(super) fn reserved_entry(
    views: &[TicketView],
    reserve: &[String],
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
            bail!("wave repack --reserve: {id} named twice — a wave holds each ticket once");
        }
        let Some(v) = by_id.get(id.as_str()) else {
            bail!(
                "wave repack --reserve: {id} is not a ticket in this tree — a reservation names \
                 the set a close marker will name, and a marker cannot name a ticket that does \
                 not exist"
            );
        };
        if !matches!(v.status, StatusName::Shipped | StatusName::Cancelled) {
            bail!(
                "wave repack --reserve: {id} is {:?}, not shipped or cancelled — reserving vouches \
                 for MEMBERSHIP, never for status, and `wave --close` re-checks this against the \
                 tree anyway",
                v.status
            );
        }
        if let Some(e) = carried.iter().find(|e| e.tickets.iter().any(|t| t == id)) {
            bail!(
                "wave repack --reserve: {id} is already pending close in wave {} — close that \
                 entry first",
                e.n
            );
        }
    }
    Ok(Some(LockWave {
        n: floor + 1 + carried.len() as u32,
        tickets: reserve.to_vec(),
    }))
}

/// The live greedy over dispatchable candidates sorted by `order`, then by
/// [`crate::store::ticket_id_order_key`].
///
/// Returns the packed waves (1..N by position). Dependency edges whose target cannot ever pack
/// (not dispatchable, not shipped/cancelled) are collected into `warnings` and do not gate —
/// see the module header for the reservation case that forces this.
pub(super) fn greedy_waves(
    views: &[TicketView],
    cap: usize,
    warnings: &mut Vec<String>,
) -> Result<Vec<Vec<String>>> {
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut cands: Vec<&TicketView> = views.iter().filter(|v| v.dispatchable()).collect();
    cands.sort_by(|a, b| {
        a.order
            .unwrap_or(i64::MAX)
            .cmp(&b.order.unwrap_or(i64::MAX))
            .then_with(|| {
                crate::store::ticket_id_order_key(a.id.as_str())
                    .cmp(&crate::store::ticket_id_order_key(b.id.as_str()))
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
    let mut packed: HashSet<String> = HashSet::new();
    let blocked = |v: &TicketView, packed: &HashSet<String>| {
        v.depends_on
            .iter()
            .any(|d| disp_ids.contains(d.as_str()) && !packed.contains(d))
    };

    let mut waves: Vec<Vec<String>> = Vec::new();
    while !remaining.is_empty() {
        let mut wave: Vec<String> = Vec::new();
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
            // Everything left is dep-blocked with nothing packable underneath: a cycle in the
            // dispatchable graph. That is a bug worth shouting about, exactly as the TSV-era
            // repack did.
            let free: Vec<&TicketView> = remaining
                .iter()
                .copied()
                .filter(|v| !blocked(v, &packed))
                .collect();
            if free.is_empty() {
                let ids: Vec<&str> = remaining.iter().take(8).map(|v| v.id.as_str()).collect();
                bail!("depends_on deadlock: {ids:?} — check those tickets' depends_on edges");
            }
            wave.push(free[0].id.clone());
        }
        let picked: HashSet<&str> = wave.iter().map(String::as_str).collect();
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
