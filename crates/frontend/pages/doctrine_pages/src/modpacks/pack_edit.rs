//! The editable shape of a modpack, and the conversions on either side of it.
//!
//! **Role:** the draft a modpack is edited as — the pack's own fields plus its addon rows — with
//! the reader that builds one from a fetched pack and the writer that turns one back into the
//! request body the API expects.
//! **Position:** shared by the read dossier, which builds a draft only to render it, and the
//! edit form, which seeds its signals from one and sends one back.
//! **Signals & state:** none — the draft is plain owned data.
//! **Invariants:** addon order is the draft's order: `to_put_body` stamps each row's index as its
//! sort order, so moving a row is expressed by moving it in the vector.

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ModpackDto;
#[cfg(target_arch = "wasm32")]
use serde_json::{Value, json};

/// One addon row of a modpack draft.
///
/// `name` is what the list shows, `required` marks it as a key dependency, `workshop_id` is the
/// id the game launcher resolves, `mod_guid` the addon's own identifier, and `version` the
/// pinned version. Every field but `required` may be empty.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq)]
pub(super) struct ModEdit {
    pub(super) name: String,
    pub(super) required: bool,
    pub(super) workshop_id: String,
    pub(super) mod_guid: String,
    pub(super) version: String,
}

/// A whole modpack, as it is being edited.
///
/// `name` and `version` label the pack, `total_size_bytes` is the download size shown to the
/// user, `workshop_url` links the collection, `is_current` marks the pack the servers run, and
/// `mods` holds the addon rows in order.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq)]
pub(super) struct PackEdit {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) total_size_bytes: i64,
    pub(super) workshop_url: String,
    pub(super) is_current: bool,
    pub(super) mods: Vec<ModEdit>,
}

#[cfg(target_arch = "wasm32")]
impl PackEdit {
    /// The draft of a fetched modpack.
    ///
    /// Addon rows keep the order the API returned them in.
    pub(super) fn from_dto(p: &ModpackDto) -> Self {
        Self {
            name: p.modpack.name.clone(),
            version: p.modpack.version.clone(),
            total_size_bytes: p.modpack.total_size_bytes,
            workshop_url: p.modpack.workshop_url.clone(),
            is_current: p.modpack.is_current,
            mods: p
                .mods
                .iter()
                .map(|m| ModEdit {
                    name: m.name.clone(),
                    required: m.is_key_dependency,
                    workshop_id: m.workshop_id.to_string(),
                    mod_guid: m.mod_guid.clone(),
                    version: m.version.clone(),
                })
                .collect(),
        }
    }

    /// The draft as the body of a modpack write request.
    ///
    /// Each addon row is stamped with its index as its sort order, so the server stores the
    /// order the draft is in.
    pub(super) fn to_put_body(&self) -> Value {
        json!({
            "name": self.name,
            "version": self.version,
            "total_size_bytes": self.total_size_bytes,
            "workshop_url": self.workshop_url,
            "is_current": self.is_current,
            "mods": self.mods.iter().enumerate().map(|(i, m)| json!({
                "name": m.name,
                "is_key_dependency": m.required,
                "sort_order": i as i64,
                "workshop_id": m.workshop_id,
                "mod_guid": m.mod_guid,
                "version": m.version,
            })).collect::<Vec<_>>(),
        })
    }
}
