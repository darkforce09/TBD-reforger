//! Bookmarks for the left editor dock.

use super::*;

/// Local storage key for the versioned bookmark collection.
pub(super) const BOOKMARKS_KEY: &str = "tbd-mc-editor-bookmarks";
/// serde load of an older blob cannot absorb (adding a `#[serde(default)]` field does NOT need a
/// bump); [`migrate_bookmarks`] then owns the upgrade.
pub(super) const BOOKMARKS_VERSION: u32 = 1;
/// parsed budget and nothing else bounds an add loop; 200 named views on one 12.8 km map is far past
/// any plausible working set.
pub(super) const BOOKMARKS_MAX: usize = 200;

/// deck zoom — the exact triple `RenderEngine::set_view` takes and `camera_snapshot` returns).
///
/// The NAME is the identity. There is no separate id: bookmarks are operator-authored and few, a
/// duplicate name would make the rename/remove buttons ambiguous, and a synthetic id would have to
/// be generated, persisted and kept unique for no benefit the operator can see.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Versioned bookmark collection in most-recent-first order.
pub struct Bookmarks {
    /// Schema version of the persisted blob (see [`BOOKMARKS_VERSION`]).
    #[serde(default)]
    pub version: u32,
    /// The saved views in display order — most recently added first.
    #[serde(default)]
    pub items: Vec<Bookmark>,
}

impl Default for Bookmarks {
    fn default() -> Self {
        Self {
            version: BOOKMARKS_VERSION,
            items: Vec::new(),
        }
    }
}

/// The identity key for a bookmark name: trimmed and case-folded, so "Levie" and " levie " are one
/// bookmark rather than two rows whose remove buttons look identical.
fn bookmark_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl Bookmarks {
    /// Parse a persisted blob, falling back to empty on any serde failure and normalising through
    /// [`migrate_bookmarks`]. Pure — no localStorage — so the whole storage contract is testable on
    /// the native build.
    #[must_use]
    pub(super) fn from_json(raw: &str) -> Self {
        migrate_bookmarks(serde_json::from_str::<Self>(raw).unwrap_or_default())
    }

    /// Serialize for persistence (empty string only if serde itself fails, which the round-trip test
    /// precludes for this shape).
    #[must_use]
    pub(super) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Is a bookmark already stored under this name? Names are compared through the same
    /// normalising key the store uses, so two spellings of one name are one bookmark.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        let k = bookmark_key(name);
        self.items.iter().any(|b| bookmark_key(&b.name) == k)
    }

    /// How many bookmarks are stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing is bookmarked, which is what the empty-state row renders from.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ADD verb — save the given camera position under `name`. Newest first, so the view just
    /// saved is the one the operator sees. Returns whether anything was added: an empty name and a
    /// duplicate name are both refused rather than stored (a nameless bookmark can never be found
    /// again, and a duplicate would make the row actions ambiguous).
    pub fn add(&mut self, name: &str, x: f64, y: f64, zoom: f64) -> bool {
        let name = name.trim();
        if name.is_empty() || self.contains(name) {
            return false;
        }
        self.items.insert(
            0,
            Bookmark {
                name: name.to_string(),
                x,
                y,
                zoom,
            },
        );
        self.items.truncate(BOOKMARKS_MAX);
        true
    }

    /// The RENAME verb. Refuses an empty target and a collision with a DIFFERENT bookmark; renaming
    /// a bookmark to its own name (or to a different casing of it) is allowed and just rewrites the
    /// label. Returns whether the collection changed.
    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        let to = to.trim();
        if to.is_empty() {
            return false;
        }
        let from_key = bookmark_key(from);
        let to_key = bookmark_key(to);
        if to_key != from_key && self.contains(to) {
            return false;
        }
        for b in &mut self.items {
            if bookmark_key(&b.name) == from_key {
                if b.name == to {
                    return false;
                }
                b.name = to.to_string();
                return true;
            }
        }
        false
    }

    /// The REMOVE verb. Idempotent — removing an absent bookmark is a no-op.
    pub fn remove(&mut self, name: &str) {
        let k = bookmark_key(name);
        self.items.retain(|b| bookmark_key(&b.name) != k);
    }
}

///
/// Beyond the version stamp this is the integrity floor for a blob another tab (or a person with
/// devtools) may have written: unnamed entries are dropped, duplicate names collapse to their first
/// occurrence, non-finite coordinates are dropped (a `NaN` centre would send the camera nowhere
/// recoverable), and the list is capped.
fn migrate_bookmarks(mut bm: Bookmarks) -> Bookmarks {
    if bm.version < BOOKMARKS_VERSION {
        bm.version = BOOKMARKS_VERSION;
    }
    let mut seen = std::collections::HashSet::new();
    bm.items.retain(|b| {
        !b.name.trim().is_empty()
            && b.x.is_finite()
            && b.y.is_finite()
            && b.zoom.is_finite()
            && seen.insert(bookmark_key(&b.name))
    });
    bm.items.truncate(BOOKMARKS_MAX);
    bm
}

#[must_use]
/// Return bookmarks whose names match the search query.
pub fn filter_bookmarks(bm: &Bookmarks, query: &str) -> Vec<Bookmark> {
    bm.items
        .iter()
        .filter(|b| matches_query(&b.name, query))
        .cloned()
        .collect()
}

/// rather than blank so the fastest path (click, Enter) still produces a distinct, findable name.
#[must_use]
pub fn default_bookmark_name(bm: &Bookmarks) -> String {
    (1..=BOOKMARKS_MAX + 1)
        .map(|n| format!("View {n}"))
        .find(|n| !bm.contains(n))
        .unwrap_or_else(|| String::from("View"))
}

#[cfg(target_arch = "wasm32")]
fn bookmarks_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// exactly like `world_layer_prefs::load_store` and `eden_dock_right::load_favourites`.
#[must_use]
pub fn load_bookmarks() -> Bookmarks {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = bookmarks_storage() {
            if let Ok(Some(raw)) = s.get_item(BOOKMARKS_KEY) {
                return Bookmarks::from_json(&raw);
            }
        }
    }
    Bookmarks::default()
}

/// so a load never sees a stale version this build wrote itself.
pub fn save_bookmarks(bm: &Bookmarks) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = bookmarks_storage() {
            let mut out = bm.clone();
            out.version = BOOKMARKS_VERSION;
            let _ = s.set_item(BOOKMARKS_KEY, &out.to_json());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bm;
}
