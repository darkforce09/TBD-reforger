//! Right dock favourites behavior.

use super::*;

/// the one localStorage key the favourites collection persists under. Namespaced
/// `tbd-mc-editor-…` like the sibling editor-local store; see the section header.
pub(in crate::v2::apps::editor::ui::docks::dock_right) const FAVOURITES_KEY: &str =
    "tbd-mc-editor-favourites";
/// the persisted blob's schema version. Bump when a field's shape changes in a way a raw
/// serde load of an older blob cannot absorb (adding a `#[serde(default)]` field does NOT need a
/// bump); [`migrate_favourites`] then owns the upgrade.
pub(in crate::v2::apps::editor::ui::docks::dock_right) const FAVOURITES_VERSION: u32 = 1;
/// how many entries the collection keeps. A cap exists because localStorage is a shared,
/// small, synchronously-parsed budget and nothing else bounds an add loop; 250 is far past any
/// plausible working set (the live registry offers a few hundred placeable rows in total).
pub(in crate::v2::apps::editor::ui::docks::dock_right) const FAVOURITES_MAX: usize = 250;

/// one starred asset.
///
/// `asset_id` is the full Enfusion `resource_name` — the SAME string a catalogue leaf uses as its
/// `CatalogNode::id` and hands the map as `PlacePayload::asset_id`. Storing the registry row's uuid
/// instead would break the moment a modpack is re-ingested with fresh row ids.
///
/// `label` is the display name **remembered at star time**. It is not the source of truth while the
/// asset is live (the catalogue's current `display_name` wins, so a renamed prefab shows its new
/// name); it exists so a STALE entry can still name itself instead of showing a raw prefab path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavouriteAsset {
    pub asset_id: String,
    #[serde(default)]
    pub label: String,
}

/// the persisted favourites blob: a version plus the starred entries, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Favourites {
    /// Schema version of the persisted blob (see [`FAVOURITES_VERSION`]).
    #[serde(default)]
    pub version: u32,
    /// The starred entries in display order — most recently starred first.
    #[serde(default)]
    pub items: Vec<FavouriteAsset>,
}

impl Default for Favourites {
    fn default() -> Self {
        Self {
            version: FAVOURITES_VERSION,
            items: Vec::new(),
        }
    }
}

impl Favourites {
    /// Parse a persisted blob, falling back to empty on any serde failure and normalising through
    /// [`migrate_favourites`]. Pure — no localStorage — so the whole storage contract is testable
    /// on the native build.
    #[must_use]
    pub(in crate::v2::apps::editor::ui::docks::dock_right) fn from_json(raw: &str) -> Self {
        migrate_favourites(serde_json::from_str::<Self>(raw).unwrap_or_default())
    }

    /// Serialize for persistence (empty string only if serde itself fails, which the round-trip
    /// test precludes for this shape).
    #[must_use]
    pub(in crate::v2::apps::editor::ui::docks::dock_right) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Is this asset id starred?
    #[must_use]
    pub fn contains(&self, asset_id: &str) -> bool {
        self.items.iter().any(|f| f.asset_id == asset_id)
    }

    /// How many assets are starred.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing is starred, which is what the empty-state row renders from.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ADD verb. Newest first, so the row an operator just starred is the one they see. A
    /// duplicate add is a no-op (the collection is a set keyed by `asset_id`), and an empty id is
    /// refused rather than stored as an entry nothing can ever resolve.
    pub fn add(&mut self, asset_id: &str, label: &str) {
        if asset_id.is_empty() || self.contains(asset_id) {
            return;
        }
        self.items.insert(
            0,
            FavouriteAsset {
                asset_id: asset_id.to_string(),
                label: label.to_string(),
            },
        );
        self.items.truncate(FAVOURITES_MAX);
    }

    /// The REMOVE verb. Idempotent — unstarring something that is not starred is a no-op.
    pub fn remove(&mut self, asset_id: &str) {
        self.items.retain(|f| f.asset_id != asset_id);
    }

    /// The star/unstar toggle behind the leaf's context action. Returns the NEW state: `true` when
    /// the asset is now starred, `false` when it was just removed.
    pub fn toggle(&mut self, asset_id: &str, label: &str) -> bool {
        if self.contains(asset_id) {
            self.remove(asset_id);
            false
        } else {
            self.add(asset_id, label);
            self.contains(asset_id)
        }
    }
}

/// bring a freshly-loaded blob up to the current version and normalise it. Idempotent.
///
/// Beyond the version stamp this is the integrity floor for a blob any other tab (or a person with
/// devtools) may have written: entries with an empty id are dropped, duplicates collapse to their
/// first occurrence, and the list is capped. Without it a duplicated id would render two rows whose
/// unstar buttons both target the same entry.
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn migrate_favourites(
    mut fav: Favourites,
) -> Favourites {
    if fav.version < FAVOURITES_VERSION {
        fav.version = FAVOURITES_VERSION;
    }
    let mut seen = std::collections::HashSet::new();
    fav.items
        .retain(|f| !f.asset_id.is_empty() && seen.insert(f.asset_id.clone()));
    fav.items.truncate(FAVOURITES_MAX);
    fav
}

#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn favourites_storage(
) -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// load the favourites collection. Off wasm (the native test build) this is always empty,
/// exactly like `world_layer_prefs::load_store`.
#[must_use]
pub fn load_favourites() -> Favourites {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = favourites_storage() {
            if let Ok(Some(raw)) = s.get_item(FAVOURITES_KEY) {
                return Favourites::from_json(&raw);
            }
        }
    }
    Favourites::default()
}

/// persist the favourites collection (no-op off wasm). The version is stamped current on
/// write so a load never sees a stale version this build wrote itself.
pub fn save_favourites(fav: &Favourites) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = favourites_storage() {
            let mut out = fav.clone();
            out.version = FAVOURITES_VERSION;
            let _ = s.set_item(FAVOURITES_KEY, &out.to_json());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = fav;
}

/// how one favourite resolved against the live catalogue. The whole stale-degradation rule
/// is this two-variant enum: a favourite is either live (and therefore placeable, through a named
/// palette) or stale (and therefore rendered disabled, named, and removable) — there is no third
/// state in which it is quietly dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FavouriteRow {
    /// The id is in the live catalogue and still placeable. `label` is the catalogue's CURRENT
    /// display name, not the remembered one.
    Live {
        asset_id: String,
        label: String,
        palette: CatalogPalette,
    },
    /// The id is gone from the live catalogue, or the row is no longer placeable by any palette.
    /// Kept, not pruned; `label` is the name remembered at star time (or the raw id if the blob
    /// carried none), so the row is never blank.
    Stale { asset_id: String, label: String },
}

impl FavouriteRow {
    /// The asset id either variant carries — what the unstar verb targets.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        match self {
            Self::Live { asset_id, .. } | Self::Stale { asset_id, .. } => asset_id,
        }
    }

    /// The name the row renders.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Live { label, .. } | Self::Stale { label, .. } => label,
        }
    }

    /// Whether this row can arm a place.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self, Self::Live { .. })
    }
}

/// resolve the persisted collection against the live catalogue rows, preserving order and
/// **count**: every stored favourite yields exactly one row. That invariant is the "degrade
/// honestly" requirement in one sentence — a favourite the catalogue no longer offers becomes a
/// [`FavouriteRow::Stale`], never a missing row.
///
/// Pure over `(&Favourites, &[RegistryItem])`, so the rule is unit-testable without a DOM.
#[must_use]
pub fn resolve_favourites(fav: &Favourites, items: &[RegistryItem]) -> Vec<FavouriteRow> {
    fav.items
        .iter()
        .map(|f| {
            let live = crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(
                items,
                &f.asset_id,
            )
            .and_then(|it| {
                crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette(it)
                    .map(|p| (it, p))
            });
            match live {
                Some((item, palette)) => FavouriteRow::Live {
                    asset_id: f.asset_id.clone(),
                    label: item.display_name.clone(),
                    palette,
                },
                None => FavouriteRow::Stale {
                    asset_id: f.asset_id.clone(),
                    label: if f.label.trim().is_empty() {
                        f.asset_id.clone()
                    } else {
                        f.label.clone()
                    },
                },
            }
        })
        .collect()
}

/// the star/unstar context action behind a palette leaf. ONE place writes the collection:
/// flip the signal, then persist — so a starred asset is on disk before the next render, and a
/// reload cannot lose the verb the operator just used.
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn toggle_favourite(
    favourites: RwSignal<Favourites>,
    asset_id: &str,
    label: &str,
) {
    favourites.update(|f| {
        f.toggle(asset_id, label);
    });
    save_favourites(&favourites.get_untracked());
}
