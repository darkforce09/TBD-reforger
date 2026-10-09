//! The ticket facts the compiler reads, the packing width and the path collision rule.
//!
//! **Role:** reduces every ticket file to a [`TicketView`] ([`load_views`]), reads the packing
//! width from the environment ([`max_concurrent`]) and decides whether two `owns` lists overlap
//! ([`collides`]).
//! **Position:** reads the typed corpus of `ticket_model`; feeds the compiler, the check, the
//! collision report, `ticket_registry`'s shipping status and the platform wave driver.
//! **Signals & state:** none; reads `.ai/tickets/` and the `TBD_MAX_CONCURRENT` variable.
//! **Invariants:** one unparseable ticket file refuses the whole load, naming it; the views come
//! back in id order, which is diagnostics order only and never a packing sort key.

use crate::TicketView;
use crate::error::{Error, Result};
use std::path::Path;
use ticket_model::Ticket;

/// One [`TicketView`] per ticket file under `root`, parents and children alike, in id order.
/// Fails when the corpus does not load (an unparseable file, or a file stem that differs from
/// its id).
pub fn load_views(root: &Path) -> Result<Vec<TicketView>> {
    // The typed corpus is the one reader of the ticket directory; its map iterates in id order.
    let corpus = ticket_model::Corpus::load(root).map_err(Error::msg)?;
    let views = corpus
        .tickets
        .into_values()
        .map(|t| match t {
            Ticket::Program(p) => TicketView {
                id: p.id,
                title: p.title,
                work: false,
                status: p.status.name(),
                order: p.status.order(),
                executor: p.executor,
                owns: p.owns,
                depends_on: p.depends_on,
                pack_last: p.pack_last == Some(true),
            },
            Ticket::Work(w) => TicketView {
                id: w.id,
                title: w.title,
                work: true,
                status: w.status.name(),
                order: w.status.order(),
                executor: w.executor,
                owns: w.owns,
                depends_on: w.depends_on,
                pack_last: w.pack_last == Some(true),
            },
        })
        .collect();
    Ok(views)
}

/// The packing width: `TBD_MAX_CONCURRENT` when it parses as a number, otherwise 8.
pub fn max_concurrent() -> usize {
    std::env::var("TBD_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
}

/// Whether two `owns` lists collide: some path pair is equal, or one path is a folder prefix of
/// the other (`crates/api/api_server/src/` collides with every file under it). The packer, the
/// check and `slice-collisions` all use this rule.
pub fn collides(a: &[String], b: &[String]) -> bool {
    for x in a {
        for y in b {
            if x == y
                || x.starts_with(&format!("{}/", y.trim_end_matches('/')))
                || y.starts_with(&format!("{}/", x.trim_end_matches('/')))
            {
                return true;
            }
        }
    }
    false
}
