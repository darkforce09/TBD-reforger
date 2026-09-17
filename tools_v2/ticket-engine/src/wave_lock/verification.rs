//! Verification.

use super::*;

/// All check findings as error strings — embedded verbatim into `ticket check`.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let lock = match load(root) {
        Ok(l) => l,
        Err(e) => return vec![format!("{e:#}")],
    };
    let views = match load_views(root) {
        Ok(v) => v,
        Err(e) => return vec![format!("wave check: {e:#}")],
    };
    let mut errors = Vec::new();

    if lock.version != LOCK_VERSION {
        errors.push(format!(
            "wave.lock version {} != {LOCK_VERSION} — regenerate with `cargo xtask wave repack`",
            lock.version
        ));
    }

    // T-914: the recorded ledger base must match a fresh derivation from git history. A close
    // marker landing (or the newest one being disavowed) without a repack leaves every open
    // wave labeled off the old base — this error is what drives the close → check-red → repack
    // loop. Deliberately NOT a contiguity rule: beyond the strictly-increasing check below,
    // open-wave labels are the packer's recorded choice, and raw-TOML stubs with arbitrary n
    // (mod_wave_tests uses 99) must stay green.
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
    // across the UNION of open-wave labels and pending emptied labels (T-925). Emptied
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
    let baseline: BTreeSet<String> = lock.tickets_in_wave(0).into_iter().collect();
    let expect_zero = wave_zero(&views, &baseline);
    let got_zero = lock.tickets_in_wave(0);
    if expect_zero != got_zero {
        let exp: BTreeSet<&String> = expect_zero.iter().collect();
        let got: BTreeSet<&String> = got_zero.iter().collect();
        let missing: Vec<&&String> = exp.difference(&got).take(10).collect();
        let extra: Vec<&&String> = got.difference(&exp).take(10).collect();
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
            let mut diffs: Vec<String> = Vec::new();
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
            errors.push(format!(
                "wave.lock {name} snapshot disagrees with the ticket files on {diffs:?} — run `cargo xtask wave repack`"
            ));
        }
    }
    if last != lock.pack_last {
        errors.push(format!(
            "wave.lock pack_last {:?} disagrees with the ticket files {:?} — run `cargo xtask wave repack`",
            lock.pack_last, last
        ));
    }

    // Waves 1+: every id dispatchable today, no id listed twice anywhere, per-wave owns
    // disjoint, cap respected, pack_last trailing as singletons. Grouping itself is the
    // packer's recorded choice — see the module header for why dependency order is enforced
    // at repack, not here.
    let by_id: BTreeMap<&str, &TicketView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut seen: HashSet<&str> = got_zero.iter().map(String::as_str).collect();
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

    // ── T-925: the pending [[emptied]] section — repack-recorded close targets ──────────────
    // Like the wave-0 baseline, entries CARRY from the previous lock at repack time (the
    // frozen {label, ticket set} of an open wave whose every ticket had shipped) and CANNOT be
    // recomputed from scratch here — at check time there is no previous lock to read, only the
    // committed one. So this validates INVARIANTS, not a fresh derivation: every label above
    // wave_base (at-or-below means its close marker landed and the repack must drop it), sets
    // nonempty, every listed ticket shipped/cancelled in the current tree (the same "done" the
    // close ceremony re-verifies before writing a marker), and the carry rule itself as a
    // FIXPOINT — running the writer's own carry with this lock as its previous must reproduce
    // the section (that drops landed labels and records any fully-shipped open wave a stale
    // lock left behind). Ascending labels and emptied/open disjointness are already pinned by
    // the union numbering check above. A hand-edit INSIDE a frozen set that stays nonempty and
    // all-shipped is not detectable from the tree, exactly as a hand-edit to the file-less
    // half of the wave-0 baseline is not — the lock is the only record of what the wave was.
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
        // T-946: recompute with the SAME floor the packer used, or `check` reds on a lock that
        // is correct — the carry rule now seats pending labels on the marker ledger.
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

pub fn cmd_check(root: &Path) -> Result<u8> {
    let errors = check_as_errors(root);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        bail!("wave check failed ({} error(s))", errors.len());
    }
    let lock = load(root)?;
    println!("wave.lock OK: {}", summary(&lock));
    Ok(0)
}
