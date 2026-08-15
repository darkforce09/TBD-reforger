//! T-916.1 — typed corpus store: every `.ai/tickets/T-*.toml`, parents AND children,
//! in one typed map, plus surgical per-file writes.
//!
//! Design authority: `docs/platform/t915_ticketboard_design.md` §Write path. This module
//! replaces the xtask Value round-trip (`load_phase2_tree` → Value mutators →
//! `save_tree`) as the mutation substrate. Three properties are load-bearing:
//!
//! - **Fail-closed load.** Any file that does not parse (or whose id disagrees with its
//!   filename) refuses the WHOLE load, naming the file — no partial corpus ever exists.
//!   A board or op over a partial corpus would silently treat missing tickets as absent,
//!   which is exactly the class of quiet wrongness the DidNotRun philosophy forbids.
//! - **Surgical writes.** [`Corpus::write_back`] touches exactly the ids it is given:
//!   render → re-parse (must succeed AND round-trip equal) → write a dot-prefixed temp
//!   file in the same directory → rename over the target. Re-parse-before-write follows
//!   the `migrate_live_tree` precedent; temp+rename kills torn reads by directory
//!   watchers (which otherwise read half-written TOMLs and flash refusals); the dot
//!   prefix keeps the temp file invisible to every `T-*.toml` glob, including this
//!   module's own loader. The `save_tree` full-rewrite-plus-delete pass is dead here BY
//!   OMISSION: nothing in this module deletes a file it was not explicitly handed
//!   ([`Corpus::delete_files`]), so no mutation can mass-delete children again.
//! - **Children are first-class.** The map holds dotted-id files too, which is what
//!   makes `ops::ship` of a child id resolve at all — the Value path's parents-only
//!   view was the "`ticket ship T-912.2` → Unknown ticket" hole.

use crate::{Ticket, parse_ticket_toml, render_ticket_toml};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// `T-` followed by ASCII digits only — same rule as `xtask::tickets_store::is_parent_id`.
/// Dotted ids (children) are never parents.
pub fn is_parent_id(id: &str) -> bool {
    match id.strip_prefix("T-") {
        Some(rest) => !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

/// Numeric part of a parent id (`T-916` → `916`); `None` for children and non-ids.
pub fn parent_numeric_id(id: &str) -> Option<u64> {
    if !is_parent_id(id) {
        return None;
    }
    id.strip_prefix("T-")?.parse().ok()
}

/// The whole on-disk registry, typed. `tickets` is public on purpose: the store is the
/// shared read substrate for the walks that today each re-glob the directory
/// (`wave_lock::load_views`, `check_open_work_owns`, `slice_collisions::ticket_facts`) —
/// they only read. Mutations should go through `ops`, whose post-image validation is the
/// only path that upholds the "no op may write a corpus its own preflight would refuse"
/// invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corpus {
    root: PathBuf,
    /// Every ticket on disk, keyed by id. Load enforces key == file stem, so
    /// `{id}.toml` is always the backing file.
    pub tickets: BTreeMap<String, Ticket>,
}

impl Corpus {
    /// Empty corpus rooted at `root` (repo root, i.e. the directory containing
    /// `.ai/tickets/`). For scratch corpora in tests and for callers that assemble
    /// tickets programmatically; the live tree goes through [`Corpus::load`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Corpus {
            root: root.into(),
            tickets: BTreeMap::new(),
        }
    }

    /// Repo root this corpus was loaded from (spec-on-disk checks resolve against it).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `.ai/tickets` under the root — the one directory every file lives in.
    pub fn tickets_dir(&self) -> PathBuf {
        self.root.join(".ai/tickets")
    }

    /// Load EVERY `.ai/tickets/T-*.toml` under `root` — parents and children alike
    /// (deliberately NOT `load_phase2_tree`, which is parents-only). Fail-closed: the
    /// first file that fails to parse refuses the whole load with an error naming that
    /// file; there is no partial corpus. Also refuses an id/filename mismatch — a file
    /// whose inner id differs from its stem would either mask or duplicate another
    /// ticket in the map, and both are corruption, not data.
    ///
    /// T-917.2: the load ALSO resolves scope legality against
    /// `.ai/tickets/scope-vocab.toml` (spec §Scope v2 — "legality is resolved at
    /// `Corpus::load` and in `check`"): a missing vocabulary refuses the load naming
    /// the path, and a work ticket whose domain/layer/component/surface is not in the
    /// tree refuses naming ticket + offending pair. `parse_ticket_toml` alone stays
    /// shape-strict (documented weakening on that fn).
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut corpus = Corpus::new(root);
        let vocab = crate::vocab::ScopeVocab::load(&corpus.root)?;
        let dir = corpus.tickets_dir();
        let rd = fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
        let mut paths: Vec<PathBuf> = Vec::new();
        for ent in rd {
            let ent = ent.map_err(|e| format!("read {}: {e}", dir.display()))?;
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("T-") && name.ends_with(".toml") {
                paths.push(ent.path());
            }
        }
        paths.sort();
        for path in &paths {
            let text =
                fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let ticket =
                parse_ticket_toml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if ticket.id() != stem {
                return Err(format!(
                    "{}: file stem {stem} does not match ticket id {} — refusing the load",
                    path.display(),
                    ticket.id()
                ));
            }
            if let Ticket::Work(w) = &ticket {
                vocab.check_scope(&w.id, &w.scope)?;
            }
            corpus.tickets.insert(stem, ticket);
        }
        Ok(corpus)
    }

    /// Read one ticket by id.
    pub fn get(&self, id: &str) -> Option<&Ticket> {
        self.tickets.get(id)
    }

    /// Next parent id numeral: max parent numeric + 1. Children NEVER affect it — the
    /// exact `tickets_store::derive_next_id` semantics (`T-090.6` does not make the next
    /// id 91), preserved so `ops::add` mints the same id the legacy `ticket add` would.
    pub fn derive_next_parent_id(&self) -> u64 {
        self.tickets
            .keys()
            .filter_map(|id| parent_numeric_id(id))
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Next free dotted child id under `parent_id`: max existing DIRECT numeric
    /// extension + 1, else `.1`. "Existing" is conservative — the scan covers both
    /// corpus keys and the parent's `children[]` entries, so a listed-but-fileless child
    /// or a stray file both block their numeral. Only single-segment all-digit suffixes
    /// count (`T-916.1` extends `T-916`; `T-916.1.1` does not), mirroring how
    /// `derive_next_parent_id` is max+1 over its own tier: freed numerals in the middle
    /// are never re-minted.
    pub fn next_child_id(&self, parent_id: &str) -> String {
        let prefix = format!("{parent_id}.");
        let direct_numeral = |id: &str| -> Option<u64> {
            let suffix = id.strip_prefix(&prefix)?;
            if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            suffix.parse().ok()
        };
        let mut max: Option<u64> = None;
        for id in self.tickets.keys() {
            if let Some(n) = direct_numeral(id) {
                max = Some(max.map_or(n, |m| m.max(n)));
            }
        }
        if let Some(Ticket::Program(p)) = self.tickets.get(parent_id) {
            for c in &p.children {
                if let Some(n) = direct_numeral(c) {
                    max = Some(max.map_or(n, |m| m.max(n)));
                }
            }
        }
        format!("{parent_id}.{}", max.map_or(1, |m| m + 1))
    }

    /// Surgical per-file write for exactly the given ids (an op's `changed` set). Two
    /// phases, check-before-write: EVERY file is rendered and re-parse-verified (must
    /// succeed AND equal the in-memory ticket — the `migrate_live_tree` re-parse gate,
    /// tightened to full round-trip equality) before a single byte lands; only then
    /// does each file get written as `.{id}.toml.tmp` in the same directory and
    /// renamed over `{id}.toml`. Same-directory rename is what makes each swap atomic
    /// on one filesystem; a watcher never observes a half-written ticket. Ids are
    /// deduped; an id with no corpus entry refuses the whole batch.
    pub fn write_back(&self, ids: &[String]) -> Result<(), String> {
        let mut unique: Vec<&String> = ids.iter().collect();
        unique.sort_unstable();
        unique.dedup();
        // Phase 1 — validate the whole batch: no partial batch on a validation error.
        let mut staged: Vec<(&String, String)> = Vec::with_capacity(unique.len());
        for id in unique {
            let ticket = self.tickets.get(id.as_str()).ok_or_else(|| {
                format!("write_back {id}: no such ticket in the corpus — refusing the batch")
            })?;
            let text = render_ticket_toml(ticket)?;
            let back = parse_ticket_toml(&text)
                .map_err(|e| format!("{id}: rendered TOML does not re-parse: {e}"))?;
            if back != *ticket {
                return Err(format!(
                    "{id}: render → re-parse does not round-trip to the same ticket — refusing to write"
                ));
            }
            staged.push((id, text));
        }
        // Phase 2 — land the bytes, temp+rename per file.
        let dir = self.tickets_dir();
        for (id, text) in staged {
            let target = dir.join(format!("{id}.toml"));
            let tmp = dir.join(format!(".{id}.toml.tmp"));
            fs::write(&tmp, &text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
            fs::rename(&tmp, &target)
                .map_err(|e| format!("rename {} -> {}: {e}", tmp.display(), target.display()))?;
        }
        Ok(())
    }

    /// Delete the backing files for ids an op removed (its `deleted` set). Refuses ids
    /// still present in the corpus (deleting a live ticket's file is incoherent — the
    /// op forgot to remove the entry) and refuses a missing file (the caller's model of
    /// the tree is stale). Explicit-ids-only is the whole point: this is the anti-
    /// `save_tree`, so no code path can cascade a delete it was not handed.
    pub fn delete_files(&self, ids: &[String]) -> Result<(), String> {
        let mut unique: Vec<&String> = ids.iter().collect();
        unique.sort_unstable();
        unique.dedup();
        let dir = self.tickets_dir();
        for id in &unique {
            if self.tickets.contains_key(id.as_str()) {
                return Err(format!(
                    "delete_files {id}: ticket is still in the corpus — refusing the batch"
                ));
            }
        }
        for id in unique {
            let path = dir.join(format!("{id}.toml"));
            fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Domain, ScopeV2, Status, WorkTicket};

    /// Real repo root, the xtask-tests precedent: `CARGO_MANIFEST_DIR/../..`.
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/")
            .parent()
            .expect("repo root")
            .to_path_buf()
    }

    /// Minimal vocabulary every scratch TREE carries (T-917.2: `Corpus::load` is
    /// fail-closed on the vocab file). Memory-only corpora (`Corpus::new`) never read it.
    const MINI_VOCAB: &str = "[repo.docs]\n";

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tbd-tickets-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch tickets dir");
        fs::write(dir.join(crate::vocab::VOCAB_REL), MINI_VOCAB).expect("write scratch vocab");
        dir
    }

    fn work(id: &str, status: Status) -> Ticket {
        Ticket::Work(WorkTicket {
            id: id.into(),
            title: format!("{id} title"),
            summary: format!("{id} summary"),
            class: Some("chore".into()),
            status,
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
            main_goal: None,
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

    /// The only live-tree files whose bytes deviate from the canonical
    /// `render_ticket_toml` form — both hand-edited outside any writer, both
    /// VALUE-equal after re-parse (measured 2026-08-14 over 1182 files):
    ///
    /// EMPTY since the T-916.1 land commit canonicalized the last two hand-edit
    /// deviations (`T-911.1` shipped_at slot, `T-916.2` inline layers array) as
    /// operator bookkeeping riding the same commit. The pin stays SELF-TIGHTENING
    /// both ways: a deviation outside the list fails the test, and a listed file
    /// that has become canonical ALSO fails until the entry is removed in the same
    /// commit — the `frozen_unmappable_is_49` exact-accounting pattern. The list
    /// may only ever shrink.
    const HAND_EDITED_NOT_CANONICAL: &[&str] = &[];

    /// T-916.1 acceptance 1 — the whole live tree round-trips byte-identically:
    /// load every on-disk `T-*.toml` (parents AND children), render each typed ticket,
    /// and the bytes must equal the file exactly — modulo the two pinned hand-edit
    /// exceptions above, which must still be value-equal. N is measured on disk at run
    /// time, never hardcoded (the corpus grows weekly).
    #[test]
    fn corpus_roundtrip_real_tree_byte_identical() {
        let root = repo_root();
        let dir = root.join(".ai/tickets");
        assert!(dir.is_dir(), "live tree missing at {}", dir.display());
        let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("read tickets dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name().is_some_and(|n| {
                    let n = n.to_string_lossy();
                    n.starts_with("T-") && n.ends_with(".toml")
                })
            })
            .collect();
        files.sort();
        let n = files.len();
        assert!(
            n > 800,
            "corpus scan must see the live tree (saw {n} files)"
        );
        assert_eq!(
            corpus.tickets.len(),
            n,
            "typed corpus must hold every on-disk file"
        );
        let mut identical = 0usize;
        let mut pinned_seen: Vec<String> = Vec::new();
        for path in &files {
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let disk = fs::read_to_string(path).expect("read ticket file");
            let ticket = corpus.get(&stem).expect("loaded ticket");
            let rendered = render_ticket_toml(ticket).expect("render");
            let pinned = HAND_EDITED_NOT_CANONICAL.contains(&stem.as_str());
            if rendered == disk {
                assert!(
                    !pinned,
                    "{stem} is byte-canonical now — shrink HAND_EDITED_NOT_CANONICAL in this commit"
                );
                identical += 1;
                continue;
            }
            if !pinned {
                let min = rendered.len().min(disk.len());
                let mut i = 0;
                while i < min && rendered.as_bytes()[i] == disk.as_bytes()[i] {
                    i += 1;
                }
                let lo = i.saturating_sub(60);
                panic!(
                    "{stem}: render differs from disk at byte {i}\nrendered: {:?}\ndisk: {:?}",
                    &rendered[lo..(i + 80).min(rendered.len())],
                    &disk[lo..(i + 80).min(disk.len())]
                );
            }
            // Pinned files must still be pure formatting deviations: the canonical
            // render must re-parse to the very same typed value.
            let back = crate::parse_ticket_toml(&rendered)
                .unwrap_or_else(|e| panic!("{stem}: canonical render does not parse: {e}"));
            assert_eq!(&back, ticket, "{stem}: pinned exception is not value-equal");
            pinned_seen.push(stem);
        }
        assert_eq!(
            identical + pinned_seen.len(),
            n,
            "every file is byte-identical or a pinned hand-edit exception"
        );
        println!("{identical}/{n} files byte-identical");
        if !pinned_seen.is_empty() {
            println!(
                "{} pinned hand-edit exception(s), value-equal, formatting only: {}",
                pinned_seen.len(),
                pinned_seen.join(", ")
            );
        }
    }

    /// T-917.3 shrink-only ratchet: the number of tickets carrying a nonempty
    /// `migration_legacy` — the walls the one-shot `cargo xtask ticket
    /// quarantine-walls` pass parked, measured on the live tree at the pass
    /// (instrument: typed corpus scan, `!migration_legacy.is_empty()`). The
    /// `HAND_EDITED_NOT_CANONICAL` self-tightening pattern, red BOTH ways:
    ///
    /// - **Growth is impossible by rule**: new tickets never quarantine — a
    ///   post-cutover mint is red in `ticket check` (`check_quarantine_mint`) and the
    ///   ops post-image gate refuses new wall summaries outright, so nothing can
    ///   legitimately add a carrier. A count above the pin means somebody hand-minted
    ///   the field.
    /// - **Every Program T drain batch SHRINKS this pin in the same commit** (spec
    ///   §Programs, Program T: decompose the wall into the typed fields, delete
    ///   `migration_legacy`, shrink the pin by exactly the batch size).
    const MIGRATION_LEGACY_PIN: usize = 569; // after T-919 batch 5

    /// T-917.3 permanent reversibility + ratchet proof over the live tree. The verb's
    /// own in-run assertion proved join("\n") == the pre-move summary bytes while it
    /// still held the original in memory; what stays provable forever is: (a) the
    /// carrier count equals the pin exactly, (b) every carrier is a WORK ticket (only
    /// the work-only quarantine pass may mint the field), (c) every parked wall is
    /// still an actual wall (>SUMMARY_WORD_CAP words — the move criterion), and (d)
    /// the wall survives a render → re-parse cycle byte-identically, so the
    /// newline-join stays stable through every future canonical rewrite.
    #[test]
    fn migration_legacy_ratchet_pin() {
        let root = repo_root();
        let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
        let mut carriers = 0usize;
        for (id, t) in &corpus.tickets {
            let legacy = match t {
                Ticket::Program(p) => {
                    assert!(
                        p.migration_legacy.is_empty(),
                        "{id}: program carries migration_legacy — only the work-only quarantine pass may mint the field"
                    );
                    continue;
                }
                Ticket::Work(w) => &w.migration_legacy,
            };
            if legacy.is_empty() {
                continue;
            }
            carriers += 1;
            let wall = legacy.join("\n");
            assert!(
                wall.split_whitespace().count() > crate::SUMMARY_WORD_CAP,
                "{id}: parked migration_legacy joins to {} words — not a wall; the quarantine only moves >{} word summaries",
                wall.split_whitespace().count(),
                crate::SUMMARY_WORD_CAP
            );
            let rendered = render_ticket_toml(t).expect("render carrier");
            let back = parse_ticket_toml(&rendered)
                .unwrap_or_else(|e| panic!("{id}: carrier render does not re-parse: {e}"));
            let back_legacy = match &back {
                Ticket::Work(w) => w.migration_legacy.clone(),
                Ticket::Program(_) => panic!("{id}: carrier re-parsed as program"),
            };
            assert_eq!(
                back_legacy.join("\n"),
                wall,
                "{id}: migration_legacy newline-join is not render-stable"
            );
        }
        assert_eq!(
            carriers, MIGRATION_LEGACY_PIN,
            "migration_legacy carrier count drifted from the pin — a drain commit must \
             SHRINK the pin in the same commit; growth means an illegal hand-mint"
        );
    }

    /// T-920.1 third shrink-only ratchet (the [`migration_legacy_ratchet_pin`]
    /// pattern, red BOTH ways): work+program tickets whose title is debt by THE
    /// instrument ([`crate::title_is_debt`] — `title == id` OR TOML-parsed title
    /// `split_whitespace().count() > 10`) must equal [`crate::TITLE_DEBT_PIN`]
    /// exactly.
    ///
    /// - **Growth is impossible by rule**: the ops post-image gate refuses writing a
    ///   debt title on any CHANGED ticket, so a count above the pin means a hand-edit
    ///   minted one — fix the title, never the pin.
    /// - **Every T-919/T-921 batch that repairs titles SHRINKS the pin in the same
    ///   commit** by the measured batch amount (t920 spec §T-921).
    #[test]
    fn title_debt_ratchet_pin() {
        let root = repo_root();
        let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
        let mut debt = 0usize;
        for (id, t) in &corpus.tickets {
            let title = match t {
                Ticket::Program(p) => &p.title,
                Ticket::Work(w) => &w.title,
            };
            if crate::title_is_debt(id, title) {
                debt += 1;
            }
        }
        assert_eq!(
            debt,
            crate::TITLE_DEBT_PIN,
            "title-debt count drifted from TITLE_DEBT_PIN — a repair commit must SHRINK \
             the pin in the same commit; growth means a gate bypass (instrument: \
             title == id or TOML-parsed title split_whitespace().count() > 10)"
        );
    }

    /// T-920.1 fourth shrink-only ratchet, same pattern: queued/ready/running/review
    /// WORK tickets with empty `main_goal` ([`crate::main_goal_is_debt`]) must equal
    /// [`crate::MAIN_GOAL_DEBT_PIN`] exactly. Quarantined carriers COUNT (the wall
    /// holds the content unprocessed; the T-919 drain fills main_goal and shrinks
    /// this pin in the same commit); new offenders are impossible — the ops
    /// post-image gate refuses a changed non-quarantined live work ticket without
    /// main_goal, and a quarantine mint past the cutover is red in check.
    #[test]
    fn main_goal_debt_ratchet_pin() {
        let root = repo_root();
        let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
        let debt = corpus
            .tickets
            .values()
            .filter(|t| match t {
                Ticket::Program(_) => false,
                Ticket::Work(w) => crate::main_goal_is_debt(w),
            })
            .count();
        assert_eq!(
            debt,
            crate::MAIN_GOAL_DEBT_PIN,
            "main_goal-debt count drifted from MAIN_GOAL_DEBT_PIN — a fill commit must \
             SHRINK the pin in the same commit; growth means a gate bypass (instrument: \
             queued/ready/running/review work tickets with empty main_goal)"
        );
    }

    /// `derive_next_parent_id` mirrors `tickets_store::derive_next_id`: max PARENT
    /// numeric + 1; children never affect it.
    #[test]
    fn derive_next_parent_id_ignores_children() {
        let mut c = Corpus::new("/nonexistent");
        c.tickets
            .insert("T-001".into(), work("T-001", Status::Idea));
        c.tickets
            .insert("T-910".into(), work("T-910", Status::Idea));
        c.tickets
            .insert("T-090.6".into(), work("T-090.6", Status::Idea));
        // A dotted id with a huge numeral must not leak into the parent tier.
        c.tickets
            .insert("T-910.999".into(), work("T-910.999", Status::Idea));
        assert_eq!(c.derive_next_parent_id(), 911);
        let mut planted = Corpus::new("/nonexistent");
        planted
            .tickets
            .insert("T-950".into(), work("T-950", Status::Idea));
        assert_eq!(planted.derive_next_parent_id(), 951);
        assert_eq!(Corpus::new("/nonexistent").derive_next_parent_id(), 1);
    }

    /// Next child id is max direct numeric extension + 1 over BOTH corpus keys and the
    /// parent's `children[]`; grandchildren never count; default is `.1`.
    #[test]
    fn next_child_id_direct_extensions_only() {
        let mut c = Corpus::new("/nonexistent");
        c.tickets
            .insert("T-916.1".into(), work("T-916.1", Status::Idea));
        c.tickets
            .insert("T-916.2".into(), work("T-916.2", Status::Idea));
        // Grandchild — a deeper extension must not bump T-916's own tier.
        c.tickets
            .insert("T-916.2.9".into(), work("T-916.2.9", Status::Idea));
        assert_eq!(c.next_child_id("T-916"), "T-916.3");
        assert_eq!(c.next_child_id("T-916.2"), "T-916.2.10");
        assert_eq!(c.next_child_id("T-999"), "T-999.1");
    }

    /// Fail-closed load: one broken file refuses the whole corpus, naming the file.
    #[test]
    fn load_refuses_naming_broken_file() {
        let root = scratch_dir("broken-load");
        let good = render_ticket_toml(&work("T-001", Status::Idea)).unwrap();
        fs::write(root.join(".ai/tickets/T-001.toml"), good).unwrap();
        fs::write(
            root.join(".ai/tickets/T-002.toml"),
            "id = \"T-002\"\nkind = \"nope\"\n",
        )
        .unwrap();
        let err = Corpus::load(&root).expect_err("broken file must refuse the load");
        assert!(err.contains("T-002.toml"), "must name the file: {err}");
    }

    /// Fail-closed load: id/filename mismatch is corruption, not data.
    #[test]
    fn load_refuses_id_filename_mismatch() {
        let root = scratch_dir("stem-mismatch");
        let text = render_ticket_toml(&work("T-001", Status::Idea)).unwrap();
        fs::write(root.join(".ai/tickets/T-777.toml"), text).unwrap();
        let err = Corpus::load(&root).expect_err("stem mismatch must refuse the load");
        assert!(err.contains("T-777") && err.contains("T-001"), "{err}");
    }

    /// T-917.2 — legality resolves at load: a work ticket whose scope pair is not in
    /// the vocabulary refuses naming ticket + pair; a MISSING vocabulary refuses
    /// naming the path (fail-closed — never a silent legality skip).
    #[test]
    fn load_refuses_vocab_illegal_scope_and_missing_vocab() {
        let root = scratch_dir("vocab-legality");
        let mut w = match work("T-001", Status::Idea) {
            Ticket::Work(w) => w,
            Ticket::Program(_) => unreachable!(),
        };
        w.scope.layer = "ghost_layer".into();
        fs::write(
            root.join(".ai/tickets/T-001.toml"),
            render_ticket_toml(&Ticket::Work(w)).unwrap(),
        )
        .unwrap();
        let err = Corpus::load(&root).expect_err("illegal pair must refuse");
        assert!(
            err.contains("T-001") && err.contains("repo.ghost_layer"),
            "must name ticket + offending pair: {err}"
        );

        fs::write(
            root.join(".ai/tickets/T-001.toml"),
            render_ticket_toml(&work("T-001", Status::Idea)).unwrap(),
        )
        .unwrap();
        Corpus::load(&root).expect("legal scope loads");

        fs::remove_file(root.join(crate::vocab::VOCAB_REL)).unwrap();
        let err = Corpus::load(&root).expect_err("missing vocab must refuse");
        assert!(err.contains("scope-vocab.toml"), "{err}");
    }

    /// Surgical write: temp+rename lands the rendered bytes and leaves no temp file;
    /// an unknown id refuses before any byte lands.
    #[test]
    fn write_back_is_surgical_and_clean() {
        let root = scratch_dir("write-back");
        let mut c = Corpus::new(&root);
        c.tickets
            .insert("T-001".into(), work("T-001", Status::Queued { order: 10 }));
        c.write_back(&["T-001".into()]).expect("write");
        let on_disk = fs::read_to_string(root.join(".ai/tickets/T-001.toml")).unwrap();
        assert_eq!(
            on_disk,
            render_ticket_toml(c.get("T-001").unwrap()).unwrap()
        );
        let leftovers: Vec<_> = fs::read_dir(root.join(".ai/tickets"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {leftovers:?}"
        );
        let err = c
            .write_back(&["T-404".into()])
            .expect_err("unknown id refuses");
        assert!(err.contains("T-404"), "{err}");
    }

    /// `delete_files` refuses ids still present in the corpus.
    #[test]
    fn delete_files_refuses_live_ids() {
        let root = scratch_dir("delete-live");
        let mut c = Corpus::new(&root);
        c.tickets
            .insert("T-001".into(), work("T-001", Status::Idea));
        let err = c
            .delete_files(&["T-001".into()])
            .expect_err("live id must refuse");
        assert!(err.contains("still in the corpus"), "{err}");
    }
}
