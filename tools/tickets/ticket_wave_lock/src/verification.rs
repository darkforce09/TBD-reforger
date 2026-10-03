//! The wave lock check: the committed lock compared with the ticket files and git history.
//!
//! **Role:** lists every way `.ai/tickets/wave.lock` disagrees with a fresh derivation
//! ([`check_as_errors`]) and runs the `wave check` command ([`cmd_check`]).
//! **Position:** re-derives through `compiler`, `parking` and `ticket_views`; `ticket_registry`
//! folds [`check_as_errors`] into `ticket check`, and xtask's `wave check` runs [`cmd_check`].
//! **Signals & state:** none in memory; reads the lock, the ticket files and git history, and
//! [`cmd_check`] prints to stdout and stderr.
//! **Invariants:** the check never writes; each finding names its fix, most of them a repack.
//! Open-wave grouping is the packer's recorded choice, so the check holds the invariants
//! (numbering, disjointness, cap, dispatchability, `pack_last` placement) rather than demanding
//! the exact grouping a fresh compile would produce.

use crate::compiler::{ledger_base, ledger_floor};
use crate::error::{Error, Result};
use crate::parking::{carry_emptied, snapshots, wave_zero};
use crate::persistence::summary;
use crate::{LOCK_VERSION, TicketView, collides, load, load_views};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;
use ticket_model::{StatusName, TicketId};

/// Every finding of the check as one error line each, empty when the lock agrees with the tree;
/// `ticket check` embeds the lines verbatim. A lock or ticket corpus that does not load is a
/// single finding.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let lock = match load(root) {
        Ok(l) => l,
        Err(e) => return vec![ticket_model::error_chain_text(&e)],
    };
    let views = match load_views(root) {
        Ok(v) => v,
        Err(e) => {
            return vec![format!(
                "wave check: {}",
                ticket_model::error_chain_text(&e)
            )];
        }
    };
    let mut errors = Vec::new();

    if lock.version != LOCK_VERSION {
        errors.push(format!(
            "wave.lock version {} != {LOCK_VERSION} — regenerate with `cargo xtask wave repack`",
            lock.version
        ));
    }

    // The recorded ledger base must match a fresh derivation from git history. A close marker
    // landing (or the newest one being disavowed) without a repack leaves every open wave
    // labelled off the old base; this finding drives the close, check, repack loop. It is not a
    // contiguity rule: beyond the strictly increasing check below, open-wave labels are the
    // packer's recorded choice, and hand-written test locks with arbitrary labels stay green.
    let derived_base = match ledger_base(root) {
        Ok(want_base) => {
            if lock.wave_base != want_base {
                errors.push(format!(
                    "wave.lock wave_base {} is stale — the close-marker ledger derives {want_base}: run `cargo xtask wave repack`",
                    lock.wave_base
                ));
            }
            Some(want_base)
        }
        Err(e) => {
            errors.push(format!("wave.lock base derivation refused: {e}"));
            None
        }
    };

    // Wave numbering: strictly increasing, wave 0 first when present, no duplicate n —
    // across the UNION of open-wave labels and pending emptied labels. Emptied
    // labels are RESERVED numbers sitting between wave_base and the open waves (the packer
    // numbers open waves past them), so the one legal ascending order is wave 0, then every
    // pending emptied label, then every open wave; a collision or inversion anywhere in
    // that union is red here.
    let mut ns: Vec<u32> = lock
        .waves
        .iter()
        .filter(|w| w.n == 0)
        .map(|w| w.n)
        .collect();
    ns.extend(lock.emptied.iter().map(|e| e.n));
    ns.extend(lock.waves.iter().filter(|w| w.n > 0).map(|w| w.n));
    let mut sorted = ns.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if ns != sorted {
        errors.push(format!(
            "wave.lock wave numbers (wave 0, pending emptied, open waves) are not strictly increasing: {ns:?}"
        ));
    }

    // Wave 0 recompute against the committed lock's own baseline.
    let baseline: BTreeSet<TicketId> = lock.tickets_in_wave(0).into_iter().collect();
    let expect_zero = wave_zero(&views, &baseline);
    let got_zero = lock.tickets_in_wave(0);
    if expect_zero != got_zero {
        let exp: BTreeSet<&TicketId> = expect_zero.iter().collect();
        let got: BTreeSet<&TicketId> = got_zero.iter().collect();
        let missing: Vec<&str> = exp
            .difference(&got)
            .take(10)
            .map(|id| id.as_str())
            .collect();
        let extra: Vec<&str> = got
            .difference(&exp)
            .take(10)
            .map(|id| id.as_str())
            .collect();
        errors.push(format!(
            "wave.lock wave 0 is stale — missing {missing:?}, extra {extra:?}: run `cargo xtask wave repack`"
        ));
    }

    // Snapshots must equal the tickets, corpus-wide.
    let (owns, deps, last) = snapshots(&views);
    for (name, want, got) in [
        ("owns", &owns, &lock.owns),
        ("depends_on", &deps, &lock.depends_on),
    ] {
        if want != got {
            let mut diffs: Vec<TicketId> = Vec::new();
            for (k, v) in want {
                if got.get(k) != Some(v) {
                    diffs.push(k.clone());
                }
            }
            for k in got.keys() {
                if !want.contains_key(k) {
                    diffs.push(k.clone());
                }
            }
            diffs.sort();
            diffs.dedup();
            diffs.truncate(10);
            let diffs: Vec<&str> = diffs.iter().map(TicketId::as_str).collect();
            errors.push(format!(
                "wave.lock {name} snapshot disagrees with the ticket files on {diffs:?} — run `cargo xtask wave repack`"
            ));
        }
    }
    if last != lock.pack_last {
        errors.push(format!(
            "wave.lock pack_last {:?} disagrees with the ticket files {:?} — run `cargo xtask wave repack`",
            lock.pack_last.iter().map(TicketId::as_str).collect::<Vec<_>>(),
            last.iter().map(TicketId::as_str).collect::<Vec<_>>()
        ));
    }

    // Open waves: every id dispatchable today, no id listed twice anywhere, per-wave owns
    // disjoint, cap respected, pack_last trailing as singletons. Grouping and dependency order
    // are the packer's recorded choice, enforced at repack.
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut seen: HashSet<&str> = got_zero.iter().map(TicketId::as_str).collect();
    let mut past_pack_last = false;
    for w in lock.waves.iter().filter(|w| w.n > 0) {
        if w.tickets.is_empty() {
            errors.push(format!("wave.lock wave {} is empty", w.n));
        }
        if w.tickets.len() > lock.max_concurrent {
            errors.push(format!(
                "wave.lock wave {} holds {} tickets over the recorded cap {}",
                w.n,
                w.tickets.len(),
                lock.max_concurrent
            ));
        }
        let is_last_wave = w.tickets.iter().any(|t| lock.pack_last.contains(t));
        if is_last_wave && w.tickets.len() > 1 {
            errors.push(format!(
                "wave.lock wave {} mixes pack_last with other tickets",
                w.n
            ));
        }
        if past_pack_last && !is_last_wave {
            errors.push(format!(
                "wave.lock wave {} follows a pack_last wave but is not one — pack_last tickets trail",
                w.n
            ));
        }
        past_pack_last |= is_last_wave;
        for t in &w.tickets {
            if !seen.insert(t.as_str()) {
                errors.push(format!("wave.lock lists {t} more than once"));
            }
            match by_id.get(t.as_str()) {
                None => errors.push(format!(
                    "wave.lock wave {} lists {t}, which has no ticket file — run `cargo xtask wave repack`",
                    w.n
                )),
                Some(v) if !v.dispatchable() => errors.push(format!(
                    "wave.lock wave {} lists {t} ({}, executor {}) — not dispatchable; run `cargo xtask wave repack`",
                    w.n,
                    v.status.as_str(),
                    v.executor.as_deref().unwrap_or("claude-code"),
                )),
                Some(_) => {}
            }
        }
        for i in 0..w.tickets.len() {
            for j in (i + 1)..w.tickets.len() {
                let a = by_id.get(w.tickets[i].as_str()).map(|v| v.owns.as_slice());
                let b = by_id.get(w.tickets[j].as_str()).map(|v| v.owns.as_slice());
                if let (Some(a), Some(b)) = (a, b)
                    && collides(a, b)
                {
                    errors.push(format!(
                        "wave.lock wave {}: {} and {} own overlapping paths — never the same wave",
                        w.n, w.tickets[i], w.tickets[j]
                    ));
                }
            }
        }
    }

    // The pending [[emptied]] section. Like the wave 0 baseline, entries carry from the
    // previous lock at repack time and cannot be recomputed from scratch: at check time only the
    // committed lock exists. So this validates invariants: every label above wave_base (at or
    // below means its close marker landed and the repack must drop it), every set nonempty,
    // every listed ticket shipped or cancelled in the current tree (the same meaning the close
    // ceremony re-verifies), and the carry rule as a fixpoint — the writer's own carry with this
    // lock as its previous must reproduce the section. Ascending labels and emptied/open
    // disjointness are held by the numbering check above. An edit inside a frozen set that stays
    // nonempty and all shipped is not detectable from the tree, as with the file-less part of
    // the wave 0 baseline: the lock is the only record of what the wave was.
    for e in &lock.emptied {
        if e.n <= lock.wave_base {
            errors.push(format!(
                "wave.lock emptied wave {} is at or below wave_base {} — its close marker landed: run `cargo xtask wave repack`",
                e.n, lock.wave_base
            ));
        }
        if e.tickets.is_empty() {
            errors.push(format!(
                "wave.lock emptied wave {} lists no tickets — an empty set can never be recorded: run `cargo xtask wave repack`",
                e.n
            ));
        }
        for t in &e.tickets {
            let landed = by_id
                .get(t.as_str())
                .map(|v| matches!(v.status, StatusName::Shipped | StatusName::Cancelled))
                .unwrap_or(false);
            if !landed {
                let what = by_id
                    .get(t.as_str())
                    .map(|v| v.status.as_str())
                    .unwrap_or("no ticket file");
                errors.push(format!(
                    "wave.lock emptied wave {} lists {t} ({what}) — not shipped in the current tree",
                    e.n
                ));
            }
        }
    }
    if let Some(want_base) = derived_base {
        // Recompute with the same floor the packer used: the carry seats pending labels on the
        // ledger floor, so any other floor would report a correct lock.
        let want_floor = ledger_floor(root, want_base).unwrap_or(want_base);
        let want = carry_emptied(Some(&lock), &views, want_base, want_floor);
        if want != lock.emptied {
            errors.push(format!(
                "wave.lock emptied section disagrees with the carry rule — pending labels {:?}, carry derives {:?}: run `cargo xtask wave repack`",
                lock.emptied.iter().map(|e| e.n).collect::<Vec<_>>(),
                want.iter().map(|e| e.n).collect::<Vec<_>>(),
            ));
        }
    }

    errors
}

/// The `wave check` command: prints every finding to stderr and fails with the finding count, or
/// prints the lock summary and returns exit code 0.
pub fn cmd_check(root: &Path) -> Result<u8> {
    let errors = check_as_errors(root);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        return Err(Error::msg(format!(
            "wave check failed ({} error(s))",
            errors.len()
        )));
    }
    let lock = load(root)?;
    println!("wave.lock OK: {}", summary(&lock));
    Ok(0)
}
