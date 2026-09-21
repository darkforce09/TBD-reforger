//! Ticket views.

use super::*;

pub fn load_views(root: &Path) -> Result<Vec<TicketView>> {
    // The shared typed corpus (`crate::store::Corpus`) replaced this module's
    // own directory walk — three near-duplicate walks existed (here, `check_open_work_owns`,
    // `slice_collisions::ticket_facts`) and the store is now the one substrate. Same
    // fail-closed contract (one unparseable file refuses the load, naming it), plus the
    // stem==id guarantee the raw walk never had. BTreeMap iteration is id-sorted — the same
    // stable order the old post-walk sort produced; it is diagnostics order only and NEVER a
    // packing sort key (candidates sort by (order, id) below; glob order is forbidden by the
    // program spec).
    let corpus = crate::Corpus::load(root).map_err(|e| anyhow::anyhow!(e))?;
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

pub fn max_concurrent() -> usize {
    std::env::var("TBD_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
}

/// Two owns sets collide when any path pair is equal or one prefix-contains the other —
/// byte-for-byte the `slice-collisions` rule.
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
