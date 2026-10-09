//! The typed corpus store over every ticket file, parents and children.
//!
//! **Role:** [`Corpus`], every `.ai/tickets/T-*.toml` in one typed map, with the fail-closed
//! load, the id minting, and the surgical writes and deletes; [`ticket_id_order_key`], the one
//! ordering of ticket ids.
//! **Position:** over the encoding and the vocabulary; `ticket_registry`'s operations mutate a
//! [`Corpus`] and write it back, and `ticket_wave_lock`, `ticket_metrics` and the checks read it.
//! **Signals & state:** a [`Corpus`] owns its map in memory; the functions read and write files
//! under its root and keep no other state.
//! **Invariants:**
//!
//! - **Fail-closed load.** A file that does not parse, whose id differs from its stem, or whose
//!   scope the vocabulary does not know refuses the whole load, naming the file; no partial
//!   corpus exists, so no reader treats a missing ticket as absent.
//! - **Surgical writes.** [`Corpus::write_back`] touches exactly the ids it is given: render,
//!   re-parse (which must succeed and equal the ticket), write a dot-prefixed temporary file in
//!   the same directory, rename it over the target. The rename keeps a directory watcher from
//!   reading a half-written file, and the dot prefix keeps the temporary file out of every
//!   `T-*.toml` glob, this module's loader included. A file is deleted only when its id is
//!   handed to [`Corpus::delete_files`], so no write cascades into a mass delete.
//! - **Children are first-class.** The map holds the dotted child files too, so an operation on
//!   a child id resolves.
//!
//! The ticketboard design specification (`documentation/tickets/specs/t915_ticketboard_design.md`,
//! its write-path section) records the reasons.

use crate::{Ticket, TicketId, parse_ticket_toml, render_ticket_toml};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    let numeral = crate::ticket_id::parent_numeral_text(parent).and_then(|text| text.parse().ok());
    (numeral.unwrap_or(u64::MAX), id)
}

/// The whole on-disk registry, typed. `tickets` is public because the store is the one read
/// path for every walk over the tickets (the wave lock's views and collisions, the `owns`
/// checks); changes go through `ticket_registry`'s operations, whose post-image validation keeps
/// any operation from writing a corpus its own preflight would refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corpus {
    root: PathBuf,
    /// Every ticket on disk, keyed by id. Load enforces key == file stem, so
    /// `{id}.toml` is always the backing file.
    pub tickets: BTreeMap<TicketId, Ticket>,
}

impl Corpus {
    /// An empty corpus rooted at `root`, the checkout root that holds `.ai/tickets/`. For
    /// scratch corpora and callers that assemble tickets in memory; it reads nothing, not even
    /// the vocabulary. The live tree goes through [`Corpus::load`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Corpus {
            root: root.into(),
            tickets: BTreeMap::new(),
        }
    }

    /// The checkout root this corpus belongs to; the checks resolve `spec` and `plan` paths
    /// against it.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The ticket directory under the root — the one directory every ticket file lives in.
    pub fn tickets_dir(&self) -> PathBuf {
        self.root.join(repository_layout::TICKETS_DIR)
    }

    /// Loads every `.ai/tickets/T-*.toml` under `root`, parents and children alike, in file
    /// name order. Fail-closed: the first file that does not parse refuses the whole load with
    /// an error naming the file, and so does a file whose id differs from its stem, which would
    /// mask or duplicate another ticket in the map.
    ///
    /// The load also resolves scope legality against `.ai/tickets/scope-vocab.toml`: a missing
    /// vocabulary refuses the load naming the path, and a work ticket whose domain, layer,
    /// component or surface the vocabulary lacks refuses naming the ticket and the word.
    /// [`crate::parse_ticket_toml`] alone checks shape only.
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
            if ticket.id() != stem.as_str() {
                return Err(format!(
                    "{}: file stem {stem} does not match ticket id {} — refusing the load",
                    path.display(),
                    ticket.id()
                ));
            }
            if let Ticket::Work(w) = &ticket {
                vocab.check_scope(&w.id, &w.scope)?;
            }
            corpus.tickets.insert(TicketId::new(stem), ticket);
        }
        Ok(corpus)
    }

    /// Read one ticket by id.
    pub fn get(&self, id: &TicketId) -> Option<&Ticket> {
        self.tickets.get(id)
    }

    /// The next parent id's numeral: the highest parent numeral plus one, or 1 for an empty
    /// corpus. Child ids never affect it, so `ticket add` mints from the parents alone.
    pub fn derive_next_parent_id(&self) -> u64 {
        self.tickets
            .keys()
            .filter_map(TicketId::parent_number)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The next free dotted child id under `parent_id`: the highest direct numeral plus one, or
    /// `.1`. The scan covers both the corpus keys and the parent's `children` entries, so a
    /// listed child without a file and a file the parent does not list both hold their
    /// numeral. Only a single all-digit segment counts (one dot extends its parent; two do
    /// not). A freed numeral below the highest is never minted again.
    pub fn next_child_id(&self, parent_id: &TicketId) -> TicketId {
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
            if let Some(n) = direct_numeral(id.as_str()) {
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
        TicketId::new(format!("{parent_id}.{}", max.map_or(1, |m| m + 1)))
    }

    /// Writes exactly the files of `ids` (an operation's changed set), in two phases. First
    /// every ticket is rendered and re-parsed, and the re-parse must succeed and equal the
    /// ticket in memory, before a single byte lands; then each file is written as
    /// `.<id>.toml.tmp` in the same directory and renamed over `<id>.toml`, which swaps it
    /// atomically on one filesystem, so a watcher never sees a half-written ticket. Duplicate
    /// ids are written once; an id with no corpus entry refuses the whole batch before any
    /// write.
    pub fn write_back(&self, ids: &[TicketId]) -> Result<(), String> {
        let mut unique: Vec<&TicketId> = ids.iter().collect();
        unique.sort_unstable();
        unique.dedup();
        // Phase 1 — validate the whole batch: no partial batch on a validation error.
        let mut staged: Vec<(&TicketId, String)> = Vec::with_capacity(unique.len());
        for id in unique {
            let ticket = self.tickets.get(id).ok_or_else(|| {
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

    /// Deletes the files of `ids`, the tickets an operation removed. Refuses the whole batch
    /// when an id is still in the corpus (the operation forgot to remove the entry), and stops
    /// at a file that is already gone (the caller's view of the tree is stale). Only the ids
    /// handed in are ever deleted.
    pub fn delete_files(&self, ids: &[TicketId]) -> Result<(), String> {
        let mut unique: Vec<&TicketId> = ids.iter().collect();
        unique.sort_unstable();
        unique.dedup();
        let dir = self.tickets_dir();
        for id in &unique {
            if self.tickets.contains_key(*id) {
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
