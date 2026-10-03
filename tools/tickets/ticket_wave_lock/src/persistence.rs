//! Reading, writing and repacking the wave lock file.
//!
//! **Role:** renders a [`WaveLock`] to TOML ([`render`]) and parses it back ([`parse`]), loads
//! and writes `.ai/tickets/wave.lock` ([`load`], [`write()`]), and runs the repack — the compile
//! over the previous lock's carries followed by the write ([`repack_quiet`],
//! [`repack_reserving`], [`cmd_repack`]) — including the one-shot build of a first lock from the
//! archived wave plans.
//! **Position:** between the compiler and the callers: `ticket_registry` repacks through
//! [`repack_quiet`] after a status write, xtask's `wave repack` runs [`cmd_repack`], and every
//! reader loads through [`load`].
//! **Signals & state:** none in memory; reads and writes the lock file under the checkout root,
//! prints the repack summary to stdout.
//! **Invariants:** the repack is the lock's only writer; a missing lock is a refusal
//! ([`missing_lock_error`]), never an empty plan; the render is deterministic, starts with the
//! generated-file header and ends with a newline.

use crate::compiler::{assemble, ledger_base, ledger_floor};
use crate::error::{Error, Result, ResultExt};
use crate::model::HEADER;

use crate::{
    TicketView, WaveLock, compile, compile_reserving, load_views, lock_path, max_concurrent,
};
use repository_layout::WAVE_LOCK;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use ticket_model::TicketId;

/// The lock as file text: the generated-file header, then the TOML body, ending in a newline.
pub fn render(lock: &WaveLock) -> Result<String> {
    let body = toml::to_string(lock).context("render wave.lock")?;
    let mut out = String::with_capacity(HEADER.len() + body.len());
    out.push_str(HEADER);
    out.push_str(&body);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Parses lock file text; a TOML error or an unknown key refuses with the parse failure as the
/// cause.
pub fn parse(text: &str) -> Result<WaveLock> {
    toml::from_str(text).context("parse wave.lock (TOML)")
}

/// The `DidNotRun` refusal text every reader shares for an absent lock, naming the path and the
/// repack that creates it.
pub fn missing_lock_error(root: &Path) -> String {
    format!(
        "{} missing — DidNotRun: run `cargo xtask wave repack`. A missing lock is a refusal, never an empty plan.",
        lock_path(root).display()
    )
}

/// Reads and parses the lock under `root`; an absent file is the [`missing_lock_error`] refusal.
pub fn load(root: &Path) -> Result<WaveLock> {
    let p = lock_path(root);
    if !p.is_file() {
        return Err(Error::msg(missing_lock_error(root)));
    }
    let text = std::fs::read_to_string(&p).with_context(|| p.display().to_string())?;
    parse(&text)
}

/// Renders `lock` and writes it to `.ai/tickets/wave.lock` under `root`.
pub fn write(root: &Path, lock: &WaveLock) -> Result<()> {
    let p = lock_path(root);
    std::fs::write(&p, render(lock)?).with_context(|| p.display().to_string())?;
    Ok(())
}

/// Recompiles and writes the lock without printing a summary: the repack `ticket ship` and
/// `ticket set-status` run after a status write and `platform wave land` runs as its final step.
/// The previous lock's wave 0 is the parked baseline and its open and pending waves feed the
/// emptied carry; a tree without a lock carries nothing. A tree that still holds an archived
/// wave plan builds its first lock from it instead.
pub fn repack_quiet(root: &Path) -> Result<WaveLock> {
    if crate::archived_wave_plans::any_tsv_present(root) {
        return migrate_from_tsv(root);
    }
    // The previous lock feeds both carries: its wave 0 is the parked baseline, and its open
    // waves and pending entries feed the emptied carry.
    let prev = load(root).ok(); // a lockless tree carries nothing
    let baseline: BTreeSet<TicketId> = prev
        .as_ref()
        .map(|p| p.tickets_in_wave(0).into_iter().collect())
        .unwrap_or_default();
    let lock = compile(root, &baseline, prev.as_ref())?;
    write(root, &lock)?;
    Ok(lock)
}

/// [`repack_quiet`] with a pending-close reservation: `reserve` names the shipped tickets of a
/// wave that dissolved one ticket at a time, recorded as an `[[emptied]]` entry. Refuses on a
/// tree that still holds an archived wave plan, and when an id is unknown, repeated, not shipped
/// or cancelled, or already pending close.
pub fn repack_reserving(root: &Path, reserve: &[TicketId]) -> Result<WaveLock> {
    if crate::archived_wave_plans::any_tsv_present(root) {
        return Err(Error::msg(
            "wave repack --reserve: this tree still has the wave-plan TSVs, so the repack is the              one-shot migration — migrate first, then reserve",
        ));
    }
    let prev = load(root).ok();
    let baseline: BTreeSet<TicketId> = prev
        .as_ref()
        .map(|p| p.tickets_in_wave(0).into_iter().collect())
        .unwrap_or_default();
    let lock = compile_reserving(root, &baseline, prev.as_ref(), reserve)?;
    write(root, &lock)?;
    Ok(lock)
}

/// Builds the first lock from the archived wave plans: the open waves are their dispatchable id
/// groups (platform plan then mod plan, label groups in file order), wave 0 is every
/// non-dispatchable listed id plus every parked ticket. Writes the lock and deletes both plans,
/// so the lock and the deletion land in one commit.
pub(super) fn migrate_from_tsv(root: &Path) -> Result<WaveLock> {
    let views = load_views(root)?;
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let rows = crate::archived_wave_plans::working_tree_rows(root)?;

    let mut label_order: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut wave0_tsv: BTreeSet<TicketId> = BTreeSet::new();
    for (label, id) in &rows {
        let disp = by_id.get(id.as_str()).map(|v| v.dispatchable()) == Some(true);
        if disp {
            if !groups.contains_key(label.as_str()) {
                label_order.push(label.as_str());
            }
            groups.entry(label.as_str()).or_default().push(id.as_str());
        } else {
            wave0_tsv.insert(TicketId::new(id.as_str()));
        }
    }
    let open: Vec<Vec<TicketId>> = label_order
        .iter()
        .map(|l| groups[l].iter().copied().map(TicketId::new).collect())
        .collect();

    // Numbered from the close-marker ledger like every repack, so `wave check`'s base check
    // holds on the result. No emptied carry: a tree with an archived plan has no previous lock
    // to carry from.
    let migration_base = ledger_base(root)?;
    let lock = assemble(
        &views,
        &wave0_tsv,
        open,
        max_concurrent(),
        migration_base,
        ledger_floor(root, migration_base)?,
        Vec::new(),
    );
    write(root, &lock)?;
    crate::archived_wave_plans::delete_tsvs(root)?;
    Ok(lock)
}

/// The one-line count of open tickets, open waves, parked tickets and pending closes that
/// `wave repack` and `wave check` print.
pub(super) fn summary(lock: &WaveLock) -> String {
    let open: usize = lock
        .waves
        .iter()
        .filter(|w| w.n > 0)
        .map(|w| w.tickets.len())
        .sum();
    let n_open = lock.waves.iter().filter(|w| w.n > 0).count();
    let parked = lock
        .waves
        .iter()
        .find(|w| w.n == 0)
        .map(|w| w.tickets.len())
        .unwrap_or(0);
    let base = format!("{open} open ticket(s) in {n_open} wave(s), {parked} parked at wave 0");
    // A pending emptied wave is what `wave --close` acts on, so the summary names any; without
    // one the summary stops at the parked count.
    match lock.emptied.len() {
        0 => base,
        k => format!("{base}, {k} emptied wave(s) pending close"),
    }
}

/// The `wave repack [--reserve <ids>]` command: repacks (reserving when `reserve` is nonempty),
/// prints what it wrote and returns exit code 0.
pub fn cmd_repack(root: &Path, reserve: &[TicketId]) -> Result<u8> {
    let migrated = crate::archived_wave_plans::any_tsv_present(root);
    let lock = if reserve.is_empty() {
        repack_quiet(root)?
    } else {
        repack_reserving(root, reserve)?
    };
    if migrated {
        println!("wave.lock: numbered from the committed wave-plan history");
    }
    if let Some(e) = lock.emptied.last().filter(|_| !reserve.is_empty()) {
        println!(
            "reserved wave {} for {} ticket(s) pending close: {}",
            e.n,
            e.tickets.len(),
            e.tickets.join(" ")
        );
    }
    println!("wrote {}: {}", WAVE_LOCK, summary(&lock));
    Ok(0)
}
