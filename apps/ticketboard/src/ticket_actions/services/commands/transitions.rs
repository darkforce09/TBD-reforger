use super::*;
/// The recovery text shown after a crashed/refused verb leaves the wave ledger
/// stale. TEXT ONLY — the app must not grow a button that runs it (that would
/// make the app a second wave writer).
pub const RECOVERY_HINT: &str = "operator: run `cargo xtask wave repack`";

/// Every wave-stale refusal in `xtask` names the repack command verbatim
/// (`wave_lock.rs`: "… is stale — …: run `cargo xtask wave repack`" and the
/// missing-lock DidNotRun text). The hint triggers on the command, not on prose.
pub(super) const WAVE_STALE_SIGNATURE: &str = "cargo xtask wave repack";

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

/// The offered set per current status — the acceptance matrix.
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
pub fn confirm_request(t: Transition, id: &str) -> Option<TicketCommand> {
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
/// (mid-verb SIGKILL between save and repack — ): the process
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
