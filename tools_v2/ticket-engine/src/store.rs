//! Typed corpus store: every `.ai/tickets/T-*.toml`, parents AND children,
//! in one typed map, plus surgical per-file writes.
//!
//! Design authority: `documentation_v2/tickets/specs/t915_ticketboard_design.md` §Write path.
//! This module replaces the xtask Value round-trip (`load_phase2_tree` → Value mutators →
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
//!   view answered "Unknown ticket" for every dotted child id.

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

/// Numeric part of a parent id (`<prefix>-916` → `916`); `None` for children and non-ids.
pub fn parent_numeric_id(id: &str) -> Option<u64> {
    if !is_parent_id(id) {
        return None;
    }
    id.strip_prefix("T-")?.parse().ok()
}

/// The one ordering key for ticket ids: the numeral of the id's top-level parent, then the
/// whole id string.
///
/// The numeral comes first, so parent 999 sorts before parent 1000 and every dotted child sorts
/// inside its own parent's run, ahead of the next parent. Within one parent the whole-string order
/// decides, so children keep plain string order (suffix `.10.5` before suffix `.2`). An id without
/// a parent numeral sorts after every id that has one. When every parent numeral has the same
/// digit count, this order equals plain string order.
///
/// Pass `&str` to compare borrowed ids, or an owned `String` where the key must outlive the
/// element it came from (a `sort_by_key` closure).
pub fn ticket_id_order_key<S: AsRef<str>>(id: S) -> (u64, S) {
    let parent = id.as_ref().split('.').next().unwrap_or_default();
    (parent_numeric_id(parent).unwrap_or(u64::MAX), id)
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
    /// tickets programmatically; the live tree goes through [`crate::Corpus::load`].
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

    /// The ticket directory under the root — the one directory every ticket file lives in.
    pub fn tickets_dir(&self) -> PathBuf {
        self.root.join(crate::repository::TICKETS_DIR)
    }

    /// Load EVERY `.ai/tickets/T-*.toml` under `root` — parents and children alike
    /// (deliberately NOT `load_phase2_tree`, which is parents-only). Fail-closed: the
    /// first file that fails to parse refuses the whole load with an error naming that
    /// file; there is no partial corpus. Also refuses an id/filename mismatch — a file
    /// whose inner id differs from its stem would either mask or duplicate another
    /// ticket in the map, and both are corruption, not data.
    ///
    /// The load ALSO resolves scope legality against
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
    /// exact `derive_next_id` semantics (a dotted child id does not make the next
    /// id 91), so `ops::add` mints from the parents alone.
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
    /// count (one dot extends its parent; two do not), mirroring how
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
#[path = "tests/store/mod.rs"]
mod tests;
