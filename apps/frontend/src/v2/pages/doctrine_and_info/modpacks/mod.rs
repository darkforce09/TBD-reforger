//! The modpacks page: the pack list, the manifest, the edit form and the route that binds them.
//!
//! **Role:** declares the route component, the master list of packs, the read dossier, the
//! administrator's edit form, the read/edit switch they share and the draft type both write, and
//! re-exports the page for the router.
//! **Position:** the `/modpacks` route, in the doctrine hub.
//! **Signals & state:** none at this level; the page owns the fetch and every shared signal.
//! **Invariants:** both panes read the one fetched list — nothing here fetches a second time.
#![allow(dead_code)]

mod mod_table;
mod mode_toggle;
mod pack_edit;
mod pack_editor;
mod page;
mod preset_list;

pub use page::ModpacksPage;
