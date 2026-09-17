//! Persistence.

use super::*;
use anyhow::Context;

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

pub fn parse(text: &str) -> Result<WaveLock> {
    toml::from_str(text).context("parse wave.lock (TOML)")
}

/// The DidNotRun refusal every reader shares: an absent lock is a refusal, never an empty plan.
pub fn missing_lock_error(root: &Path) -> String {
    format!(
        "{} missing — DidNotRun: run `cargo xtask wave repack`. A missing lock is a refusal, never an empty plan.",
        lock_path(root).display()
    )
}

pub fn load(root: &Path) -> Result<WaveLock> {
    let p = lock_path(root);
    if !p.is_file() {
        bail!("{}", missing_lock_error(root));
    }
    let text = std::fs::read_to_string(&p).with_context(|| p.display().to_string())?;
    parse(&text)
}

pub fn write(root: &Path, lock: &WaveLock) -> Result<()> {
    let p = lock_path(root);
    std::fs::write(&p, render(lock)?).with_context(|| p.display().to_string())?;
    Ok(())
}

/// Refresh the lock after a registry status write (the `ticket ship` / `ticket set-status`
/// bookkeeping hook, and `platform wave land`'s final step). Same writer as `wave repack`;
/// wave 0 baseline carries over from the previous lock when one exists.
pub fn repack_quiet(root: &Path) -> Result<WaveLock> {
    if crate::wave_lock::legacy_plan::any_tsv_present(root) {
        return migrate_from_tsv(root);
    }
    // The previous committed lock is BOTH carries: its wave 0 is the parked baseline, and its
    // open waves + pending entries feed the emptied carry (T-925, `carry_emptied`).
    let prev = load(root).ok(); // first-ever compile on a lockless tree carries nothing
    let baseline: BTreeSet<String> = prev
        .as_ref()
        .map(|p| p.tickets_in_wave(0).into_iter().collect())
        .unwrap_or_default();
    let lock = compile(root, &baseline, prev.as_ref())?;
    write(root, &lock)?;
    Ok(lock)
}

/// [`repack_quiet`] with a pending-close reservation (`reserved_entry`). Same writer, same
/// carries; the reservation is the one input the tickets themselves can no longer supply.
pub fn repack_reserving(root: &Path, reserve: &[String]) -> Result<WaveLock> {
    if crate::wave_lock::legacy_plan::any_tsv_present(root) {
        bail!(
            "wave repack --reserve: this tree still has the wave-plan TSVs, so the repack is the              one-shot migration — migrate first, then reserve"
        );
    }
    let prev = load(root).ok();
    let baseline: BTreeSet<String> = prev
        .as_ref()
        .map(|p| p.tickets_in_wave(0).into_iter().collect())
        .unwrap_or_default();
    let lock = compile_reserving(root, &baseline, prev.as_ref(), reserve)?;
    write(root, &lock)?;
    Ok(lock)
}

/// One-shot cutover: waves 1+ are the committed TSV open-id groups (platform then mod, label
/// groups in file order, relabeled 1..N), wave 0 is every non-dispatchable TSV id plus every
/// parked ticket. Deletes both TSVs — the lock and the deletion ride the same commit.
pub(super) fn migrate_from_tsv(root: &Path) -> Result<WaveLock> {
    let views = load_views(root)?;
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let rows = crate::wave_lock::legacy_plan::working_tree_rows(root)?;

    let mut label_order: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut wave0_tsv: BTreeSet<String> = BTreeSet::new();
    for (label, id) in &rows {
        let disp = by_id.get(id.as_str()).map(|v| v.dispatchable()) == Some(true);
        if disp {
            if !groups.contains_key(label.as_str()) {
                label_order.push(label.as_str());
            }
            groups.entry(label.as_str()).or_default().push(id.as_str());
        } else {
            wave0_tsv.insert(id.clone());
        }
    }
    let open: Vec<Vec<String>> = label_order
        .iter()
        .map(|l| groups[l].iter().map(|s| s.to_string()).collect())
        .collect();

    // T-914: the migration arm numbers from the ledger too. The T-912.2 cutover relabeled the
    // TSV groups 1..N, but that lock has already landed in real history — this arm is only
    // reachable on a hypothetical TSV-bearing tree, and numbering it off the ledger keeps the
    // base check green there instead of red-on-arrival. No emptied carry: the TSV era
    // predates the section, and a TSV-bearing tree has no previous LOCK to carry from.
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
    crate::wave_lock::legacy_plan::delete_tsvs(root)?;
    Ok(lock)
}

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
    // T-925: a pending emptied wave is the thing `wave --close` acts on — say so whenever one
    // exists; byte-identical summary when none does.
    match lock.emptied.len() {
        0 => base,
        k => format!("{base}, {k} emptied wave(s) pending close"),
    }
}

pub fn cmd_repack(root: &Path, reserve: &[String]) -> Result<u8> {
    let migrated = crate::wave_lock::legacy_plan::any_tsv_present(root);
    let lock = if reserve.is_empty() {
        repack_quiet(root)?
    } else {
        repack_reserving(root, reserve)?
    };
    if migrated {
        println!("wave.lock: migrated from the committed wave-plan TSVs (both deleted)");
    }
    if let Some(e) = lock.emptied.last().filter(|_| !reserve.is_empty()) {
        println!(
            "reserved wave {} for {} ticket(s) pending close: {}",
            e.n,
            e.tickets.len(),
            e.tickets.join(" ")
        );
    }
    println!("wrote {}: {}", LOCK_REL, summary(&lock));
    Ok(0)
}
