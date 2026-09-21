//! Mutation verbs (T-915.4, design §Write path "App-side verb plumbing") — pure,
//! unit-tested, no egui and no subprocess types.
//!
//! The xtask verbs are THE write path: every mutation shells
//! `cargo xtask ticket <verb>` through the T-915.3 subproc helper (explicit
//! `current_dir` at the repo root, robustly resolved cargo). The app NEVER writes
//! ticket files itself, NEVER runs `wave repack` on its own initiative (a mid-verb
//! crash shows the recovery command as TEXT — [`RECOVERY_HINT`]), and NEVER
//! auto-retries a failed verb.
//!
//! This module owns the pure halves: typed argv builders (alias-expanded exactly
//! like `trust::CHECK_ARGS`), the single-flight FIFO [`VerbQueue`], the
//! compare-and-swap [`CasGuard`] (content hash of the ticket file's bytes), the
//! offered-transitions matrix ([`offered_transitions`] — exhaustive match, no
//! wildcard, so a 9th `StatusName` fails compile), the wave-stale recovery-hint
//! trigger, and the remove type-to-confirm gate.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use ticket_engine::StatusName;

use crate::board;

/// argv prefix spawned before the verb tail: the `.cargo/config.toml` alias
/// expansion of `cargo xtask ticket` (`xtask = "run --package xtask --"`) —
/// byte-equivalent without depending on alias resolution, mirroring
/// `trust::CHECK_ARGS`.
pub const TICKET_PREFIX: [&str; 5] = ["run", "--package", "xtask", "--", "ticket"];

/// The recovery text shown after a crashed/refused verb leaves the wave ledger
/// stale. TEXT ONLY — the app must not grow a button that runs it (that would
/// make the app a second wave writer).
pub const RECOVERY_HINT: &str = "operator: run `cargo xtask wave repack`";

/// Every wave-stale refusal in `xtask` names the repack command verbatim
/// (`wave_lock.rs`: "… is stale — …: run `cargo xtask wave repack`" and the
/// missing-lock DidNotRun text). The hint triggers on the command, not on prose.
const WAVE_STALE_SIGNATURE: &str = "cargo xtask wave repack";

// ---- requests ----

/// One verb invocation, fully built: the argv to hand to the subproc helper and
/// the literal command line the operator confirms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbRequest {
    /// argv after `cargo` — [`TICKET_PREFIX`] + verb tail.
    pub args: Vec<String>,
    /// `cargo xtask ticket ship T-915.4` — shown verbatim in every confirm
    /// dialog and in the drawer header.
    pub display: String,
    /// Compare-and-swap target; `None` only for `add` (no pre-existing file).
    pub guard: Option<CasGuard>,
}

impl VerbRequest {
    pub fn with_guard(mut self, guard: CasGuard) -> Self {
        self.guard = Some(guard);
        self
    }
}

/// Single-quote an argument for DISPLAY when it contains anything beyond the
/// plain filename alphabet. The spawned argv is the exact string — no shell ever
/// parses it; quoting exists so the confirm dialog shows a paste-able line.
fn sh_quote(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@' | '+' | '=')
        });
    if plain {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn request(tail: Vec<String>) -> VerbRequest {
    let mut args: Vec<String> = TICKET_PREFIX.iter().map(|s| (*s).to_owned()).collect();
    args.extend(tail.iter().cloned());
    let display = std::iter::once("cargo xtask ticket".to_owned())
        .chain(tail.iter().map(|a| sh_quote(a)))
        .collect::<Vec<_>>()
        .join(" ");
    VerbRequest {
        args,
        display,
        guard: None,
    }
}

// ---- builders (one per CLI verb — arg shapes mirror tools_v2/xtask/src/main.rs TicketCmd) ----

/// `ticket ship <id>` — status→shipped, stamps completed_at, clears active.
pub fn ship(id: &str) -> VerbRequest {
    request(vec!["ship".into(), id.into()])
}

/// `ticket set-status <id> <status>` — the raw 8-value enum gate.
pub fn set_status(id: &str, status: StatusName) -> VerbRequest {
    request(vec!["set-status".into(), id.into(), status.as_str().into()])
}

/// `ticket mark-ready <id> [spec]` — the verb takes ONLY id + spec; main_goal /
/// acceptance backfill is the verb's own behavior, never a UI field.
pub fn mark_ready(id: &str, spec: Option<&str>) -> VerbRequest {
    let mut tail = vec!["mark-ready".to_owned(), id.to_owned()];
    if let Some(spec) = spec {
        tail.push(spec.to_owned());
    }
    request(tail)
}

/// `ticket reorder <id> <after>` — order = anchor + 1; flips idea→queued
/// server-side.
pub fn reorder(id: &str, after: &str) -> VerbRequest {
    request(vec!["reorder".into(), id.into(), after.into()])
}

/// `ticket add <title> [--summary <s>]` — id minted server-side (max parent
/// numeric + 1), kind work, status idea.
pub fn add(title: &str, summary: &str) -> VerbRequest {
    let mut tail = vec!["add".to_owned(), title.to_owned()];
    if !summary.trim().is_empty() {
        tail.push("--summary".to_owned());
        tail.push(summary.to_owned());
    }
    request(tail)
}

/// `ticket add-child <parent> <title> [--summary <s>] [--promote]` — a work
/// parent refuses without `--promote` (the atomic work→program rewrite).
pub fn add_child(parent: &str, title: &str, summary: &str, promote: bool) -> VerbRequest {
    let mut tail = vec!["add-child".to_owned(), parent.to_owned(), title.to_owned()];
    if !summary.trim().is_empty() {
        tail.push("--summary".to_owned());
        tail.push(summary.to_owned());
    }
    if promote {
        tail.push("--promote".to_owned());
    }
    request(tail)
}

/// `ticket remove <id> [--force]` — a program refuses without `--force`
/// (cascade-deletes every descendant ticket file).
pub fn remove(id: &str, force: bool) -> VerbRequest {
    let mut tail = vec!["remove".to_owned(), id.to_owned()];
    if force {
        tail.push("--force".to_owned());
    }
    request(tail)
}

/// `ticket advance-slice <id>` — programs only; walks typed children.
pub fn advance_slice(id: &str) -> VerbRequest {
    request(vec!["advance-slice".into(), id.into()])
}

// ---- compare-and-swap guard ----

/// Fingerprint of the target ticket file, captured when the affordance was
/// rendered/clicked. At dispatch the file is re-hashed; a mismatch refuses the
/// dispatch (no subprocess) with a "file changed on disk — reloading" toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasGuard {
    pub path: PathBuf,
    /// FNV-1a over the file bytes; `None` = unreadable/absent at capture time.
    pub pre: Option<u64>,
}

/// FNV-1a 64 — cheap, dependency-free, and content-based on purpose (mtime lies
/// under editors that preserve timestamps; length misses same-length edits).
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn fingerprint_file(path: &Path) -> Option<u64> {
    fs::read(path).ok().map(|bytes| hash_bytes(&bytes))
}

/// Capture the guard for a ticket file NOW (render-of-menu / click time).
pub fn guard_for(path: &Path) -> CasGuard {
    CasGuard {
        path: path.to_path_buf(),
        pre: fingerprint_file(path),
    }
}

/// Re-hash at dispatch time. `None` (no target file — `add`) always passes; a
/// guard passes only when the bytes hash identically, including the
/// both-absent case (still-missing file: the verb's own "Unknown ticket"
/// refusal is the right surface, not a CAS refusal).
pub fn cas_ok(guard: Option<&CasGuard>) -> bool {
    match guard {
        None => true,
        Some(g) => fingerprint_file(&g.path) == g.pre,
    }
}

// ---- single-flight queue ----

/// Result of [`VerbQueue::finish`].
#[derive(Debug, PartialEq, Eq)]
pub struct Finish {
    /// Pending requests dropped because the finished verb failed — surfaced in
    /// the drawer; nothing auto-retries.
    pub dropped: usize,
    /// The next FIFO request, already marked running — the caller must spawn it
    /// or report it back through another `finish` (CAS refusal).
    pub next: Option<VerbRequest>,
}

/// One verb subprocess at a time; extra requests wait FIFO, each keeping the
/// fingerprint captured when its affordance was used. A failure drops the whole
/// pending tail — cascading writes onto a refusal helps nobody, and the app
/// never auto-retries.
#[derive(Debug, Default)]
pub struct VerbQueue {
    running: Option<VerbRequest>,
    pending: VecDeque<VerbRequest>,
}

impl VerbQueue {
    /// Submit a request. `Some` ⇒ the queue was idle — spawn it NOW; `None` ⇒
    /// parked FIFO behind the in-flight verb.
    #[must_use]
    pub fn submit(&mut self, req: VerbRequest) -> Option<VerbRequest> {
        if self.running.is_some() {
            self.pending.push_back(req);
            None
        } else {
            self.running = Some(req.clone());
            Some(req)
        }
    }

    /// The in-flight verb finished (or a popped-but-CAS-refused request was
    /// abandoned — report that as `success = true`: the refusal is not a verb
    /// failure). Failure clears the pending tail; success pops the next request
    /// and marks it running.
    #[must_use]
    pub fn finish(&mut self, success: bool) -> Finish {
        self.running = None;
        if !success {
            let dropped = self.pending.len();
            self.pending.clear();
            return Finish {
                dropped,
                next: None,
            };
        }
        let next = self.pending.pop_front();
        if let Some(n) = &next {
            self.running = Some(n.clone());
        }
        Finish { dropped: 0, next }
    }

    /// The app's in-flight flag (drives `set_verb_in_flight` and every disabled
    /// affordance).
    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// The literal command line of the running verb (drawer header).
    pub fn running_display(&self) -> Option<&str> {
        self.running.as_ref().map(|r| r.display.as_str())
    }
}

// ---- offered transitions (the normal, non-advanced surface) ----

/// A transition affordance offered on a card / in the detail panel. Each maps to
/// exactly one xtask verb; `QueueAfter` and `MarkReady` open forms (anchor
/// picker / Ready-prose), the rest are one-click confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// idea → `ticket reorder <id> <anchor>` (flips idea→queued server-side).
    QueueAfter,
    /// queued → `ticket mark-ready <id> <spec>`.
    MarkReady,
    /// ready/review → `ticket ship <id>`.
    Ship,
    /// ready/review → `ticket set-status <id> queued`.
    DemoteToQueued,
    /// ready/review → `ticket set-status <id> deferred`.
    Defer,
    /// ready/review → `ticket set-status <id> cancelled`.
    CancelTicket,
    /// shipped/cancelled/deferred → `ticket set-status <id> queued`.
    ReopenToQueued,
}

/// The offered set per current status — the T-915.4 acceptance matrix.
/// Exhaustive match, NO wildcard: a 9th `StatusName` variant fails this compile.
/// `running` offers NOTHING here (the runner's claim); its Cancel lives behind
/// the detail panel's Advanced section — and `running` is never a TARGET
/// anywhere in the normal UI (pinned by test).
pub fn offered_transitions(status: StatusName) -> Vec<Transition> {
    match status {
        StatusName::Idea => vec![Transition::QueueAfter],
        StatusName::Queued => vec![Transition::MarkReady],
        StatusName::Ready | StatusName::Review => vec![
            Transition::Ship,
            Transition::DemoteToQueued,
            Transition::Defer,
            Transition::CancelTicket,
        ],
        StatusName::Running => vec![],
        StatusName::Shipped | StatusName::Deferred | StatusName::Cancelled => {
            vec![Transition::ReopenToQueued]
        }
    }
}

/// Menu / button label. `…` marks the ones that open a further form or confirm
/// with input; every path ends in a confirm showing the literal command.
pub fn transition_label(t: Transition) -> &'static str {
    match t {
        Transition::QueueAfter => "Queue after…",
        Transition::MarkReady => "Mark ready…",
        Transition::Ship => "Ship…",
        Transition::DemoteToQueued => "Demote to queued",
        Transition::Defer => "Defer",
        Transition::CancelTicket => "Cancel",
        Transition::ReopenToQueued => "Reopen to queued",
    }
}

/// The one-verb mapping for confirm-style transitions; the two form transitions
/// (`QueueAfter`, `MarkReady`) return `None` — their dialogs build the request
/// from operator input.
pub fn confirm_request(t: Transition, id: &str) -> Option<VerbRequest> {
    match t {
        Transition::Ship => Some(ship(id)),
        Transition::DemoteToQueued | Transition::ReopenToQueued => {
            Some(set_status(id, StatusName::Queued))
        }
        Transition::Defer => Some(set_status(id, StatusName::Deferred)),
        Transition::CancelTicket => Some(set_status(id, StatusName::Cancelled)),
        Transition::QueueAfter | Transition::MarkReady => None,
    }
}

/// Extra honesty line under the confirm's command, where a transition has a
/// known server-side refusal worth naming up front.
pub fn confirm_note(t: Transition) -> Option<&'static str> {
    match t {
        Transition::ReopenToQueued => Some(
            "set-status queued — refuses server-side if order rules break; \
             the refusal streams verbatim.",
        ),
        Transition::QueueAfter
        | Transition::MarkReady
        | Transition::Ship
        | Transition::DemoteToQueued
        | Transition::Defer
        | Transition::CancelTicket => None,
    }
}

// ---- advanced affordances (collapsed section in the detail panel) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedAction {
    /// Raw `set-status` dropdown over all 8 statuses + dispatch.
    RawSetStatus,
    /// `advance-slice` — programs only.
    AdvanceSlice,
    /// `remove [--force]` behind type-to-confirm.
    Remove,
    /// The ONLY manual affordance on a running ticket (the runner's claim).
    CancelRunning,
}

/// Advanced set per kind + status. The running gate is an exhaustive match too —
/// same 9th-status compile break as [`offered_transitions`].
pub fn advanced_actions(status: StatusName, is_program: bool) -> Vec<AdvancedAction> {
    let cancel_running = match status {
        StatusName::Running => true,
        StatusName::Idea
        | StatusName::Queued
        | StatusName::Ready
        | StatusName::Review
        | StatusName::Shipped
        | StatusName::Deferred
        | StatusName::Cancelled => false,
    };
    let mut out = vec![AdvancedAction::RawSetStatus];
    if is_program {
        out.push(AdvancedAction::AdvanceSlice);
    }
    out.push(AdvancedAction::Remove);
    if cancel_running {
        out.push(AdvancedAction::CancelRunning);
    }
    out
}

// ---- recovery hint + success tail ----

/// True when the merged verb output carries the wave-stale / check-red
/// signature — every such refusal in xtask names the repack command verbatim.
/// The app then shows [`RECOVERY_HINT`] as TEXT (no button).
pub fn wants_recovery_hint<'a>(lines: impl IntoIterator<Item = &'a str>) -> bool {
    lines.into_iter().any(|l| l.contains(WAVE_STALE_SIGNATURE))
}

/// The drawer's recovery-hint decision. `killed` = the verb died on a signal
/// (mid-verb SIGKILL between save and repack — acceptance 5): the process
/// printed nothing about the stale lock, but the crash itself is exactly the
/// state the repack recovers, so the hint shows. Otherwise the hint needs the
/// wave-stale signature in the log. A spawn failure is NOT `killed` — nothing
/// ran, nothing is stale.
pub fn recovery_hint_applies<'a>(killed: bool, lines: impl IntoIterator<Item = &'a str>) -> bool {
    killed || wants_recovery_hint(lines)
}

/// Cargo build/launch noise on the merged stream — never the verb's own output.
fn is_cargo_noise(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("Compiling ")
        || t.starts_with("Finished ")
        || t.starts_with("Running `")
        || t.starts_with("Blocking ")
        || t.starts_with("Downloading ")
        || t.starts_with("Downloaded ")
        || t.starts_with("Updating ")
        || t.starts_with("Checking ")
        || t.starts_with("Fresh ")
        || t.starts_with("Locking ")
        || t.starts_with("warning")
        || t.starts_with("note:")
}

/// The success-toast text: the LAST non-empty, non-cargo line of the merged
/// stream (e.g. `T-905 -> shipped`). `None` when the verb printed nothing
/// (set-status is silent on success) — the caller falls back to the exit line.
pub fn success_tail<'a>(lines: impl DoubleEndedIterator<Item = &'a str>) -> Option<String> {
    lines
        .rev()
        .find(|l| !l.trim().is_empty() && !is_cargo_noise(l))
        .map(|l| l.trim().to_owned())
}

// ---- remove gates ----

/// Type-to-confirm: the operator must type the exact ticket id (surrounding
/// whitespace forgiven, case NOT — ids are uppercase `T-`).
pub fn remove_gate_ok(typed: &str, id: &str) -> bool {
    typed.trim() == id
}

/// The descendant closure `remove --force` cascade-deletes: every corpus id
/// that is a dotted extension of `id`, numerically sorted. Pure set arithmetic
/// over the loaded corpus — the red warning list in the Remove dialog.
pub fn descendants<'a>(ids: impl IntoIterator<Item = &'a str>, id: &str) -> Vec<String> {
    let prefix = format!("{id}.");
    let mut out: Vec<String> = ids
        .into_iter()
        .filter(|c| c.starts_with(&prefix))
        .map(str::to_owned)
        .collect();
    out.sort_by_key(|c| board::id_sort_key(c));
    out
}

#[cfg(test)]
#[path = "tests/verbs_tests.rs"]
mod tests;
