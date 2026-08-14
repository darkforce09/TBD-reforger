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

use tbd_tickets::StatusName;

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

// ---- builders (one per CLI verb — arg shapes mirror xtask/src/main.rs TicketCmd) ----

/// `ticket ship <id>` — status→shipped, stamps completed_at, clears active.
pub fn ship(id: &str) -> VerbRequest {
    request(vec!["ship".into(), id.into()])
}

/// `ticket set-status <id> <status>` — the raw 8-value enum gate.
pub fn set_status(id: &str, status: StatusName) -> VerbRequest {
    request(vec!["set-status".into(), id.into(), status.as_str().into()])
}

/// `ticket mark-ready <id> [spec]` — the verb takes ONLY id + spec; user_story /
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
mod tests {
    use super::*;
    use crate::testutil::Scratch;

    fn tail_of(req: &VerbRequest) -> Vec<&str> {
        assert_eq!(
            &req.args[..5],
            &TICKET_PREFIX[..],
            "every verb argv starts with the alias expansion"
        );
        req.args[5..].iter().map(String::as_str).collect()
    }

    #[test]
    fn ship_builder_argv_and_display() {
        let req = ship("T-915.4");
        assert_eq!(tail_of(&req), vec!["ship", "T-915.4"]);
        assert_eq!(req.display, "cargo xtask ticket ship T-915.4");
        assert_eq!(req.guard, None);
    }

    #[test]
    fn set_status_builder_covers_the_whole_enum() {
        for status in board::STATUS_ORDER {
            let req = set_status("T-9", status);
            assert_eq!(tail_of(&req), vec!["set-status", "T-9", status.as_str()]);
            assert_eq!(
                req.display,
                format!("cargo xtask ticket set-status T-9 {}", status.as_str())
            );
        }
    }

    #[test]
    fn mark_ready_with_and_without_spec() {
        let bare = mark_ready("T-9", None);
        assert_eq!(tail_of(&bare), vec!["mark-ready", "T-9"]);
        let with = mark_ready("T-9", Some("docs/platform/spec.md"));
        assert_eq!(
            tail_of(&with),
            vec!["mark-ready", "T-9", "docs/platform/spec.md"]
        );
        assert_eq!(
            with.display,
            "cargo xtask ticket mark-ready T-9 docs/platform/spec.md"
        );
    }

    #[test]
    fn reorder_and_advance_slice_builders() {
        assert_eq!(
            tail_of(&reorder("T-9", "T-8")),
            vec!["reorder", "T-9", "T-8"]
        );
        assert_eq!(
            reorder("T-9", "T-8").display,
            "cargo xtask ticket reorder T-9 T-8"
        );
        assert_eq!(tail_of(&advance_slice("T-9")), vec!["advance-slice", "T-9"]);
    }

    #[test]
    fn add_builder_summary_flag_and_display_quoting() {
        let bare = add("Solo", "   ");
        assert_eq!(tail_of(&bare), vec!["add", "Solo"]);
        let full = add("Two words", "a summary");
        assert_eq!(
            tail_of(&full),
            vec!["add", "Two words", "--summary", "a summary"]
        );
        // argv carries the raw strings; only the DISPLAY quotes them.
        assert_eq!(
            full.display,
            "cargo xtask ticket add 'Two words' --summary 'a summary'"
        );
    }

    #[test]
    fn add_child_flag_combos() {
        let plain = add_child("T-9", "kid", "", false);
        assert_eq!(tail_of(&plain), vec!["add-child", "T-9", "kid"]);
        let sum = add_child("T-9", "kid", "why", false);
        assert_eq!(
            tail_of(&sum),
            vec!["add-child", "T-9", "kid", "--summary", "why"]
        );
        let promote = add_child("T-9", "kid", "", true);
        assert_eq!(
            tail_of(&promote),
            vec!["add-child", "T-9", "kid", "--promote"]
        );
        let both = add_child("T-9", "kid", "why", true);
        assert_eq!(
            tail_of(&both),
            vec!["add-child", "T-9", "kid", "--summary", "why", "--promote"]
        );
        assert_eq!(
            both.display,
            "cargo xtask ticket add-child T-9 kid --summary why --promote"
        );
    }

    #[test]
    fn remove_force_combo() {
        assert_eq!(tail_of(&remove("T-9", false)), vec!["remove", "T-9"]);
        assert_eq!(
            tail_of(&remove("T-9", true)),
            vec!["remove", "T-9", "--force"]
        );
        assert_eq!(
            remove("T-9", true).display,
            "cargo xtask ticket remove T-9 --force"
        );
    }

    #[test]
    fn display_quoting_escapes_embedded_single_quotes() {
        let req = add("it's broken", "");
        assert_eq!(tail_of(&req), vec!["add", "it's broken"]);
        assert_eq!(req.display, "cargo xtask ticket add 'it'\\''s broken'");
    }

    // ---- queue ----

    #[test]
    fn queue_is_single_flight_fifo() {
        let mut q = VerbQueue::default();
        assert!(!q.busy());
        let started = q.submit(ship("T-1"));
        assert_eq!(
            started.as_ref().map(|r| r.display.as_str()),
            Some("cargo xtask ticket ship T-1")
        );
        assert!(q.busy(), "in-flight from submit");
        assert_eq!(q.running_display(), Some("cargo xtask ticket ship T-1"));

        assert!(q.submit(ship("T-2")).is_none(), "second submit parks FIFO");
        assert!(q.submit(ship("T-3")).is_none());
        assert_eq!(q.pending_len(), 2);

        let fin = q.finish(true);
        assert_eq!(fin.dropped, 0);
        assert_eq!(
            fin.next.as_ref().map(|r| r.display.as_str()),
            Some("cargo xtask ticket ship T-2"),
            "FIFO order"
        );
        assert!(q.busy(), "in-flight flag stays up across the handoff");

        let fin = q.finish(true);
        assert_eq!(
            fin.next.as_ref().map(|r| r.display.as_str()),
            Some("cargo xtask ticket ship T-3")
        );
        let fin = q.finish(true);
        assert_eq!(
            fin,
            Finish {
                dropped: 0,
                next: None
            }
        );
        assert!(!q.busy(), "idle only after the last verb finishes");
    }

    #[test]
    fn queue_failure_drops_the_pending_tail_and_never_retries() {
        let mut q = VerbQueue::default();
        let _ = q.submit(ship("T-1"));
        assert!(q.submit(ship("T-2")).is_none());
        assert!(q.submit(ship("T-3")).is_none());
        let fin = q.finish(false);
        assert_eq!(fin.dropped, 2, "both pending requests dropped");
        assert_eq!(fin.next, None, "nothing dispatches after a failure");
        assert!(!q.busy());
        assert_eq!(q.pending_len(), 0);
        // The queue is reusable afterwards — a fresh submit starts clean.
        assert!(q.submit(ship("T-4")).is_some());
    }

    #[test]
    fn queued_requests_keep_their_captured_fingerprints() {
        let mut q = VerbQueue::default();
        let guard = CasGuard {
            path: PathBuf::from("/repo/.ai/tickets/T-2.toml"),
            pre: Some(42),
        };
        let _ = q.submit(ship("T-1"));
        assert!(q.submit(ship("T-2").with_guard(guard.clone())).is_none());
        let fin = q.finish(true);
        assert_eq!(
            fin.next.unwrap().guard,
            Some(guard),
            "the pre-dispatch fingerprint rides the queue untouched"
        );
    }

    // ---- CAS guard ----

    #[test]
    fn cas_guard_passes_unchanged_and_refuses_changed_or_deleted() {
        let s = Scratch::new("cas");
        let file = s.path().join("T-1.toml");
        fs::write(&file, b"id = \"T-1\"\n").unwrap();
        let g = guard_for(&file);
        assert!(g.pre.is_some());
        assert!(cas_ok(Some(&g)), "unchanged bytes pass");

        // Same length, different bytes — content hash catches what len+mtime miss.
        fs::write(&file, b"id = \"T-2\"\n").unwrap();
        assert!(!cas_ok(Some(&g)), "changed bytes refuse");

        let g2 = guard_for(&file);
        assert!(cas_ok(Some(&g2)));
        fs::remove_file(&file).unwrap();
        assert!(!cas_ok(Some(&g2)), "deleted file refuses");

        assert!(cas_ok(None), "`add` has no target file — no guard");
    }

    #[test]
    fn hash_is_content_sensitive() {
        assert_ne!(hash_bytes(b"aaaa"), hash_bytes(b"aaab"));
        assert_eq!(hash_bytes(b""), 0xcbf2_9ce4_8422_2325);
    }

    // ---- offered transitions ----

    /// The acceptance matrix, pinned literally per status. The exhaustive
    /// wildcard-free match in `offered_transitions` (and the STATUS_ORDER walk
    /// in `board::column_of`) makes a 9th status a compile error, not a silent
    /// fall-through.
    #[test]
    fn offered_transitions_matrix_pinned() {
        use Transition::*;
        assert_eq!(offered_transitions(StatusName::Idea), vec![QueueAfter]);
        assert_eq!(offered_transitions(StatusName::Queued), vec![MarkReady]);
        assert_eq!(
            offered_transitions(StatusName::Ready),
            vec![Ship, DemoteToQueued, Defer, CancelTicket]
        );
        assert_eq!(
            offered_transitions(StatusName::Review),
            vec![Ship, DemoteToQueued, Defer, CancelTicket],
            "review mirrors ready"
        );
        assert_eq!(
            offered_transitions(StatusName::Running),
            vec![],
            "running is the runner's claim — no manual transitions"
        );
        assert_eq!(
            offered_transitions(StatusName::Shipped),
            vec![ReopenToQueued]
        );
        assert_eq!(
            offered_transitions(StatusName::Deferred),
            vec![ReopenToQueued]
        );
        assert_eq!(
            offered_transitions(StatusName::Cancelled),
            vec![ReopenToQueued]
        );
    }

    #[test]
    fn running_is_never_a_dispatch_target_in_the_normal_ui() {
        for status in board::STATUS_ORDER {
            for t in offered_transitions(status) {
                if let Some(req) = confirm_request(t, "T-1") {
                    assert!(
                        !req.args.iter().any(|a| a == "running"),
                        "{t:?} on {status:?} must not target running: {}",
                        req.display
                    );
                }
            }
        }
        // The two form transitions carry no status literal at all.
        assert_eq!(confirm_request(Transition::QueueAfter, "T-1"), None);
        assert_eq!(confirm_request(Transition::MarkReady, "T-1"), None);
    }

    #[test]
    fn confirm_requests_map_to_exactly_one_verb_line() {
        let cases = [
            (Transition::Ship, "cargo xtask ticket ship T-9"),
            (
                Transition::DemoteToQueued,
                "cargo xtask ticket set-status T-9 queued",
            ),
            (
                Transition::ReopenToQueued,
                "cargo xtask ticket set-status T-9 queued",
            ),
            (
                Transition::Defer,
                "cargo xtask ticket set-status T-9 deferred",
            ),
            (
                Transition::CancelTicket,
                "cargo xtask ticket set-status T-9 cancelled",
            ),
        ];
        for (t, want) in cases {
            assert_eq!(confirm_request(t, "T-9").unwrap().display, want);
        }
        assert!(confirm_note(Transition::ReopenToQueued).is_some());
        assert!(confirm_note(Transition::Ship).is_none());
    }

    #[test]
    fn advanced_matrix_per_kind_and_status() {
        use AdvancedAction::*;
        // Work tickets: raw set-status + remove; no advance-slice.
        assert_eq!(
            advanced_actions(StatusName::Queued, false),
            vec![RawSetStatus, Remove]
        );
        // Programs add advance-slice.
        assert_eq!(
            advanced_actions(StatusName::Ready, true),
            vec![RawSetStatus, AdvanceSlice, Remove]
        );
        // Running adds the one manual escape hatch — Cancel — behind Advanced.
        assert_eq!(
            advanced_actions(StatusName::Running, false),
            vec![RawSetStatus, Remove, CancelRunning]
        );
        assert_eq!(
            advanced_actions(StatusName::Running, true),
            vec![RawSetStatus, AdvanceSlice, Remove, CancelRunning]
        );
        for status in board::STATUS_ORDER {
            let has_cancel = advanced_actions(status, false).contains(&CancelRunning);
            assert_eq!(
                has_cancel,
                status == StatusName::Running,
                "CancelRunning is running-only"
            );
        }
    }

    // ---- recovery hint ----

    /// The three real wave-stale shapes from xtask (`wave_lock.rs` base drift,
    /// wave-0 membership drift, and the missing-lock DidNotRun) all trigger; a
    /// generic check ERROR or rustc noise does not.
    #[test]
    fn recovery_hint_triggers_on_the_wave_stale_signature_only() {
        let stale_base = [
            "ERROR: wave.lock wave_base 131 is stale — the close-marker ledger \
             derives 132: run `cargo xtask wave repack`",
        ];
        assert!(wants_recovery_hint(stale_base));
        let stale_membership = [
            "some earlier line",
            "ERROR: wave.lock wave 0 is stale — missing [\"T-9\"], extra []: run \
             `cargo xtask wave repack`",
        ];
        assert!(wants_recovery_hint(stale_membership));
        let missing_lock = [
            "/repo/.ai/tickets/wave.lock missing — DidNotRun: run `cargo xtask wave \
             repack`. A missing lock is a refusal, never an empty plan.",
        ];
        assert!(wants_recovery_hint(missing_lock));

        let unrelated = [
            "ERROR: T-915.9 status ready without order",
            "error[E0308]: mismatched types",
            "Unknown ticket: T-999",
        ];
        assert!(!wants_recovery_hint(unrelated));
        assert!(
            RECOVERY_HINT.contains(WAVE_STALE_SIGNATURE),
            "the hint shows the same command the refusals name"
        );
    }

    /// The mid-verb SIGKILL case (acceptance 5): a signal-killed verb printed
    /// nothing about the lock, but the crash IS the wave-stale hazard — the
    /// hint shows. A clean nonzero exit still needs the signature.
    #[test]
    fn recovery_hint_applies_on_signal_kill_even_with_a_silent_log() {
        assert!(recovery_hint_applies(true, []));
        assert!(recovery_hint_applies(
            true,
            ["   Compiling xtask v0.1.0 (/repo/xtask)"]
        ));
        assert!(!recovery_hint_applies(false, ["Unknown ticket: T-999"]));
        assert!(recovery_hint_applies(
            false,
            [
                "ERROR: wave.lock wave 0 is stale — missing [\"T-9\"], extra []: run `cargo xtask wave repack`"
            ]
        ));
    }

    #[test]
    fn success_tail_skips_cargo_noise_and_blank_lines() {
        let ship_stream = [
            "    Blocking waiting for file lock on build directory",
            "   Compiling xtask v0.1.0 (/repo/xtask)",
            "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m",
            "     Running `target/debug/xtask ticket ship T-905`",
            "T-905 -> shipped",
            "",
        ];
        assert_eq!(
            success_tail(ship_stream.into_iter()),
            Some("T-905 -> shipped".to_owned())
        );
        // set-status is silent on success — only cargo noise on the stream.
        let silent = [
            "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.1s",
            "     Running `target/debug/xtask ticket set-status T-9 queued`",
        ];
        assert_eq!(success_tail(silent.into_iter()), None);
    }

    // ---- remove gates ----

    #[test]
    fn remove_type_to_confirm_gate_requires_the_exact_id() {
        assert!(remove_gate_ok("T-915.4", "T-915.4"));
        assert!(
            remove_gate_ok("  T-915.4  ", "T-915.4"),
            "whitespace trimmed"
        );
        assert!(!remove_gate_ok("t-915.4", "T-915.4"), "case-sensitive");
        assert!(!remove_gate_ok("T-915", "T-915.4"));
        assert!(!remove_gate_ok("T-915.40", "T-915.4"));
        assert!(!remove_gate_ok("", "T-915.4"));
    }

    #[test]
    fn descendants_closure_by_dotted_prefix() {
        let ids = [
            "T-9", "T-9.1", "T-9.10", "T-9.2", "T-9.2.1", "T-90", "T-90.1", "T-8",
        ];
        assert_eq!(
            descendants(ids, "T-9"),
            vec!["T-9.1", "T-9.2", "T-9.2.1", "T-9.10"],
            "numeric sort, dot-boundary (never T-90)"
        );
        assert!(descendants(ids, "T-8").is_empty());
    }
}
