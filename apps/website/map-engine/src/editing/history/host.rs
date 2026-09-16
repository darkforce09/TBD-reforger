//! Role: the host services the undo drive cannot supply itself.
//! Position: `editing/history` in the map engine.
//! Signals & state: one installed service table per thread.
//! Invariants: every service has a defined default, so an uninstalled drive still undoes and
//! redoes the document correctly — it simply tells nobody. That is the honest degradation: the
//! document is the truth, and a host that has not arrived yet has nothing to refresh.

use std::cell::Cell;

/// What the undo drive asks of whoever is hosting the document.
///
/// Plain function pointers, not closures: the table is `Copy`, holds no captured state, and is
/// installed once at mount.
#[derive(Clone, Copy)]
pub struct HistoryHost {
    /// The document changed under the host. Everything that follows a committed edit is the
    /// host's: prune its selection against the settled document, rebind its render lanes, bump
    /// its version, mark the work unsaved, schedule a persist, and refresh its readouts.
    ///
    /// It is ONE hook rather than six because the order between those steps is load-bearing and
    /// belongs to whoever owns them — a drive that called them separately would be deciding that
    /// order from the wrong side of the wall.
    pub after_document_change: fn(),
}

fn no_change() {}

impl Default for HistoryHost {
    fn default() -> Self {
        Self {
            after_document_change: no_change,
        }
    }
}

thread_local! {
    static HOST: Cell<HistoryHost> = const {
        Cell::new(HistoryHost {
            after_document_change: no_change,
        })
    };
}

/// Install the host service table. Later installs replace earlier ones wholesale.
pub fn install_host(host: HistoryHost) {
    HOST.with(|h| h.set(host));
}

/// The installed services.
#[must_use]
pub fn host() -> HistoryHost {
    HOST.with(Cell::get)
}
