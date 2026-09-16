//! Role: side cache.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Arc;
use super::AtomicU64;
use super::Doc;
use super::HashMap;
use super::Ordering;
use super::RefCell;
use super::RefMut;
use super::Subscription;

/// Domain representation of side key memo.
pub(super) struct SideKeyMemo {
    /// Version.
    pub(super) version: Arc<AtomicU64>,

    /// Entries.
    pub(super) entries: RefCell<(u64, HashMap<String, String>)>,

    /// sub.
    pub(super) _sub: Subscription,
}

impl SideKeyMemo {
    /// Install using the supplied domain data.
    pub(super) fn install(doc: &Doc) -> Option<Self> {
        let version = Arc::new(AtomicU64::new(1));
        let bump = Arc::clone(&version);
        let sub = doc
            .observe_after_transaction(move |_txn| {
                bump.fetch_add(1, Ordering::Relaxed);
            })
            .ok()?;
        Some(Self {
            version,
            entries: RefCell::new((0, HashMap::new())),
            _sub: sub,
        })
    }
}

impl SideKeyMemo {
    /// Entries using the supplied domain data.
    pub(super) fn entries(&self) -> RefMut<'_, HashMap<String, String>> {
        let live = self.version.load(Ordering::Relaxed);
        let mut e = self.entries.borrow_mut();
        if e.0 != live {
            e.1.clear();
            e.0 = live;
        }
        RefMut::map(e, |e| &mut e.1)
    }
}
