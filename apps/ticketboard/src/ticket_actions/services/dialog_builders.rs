use crate::ticket_actions::{
    models::*,
    services::commands::{self as verbs, FileChangeGuard, Transition},
};
use crate::ticket_registry::models::projection as board;
use ticket_engine::Ticket;

// ---- dialog constructors (fingerprints captured HERE) ----

/// Dialog for a normal transition. `guard` was captured at render-of-menu /
/// click time by the caller.
pub fn transition_dialog(
    b: &TicketActionContext<'_>,
    index: usize,
    t: Transition,
    guard: FileChangeGuard,
) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    let id = loaded.ticket.id().to_owned();
    match t {
        Transition::QueueAfter => Dialog::AnchorPick {
            id,
            guard,
            filter: String::new(),
            selected: None,
        },
        Transition::MarkReady => Dialog::MarkReady {
            spec: ready_spec_prefill(b, index),
            id,
            guard,
            stat: None,
        },
        Transition::Ship
        | Transition::DemoteToQueued
        | Transition::Defer
        | Transition::CancelTicket
        | Transition::ReopenToQueued => {
            let req = verbs::confirm_request(t, &id)
                .expect("non-form transitions map to one verb")
                .with_guard(guard);
            Dialog::Confirm {
                title: format!(
                    "{} — {id}",
                    verbs::transition_label(t).trim_end_matches('…')
                ),
                note: verbs::confirm_note(t).map(str::to_owned),
                req,
            }
        }
    }
}

/// Ready-form spec prefill: the ticket's own spec, else the parent program's
/// spec when it has one, else empty.
pub(crate) fn ready_spec_prefill(b: &TicketActionContext<'_>, index: usize) -> String {
    let v = board::view(&b.corpus.tickets[index].ticket);
    if let Some(own) = v.spec {
        return own.to_owned();
    }
    if let Some(parent) = v.parent
        && let Some(&pi) = b.id_to_index.get(parent)
        && let Some(spec) = board::view(&b.corpus.tickets[pi].ticket).spec
    {
        return spec.to_owned();
    }
    String::new()
}

/// Anchor picker via drag-onto-queued (fingerprint captured at drop time).
pub fn anchor_dialog(b: &TicketActionContext<'_>, index: usize) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::AnchorPick {
        id: loaded.ticket.id().to_owned(),
        guard: verbs::guard_for(&loaded.path),
        filter: String::new(),
        selected: None,
    }
}

pub fn add_dialog() -> Dialog {
    Dialog::AddTicket {
        title: String::new(),
        summary: String::new(),
    }
}

pub fn add_child_dialog(
    b: &TicketActionContext<'_>,
    index: usize,
    guard: FileChangeGuard,
) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::AddChild {
        parent: loaded.ticket.id().to_owned(),
        parent_is_work: matches!(loaded.ticket, Ticket::Work(_)),
        guard,
        title: String::new(),
        summary: String::new(),
        promote: false,
    }
}

pub fn remove_dialog(b: &TicketActionContext<'_>, index: usize, guard: FileChangeGuard) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::Remove {
        id: loaded.ticket.id().to_owned(),
        is_program: matches!(loaded.ticket, Ticket::Program(_)),
        guard,
        force: false,
        typed: String::new(),
    }
}

/// Anchor candidates: every corpus ticket carrying an order (except the ticket
/// itself), `(order, id)`-sorted, filtered on lowercase id+title.
pub(crate) fn anchor_candidates(
    b: &TicketActionContext<'_>,
    exclude: &str,
    filter: &str,
) -> Vec<(i64, String, String)> {
    let needle = filter.trim().to_lowercase();
    let mut out: Vec<(i64, String, String)> = b
        .corpus
        .tickets
        .iter()
        .filter_map(|loaded| {
            let v = board::view(&loaded.ticket);
            let order = v.status.order()?;
            if v.id == exclude {
                return None;
            }
            if !needle.is_empty()
                && !format!("{}\n{}", v.id, v.title)
                    .to_lowercase()
                    .contains(&needle)
            {
                return None;
            }
            Some((order, v.id.to_owned(), v.title.to_owned()))
        })
        .collect();
    out.sort_by_key(|a| (a.0, board::id_sort_key(&a.1)));
    out
}
