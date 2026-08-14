//! T-917.3 — wall quarantine (`cargo xtask ticket quarantine-walls`): pass 1 of the
//! wall triage, mechanical and byte-reversible (spec §Wall quarantine).
//!
//! One-shot in effect, idempotent by emptiness: for every WORK ticket whose
//! TOML-parsed `summary` exceeds [`SUMMARY_WORD_CAP`] whitespace-split words, the
//! summary moves VERBATIM into `migration_legacy[]` split on newlines only (a
//! single-line wall becomes a one-element array) and `summary` becomes the ticket's
//! existing `title` — human-written, card-length, zero invented prose. Joining
//! `migration_legacy` with `"\n"` must reproduce the original summary bytes exactly;
//! the verb proves that per file twice — in memory before any byte lands, and against
//! the re-read tree after `Corpus::write_back` — while it still holds the original
//! (at commit time that original IS the `git show HEAD^:` content, which is what the
//! T-917.3 acceptance line compares against). A second run finds every carrier
//! already parked (nonempty `migration_legacy` skips) and prints
//! "0 summaries over cap; nothing to do".
//!
//! Deliberate non-moves, both reported:
//!
//! - **Programs.** The spec's quarantine paragraph binds on work tickets; over-cap
//!   PROGRAM summaries (T-916-class walls) are counted and printed as a future note,
//!   never moved — widening the pass to programs is a spec decision, not a slice
//!   liberty.
//! - **Over-cap titles.** If a quarantined ticket's own title exceeds the cap, the
//!   title still becomes the summary verbatim — truncation is FORBIDDEN; the
//!   check-level summary cap exempts exactly the nonempty-`migration_legacy` set, so
//!   these stay green. The count is reported.
//!
//! No sentence-splitting into semantic fields: a semantic field filled by a
//! non-semantic process launders unsorted prose as classified data. Decomposition is
//! Program T (AI batches of 20–30 draining the shrink-only
//! `MIGRATION_LEGACY_PIN` in `crates/tbd-tickets/src/store.rs`).
//!
//! wave.lock byte-neutrality is an in-run tripwire (spec §Migration mechanics):
//! summaries are not lock inputs, so the verb refuses if the lock bytes moved.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tbd_tickets::{Corpus, SUMMARY_WORD_CAP, Ticket};

/// What one quarantine pass did (ids, corpus order).
pub struct QuarantineReport {
    /// Work tickets whose summary moved into `migration_legacy` (the write set).
    pub moved: Vec<String>,
    /// Moved tickets whose TITLE itself exceeds the cap — kept verbatim as summary
    /// (truncate is forbidden; the check exemption covers them).
    pub over_cap_titles: Vec<String>,
    /// Programs whose summary exceeds the cap — reported, never moved (work-only).
    pub program_walls: Vec<String>,
    /// Pre-move summary bytes per moved id — the reversibility oracle.
    pub originals: BTreeMap<String, String>,
}

/// The pure pass over a loaded corpus. Mutates qualifying work tickets in memory and
/// proves `migration_legacy.join("\n") == original summary` byte-exactly per file
/// before returning; the caller lands the bytes via [`Corpus::write_back`].
pub fn quarantine_walls(corpus: &mut Corpus) -> Result<QuarantineReport> {
    let mut report = QuarantineReport {
        moved: Vec::new(),
        over_cap_titles: Vec::new(),
        program_walls: Vec::new(),
        originals: BTreeMap::new(),
    };
    for (id, ticket) in corpus.tickets.iter_mut() {
        match ticket {
            Ticket::Program(p) => {
                if p.summary.split_whitespace().count() > SUMMARY_WORD_CAP {
                    report.program_walls.push(id.clone());
                }
            }
            Ticket::Work(w) => {
                if !w.migration_legacy.is_empty() {
                    // Already quarantined (summary is the title fallback, possibly
                    // itself over the cap) — idempotent by emptiness, never re-park.
                    continue;
                }
                if w.summary.split_whitespace().count() <= SUMMARY_WORD_CAP {
                    continue;
                }
                let original = w.summary.clone();
                let legacy: Vec<String> = original.split('\n').map(str::to_string).collect();
                if legacy.join("\n") != original {
                    bail!(
                        "{id}: newline-join does not reproduce the original summary bytes — refusing the pass"
                    );
                }
                w.migration_legacy = legacy;
                w.summary = w.title.clone();
                if w.title.split_whitespace().count() > SUMMARY_WORD_CAP {
                    report.over_cap_titles.push(id.clone());
                }
                report.originals.insert(id.clone(), original);
                report.moved.push(id.clone());
            }
        }
    }
    Ok(report)
}

/// The verb: load → pass → surgical write → re-read proof → sync-surface regen →
/// wave.lock byte tripwire.
pub fn cmd_quarantine_walls(root: &Path) -> Result<()> {
    let lock_path = root.join(".ai/tickets/wave.lock");
    let lock_before = fs::read(&lock_path).ok();

    let mut corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    let report = quarantine_walls(&mut corpus)?;
    let m = report.moved.len();
    if m == 0 {
        println!("0 summaries over cap; nothing to do");
        return Ok(());
    }
    corpus
        .write_back(&report.moved)
        .map_err(anyhow::Error::msg)?;

    // Re-read proof: the LANDED bytes must reverse to the pre-move summary — and the
    // parked ticket's summary must be its title verbatim.
    let reread = Corpus::load(root).map_err(anyhow::Error::msg)?;
    for id in &report.moved {
        let w = match reread.get(id) {
            Some(Ticket::Work(w)) => w,
            Some(Ticket::Program(_)) | None => bail!("{id}: moved ticket missing after write"),
        };
        let original = report
            .originals
            .get(id)
            .with_context(|| format!("{id}: no recorded original"))?;
        if &w.migration_legacy.join("\n") != original {
            bail!("{id}: re-read migration_legacy does not join back to the original summary");
        }
        if w.summary != w.title {
            bail!("{id}: re-read summary is not the title fallback");
        }
    }

    println!(
        "{m} summaries >{SUMMARY_WORD_CAP} TOML-parsed whitespace-split words moved to migration_legacy (instrument: TOML-parse then split_whitespace().count())"
    );
    println!(
        "{m}/{m} reversible: newline-join byte-compare vs the pre-move summary per file (asserted in memory before write and against the re-read tree)"
    );
    println!(
        "over-cap titles kept verbatim as summary — truncate forbidden, check exemption covers them ({}): {}",
        report.over_cap_titles.len(),
        if report.over_cap_titles.is_empty() {
            "none".to_string()
        } else {
            report.over_cap_titles.join(", ")
        }
    );
    println!(
        "future note — {} program summaries over the cap, NOT moved (quarantine is work-only per spec §Wall quarantine): {}",
        report.program_walls.len(),
        if report.program_walls.is_empty() {
            "none".to_string()
        } else {
            report.program_walls.join(", ")
        }
    );
    println!("ratchet: MIGRATION_LEGACY_PIN in crates/tbd-tickets/src/store.rs must equal {m}");

    // Regenerate the sync surface (docs/TICKET_*.md ×5 + MILESTONES + queue.json +
    // CLAUDE marker) — sync.rs copies summaries verbatim, so the views change in the
    // same commit as the quarantine (the parity class the spec pins).
    let registry = crate::registry::load_registry(root)?;
    crate::sync::cmd_sync(root, &registry)?;

    let lock_after = fs::read(&lock_path).ok();
    if lock_before != lock_after {
        bail!(
            ".ai/tickets/wave.lock bytes changed — summaries are not lock inputs; the pass perturbed something it must not"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tbd_tickets::{Domain, ScopeV2, Status, WorkTicket, render_ticket_toml};

    fn scratch_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tbd-quarantine-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch");
        fs::write(dir.join(".ai/tickets/scope-vocab.toml"), "[repo.docs]\n").expect("vocab");
        dir
    }

    fn work(id: &str, title: &str, summary: &str) -> Ticket {
        Ticket::Work(WorkTicket {
            id: id.into(),
            title: title.into(),
            summary: summary.into(),
            class: Some("chore".into()),
            status: Status::Idea,
            executor: None,
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: ScopeV2 {
                domain: Domain::Repo,
                layer: "docs".into(),
                component: None,
                surface: vec![],
            },
            user_story: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
            citations: vec![],
            shipped_at: None,
            priority: None,
            created_at: None,
            completed_at: None,
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec![],
            pack_last: None,
        })
    }

    fn words(n: usize, stem: &str) -> String {
        (1..=n)
            .map(|i| format!("{stem}{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Walls move (multi-line split on newlines, single-line one-element), join
    /// reproduces the original bytes ON DISK, summary becomes the title verbatim
    /// (over-cap titles included, untruncated), short summaries and programs stay
    /// untouched, and a second pass finds nothing.
    #[test]
    fn quarantine_moves_walls_reversibly_then_second_run_is_empty() {
        let root = scratch_root("pass");
        let multiline = format!(
            "{}\n{}\nquoted \"err\" and a \\ path",
            words(30, "a"),
            words(20, "b")
        );
        let long_title = words(45, "t");
        let program_toml = format!(
            "id = \"T-005\"\nkind = \"program\"\ntitle = \"prog\"\nsummary = \"{}\"\nstatus = \"idea\"\nchildren = [\"T-005.1\"]\n",
            words(60, "p")
        );
        let mut c = Corpus::new(&root);
        for t in [
            work("T-001", "multi-line wall", &multiline),
            work("T-002", "short stays", "well under the cap"),
            work("T-003", "single-line wall", &words(41, "s")),
            work("T-004", &long_title, &words(50, "w")),
        ] {
            c.tickets.insert(t.id().to_string(), t);
        }
        let seed: Vec<String> = c.tickets.keys().cloned().collect();
        c.write_back(&seed).expect("seed tree");
        fs::write(root.join(".ai/tickets/T-005.toml"), program_toml).expect("seed program");

        let mut corpus = Corpus::load(&root).expect("load scratch");
        let before_short = fs::read_to_string(root.join(".ai/tickets/T-002.toml")).unwrap();
        let report = quarantine_walls(&mut corpus).expect("pass");
        assert_eq!(report.moved, vec!["T-001", "T-003", "T-004"]);
        assert_eq!(
            report.over_cap_titles,
            vec!["T-004"],
            "45-word title is over-cap"
        );
        assert_eq!(
            report.program_walls,
            vec!["T-005"],
            "program wall reported, not moved"
        );
        corpus.write_back(&report.moved).expect("land");

        let reread = Corpus::load(&root).expect("reload");
        for (id, original, want_lines) in [
            ("T-001", multiline.as_str(), 3),
            ("T-003", &words(41, "s") as &str, 1),
        ] {
            let w = match reread.get(id) {
                Some(Ticket::Work(w)) => w,
                _ => panic!("{id} must be work"),
            };
            assert_eq!(
                w.migration_legacy.len(),
                want_lines,
                "{id} split on newlines only"
            );
            assert_eq!(
                w.migration_legacy.join("\n"),
                original,
                "{id} join == original bytes"
            );
            assert_eq!(w.summary, w.title, "{id} summary is the title verbatim");
        }
        let w4 = match reread.get("T-004") {
            Some(Ticket::Work(w)) => w,
            _ => panic!("work"),
        };
        assert_eq!(
            w4.summary, long_title,
            "over-cap title kept verbatim, untruncated"
        );
        assert_eq!(
            fs::read_to_string(root.join(".ai/tickets/T-002.toml")).unwrap(),
            before_short,
            "under-cap ticket is byte-untouched"
        );
        match reread.get("T-005") {
            Some(Ticket::Program(p)) => {
                assert!(p.migration_legacy.is_empty(), "programs are never moved")
            }
            _ => panic!("program"),
        }

        // Second pass: idempotent by emptiness.
        let mut again = Corpus::load(&root).expect("reload");
        let second = quarantine_walls(&mut again).expect("second pass");
        assert!(
            second.moved.is_empty(),
            "second run must find nothing to move"
        );
        assert_eq!(second.program_walls, vec!["T-005"], "program note persists");
        fs::remove_dir_all(&root).unwrap();
    }

    /// The moved files still render canonically: a full render → re-parse of a parked
    /// ticket is value-stable (what keeps the corpus roundtrip gate green).
    #[test]
    fn parked_ticket_renders_canonically() {
        let root = scratch_root("canonical");
        let mut c = Corpus::new(&root);
        let wall = format!(
            "{}\nsecond \"line\" with escapes \\ and | pipes",
            words(41, "x")
        );
        c.tickets.insert("T-001".into(), work("T-001", "t", &wall));
        c.write_back(&["T-001".into()]).expect("seed");
        let mut corpus = Corpus::load(&root).expect("load");
        let report = quarantine_walls(&mut corpus).expect("pass");
        assert_eq!(report.moved, vec!["T-001"]);
        corpus.write_back(&report.moved).expect("land");
        let disk = fs::read_to_string(root.join(".ai/tickets/T-001.toml")).unwrap();
        let reparsed = tbd_tickets::parse_ticket_toml(&disk).expect("landed file parses");
        assert_eq!(
            render_ticket_toml(&reparsed).unwrap(),
            disk,
            "landed bytes are the canonical render"
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
