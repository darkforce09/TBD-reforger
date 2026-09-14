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

use crate::v2::core::api::dto::ModpackDto;
use serde_json::{json, Value};

/// One addon row of a modpack draft.
///
/// `name` is what the list shows, `required` marks it as a key dependency, `workshop_id` is the
/// id the game launcher resolves, `mod_guid` the addon's own identifier, and `version` the
/// pinned version. Every field but `required` may be empty.
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
#[derive(Clone, PartialEq)]
pub(super) struct PackEdit {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) total_size_bytes: i64,
    pub(super) workshop_url: String,
    pub(super) is_current: bool,
    pub(super) mods: Vec<ModEdit>,
}

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
                    name: vstr(m, "name"),
                    required: m
                        .get("is_key_dependency")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    workshop_id: vstr(m, "workshop_id"),
                    mod_guid: vstr(m, "mod_guid"),
                    version: vstr(m, "version"),
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

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A byte count as a short human-readable size.
///
/// A gigabyte or more reads with one decimal; anything smaller is rounded to whole megabytes,
/// and a count below one byte reads as zero.
pub(super) fn format_bytes(bytes: i64) -> String {
    if bytes < 1 {
        return "0 B".into();
    }
    let gb = bytes as f64 / 1024f64.powi(3);
    if gb >= 1.0 {
        return format!("{gb:.1} GB");
    }
    format!("{:.0} MB", bytes as f64 / 1024f64.powi(2))
}
