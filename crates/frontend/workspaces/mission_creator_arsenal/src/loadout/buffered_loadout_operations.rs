//! Plan buffered loadout writes and report committed results.

use super::*;
use deterministic_random::SplitMix64;

/* ═════ buffered loadout Copy, Apply, and Remove Everything ═════ */

/// Apply draws one buffered loadout per target entity. The draw is:
///
/// * **Mixed, not counted.** The seed and the ordinal pass through
///   [`SplitMix64::finalise`], an avalanche mix rather than a source of entropy, so seeds and
///   ordinals that differ by 1 produce draws that differ everywhere: a fair die, not a counter.
/// * **Uniform** over the buffer. The mix is scaled by a widening multiply
///   (`(r × len) >> 64`) rather than `r % len`, so the buckets are equal-sized by construction
///   instead of equal-to-within-a-modulo-bias.
/// * **Independent per entity.** `ordinal` is the target's index in the selection, mixed into the
///   stream separately, so ten entities get ten draws — not one draw applied ten times. Two targets
///   landing on the same source is a legitimate outcome of a fair die, not a bug.
/// * **Deterministic given `(seed, ordinal, len)`, and therefore reproducible.** This is the half
///   that makes the feature reasonable to reason about: an assignment is a pure function of a
///   number, so a test can assert an exact distribution, and a bug report that says "the third
///   Apply of the session" replays exactly. [`mission_operations::cargo::next_apply_seed`]
///   advances the session seed by [`SplitMix64::INCREMENT`] once per Apply, so pressing the button twice re-rolls (which is what an
///   author means by random) while the *sequence* stays fixed (which is what a reviewer means by
///   reproducible). Deliberately NO wall clock and no JS RNG: a clock would make the behaviour
///   untestable natively and irreproducible in a bug report, and would buy nothing an author can
///   perceive.
/// * **Degenerate at `len == 1`.** One buffered loadout means every target gets it, with no draw at
///   all — the single-source Copy→Apply case is plain deterministic behaviour, and randomness must
///   not be able to make it surprising.
#[must_use]
pub fn buffer_draw(seed: u64, ordinal: u64, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let r = SplitMix64::finalise(
        seed ^ SplitMix64::finalise(ordinal.wrapping_add(SplitMix64::INCREMENT)),
    );
    let wide = u128::from(r) * u128::try_from(len).unwrap_or(1);
    usize::try_from(wide >> 64).unwrap_or(0)
}

/// Runs compatibility, attachment, and capacity checks for an incoming loadout.
/// Import and buffered Apply use the same gate. Unworn cargo remains advisory because the target kit may provide a garment.
pub(super) fn loadout_rule_refusals(
    picks: &HashMap<String, String>,
    cargo: &[mission_operations::cargo_rules::CargoRow],
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Vec<rules::RowError> {
    let mut refusals = validate_loadout(picks, feed.ready_graph(), feed.status);
    refusals.extend(attachment_errors(picks, feed));
    refusals.extend(rules::cargo_capacity_errors(
        picks,
        cargo,
        &index_by_name(items),
    ));
    refusals
}

/// Validates every buffered source before Apply draws any loadout.
/// Each finding names its source entity so the result is independent of random selection.
#[must_use]
pub fn buffer_refusals(
    buffer: &[BufferedLoadout],
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Vec<rules::RowError> {
    let mut out = Vec::new();
    for entry in buffer {
        let picks = loadout_to_picks(entry.loadout_json.as_deref());
        let (cargo, _present) =
            mission_operations::cargo_rules::cargo_from_loadout(entry.loadout_json.as_deref());
        out.extend(
            loadout_rule_refusals(&picks, &cargo, items, feed)
                .into_iter()
                .map(|e| rules::RowError {
                    key: e.key,
                    message: format!("Buffered loadout from {} — {}", entry.source_id, e.message),
                }),
        );
    }
    out
}

/// Plans one buffered loadout write per selected target.
/// The plan is all-or-nothing and copies valid persisted bytes without reserializing them.
pub fn plan_apply(
    targets: &[String],
    buffer: &[BufferedLoadout],
    seed: u64,
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> crate::error::Result<Vec<LoadoutWrite>> {
    if targets.is_empty() || buffer.is_empty() {
        return Ok(Vec::new());
    }
    let refusals = buffer_refusals(buffer, items, feed);
    if !refusals.is_empty() {
        return Err(crate::error::Error::LoadoutRefused(refusals));
    }
    let mut writes = Vec::with_capacity(targets.len());
    for (ordinal, target) in targets.iter().enumerate() {
        let src = &buffer[buffer_draw(seed, ordinal as u64, buffer.len())];
        writes.push(LoadoutWrite {
            target_id: target.clone().into(),
            source_id: Some(src.source_id.clone()),
            loadout_json: src.loadout_json.clone(),
        });
    }
    Ok(writes)
}

/// Builds a bare loadout with an explicit empty cargo array.
/// The empty array records that the author cleared cargo, preventing defaults from returning.
#[must_use]
pub fn stripped_loadout() -> String {
    let mut wear = serde_json::Map::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_none()) {
        wear.insert(row.key.to_string(), serde_json::Value::Null);
    }
    serde_json::json!({
        "version": 2,
        "wear": wear,
        "weapons": [],
        "cargo": [],
    })
    .to_string()
}

/// Plans the stripped loadout for every selected target.
#[must_use]
pub fn plan_remove(targets: &[String]) -> Vec<LoadoutWrite> {
    targets
        .iter()
        .map(|id| LoadoutWrite {
            target_id: id.clone().into(),
            source_id: None,
            loadout_json: Some(stripped_loadout()),
        })
        .collect()
}

/// Runs the history tail only when the document accepts the write.
/// Returns the acknowledgement so the caller can report a refused write.
pub fn commit_one_write(commit: impl FnOnce() -> bool, tail: impl FnOnce()) -> bool {
    let took = commit();
    if took {
        tail();
    }
    took
}

/// The Copy receipt. Counts the bare sources out loud: buffering an entity with no loadout is legal
/// and useful, but an author who selected forty soldiers and copied forty bare kits should be told
/// before they Apply, not after.
#[must_use]
pub fn copy_receipt(buffer: &[BufferedLoadout]) -> String {
    let bare = buffer.iter().filter(|b| b.loadout_json.is_none()).count();
    let mut line = format!(
        "Copied {} loadout(s) to the buffer. Apply writes one of them to each selected entity, picked at random.",
        buffer.len()
    );
    if bare > 0 {
        line.push_str(&format!(
            " {bare} of them carry no loadout at all — applying one of those leaves that entity bare."
        ));
    }
    line
}

/// The Apply receipt — built from `commits` (what the document took), never from the plan length.
/// It states the undo cost in the same breath, because N-presses-to-undo is a thing the author is
/// about to need and the only place they can learn it is here.
#[must_use]
pub fn apply_receipt(planned: usize, buffer_len: usize, commits: usize) -> String {
    let mut line = format!(
        "Applied {commits} loadout(s), drawn at random from a {buffer_len}-loadout buffer. \
         That is {commits} undo step(s) — one per entity, because there is no atomic multi-entity \
         loadout write (T-732), so Ctrl+Z {commits} times to put it back."
    );
    if commits != planned {
        line.push_str(&format!(
            " WARNING: {planned} write(s) were planned and {commits} reached the document."
        ));
    }
    line
}

/// The Remove Everything receipt. Says the anti-reseed half out loud — an author who strips cargo
/// needs to know it will not silently return, and that promise is the whole reason
/// [`stripped_loadout`] emits `cargo: []`.
#[must_use]
pub fn remove_receipt(planned: usize, commits: usize) -> String {
    let mut line = format!(
        "Stripped {commits} entity(ies) — every wear row, weapon and cargo row cleared, and cargo \
         stays cleared (no default re-seed). That is {commits} undo step(s), one per entity (T-732)."
    );
    if commits != planned {
        line.push_str(&format!(
            " WARNING: {planned} write(s) were planned and {commits} reached the document."
        ));
    }
    line
}
