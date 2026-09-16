//! Role: comments.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MissionDocCore;
use super::comment_row;
use super::comment_str;
use super::comment_xz;
use super::read_comment_map;
use super::remove_id_from_all_layers;
use yrs::Transact;
use yrs::types::ToJson;

impl MissionDocCore {
    /// Add comment using the supplied domain data.
    pub fn add_comment(&self, id: &str, title: &str, tooltip: &str, x: f64, z: f64) {
        let mut txn = self.begin();
        self.comments.insert(
            &mut txn,
            id,
            Any::Map(Arc::new(comment_row(id, title, tooltip, x, z))),
        );
    }
}

impl MissionDocCore {
    /// Set comment title using the supplied domain data.
    pub fn set_comment_title(&self, id: &str, title: &str) {
        self.set_comment_field(id, "title", Any::String(title.into()));
    }
}

impl MissionDocCore {
    /// Set comment tooltip using the supplied domain data.
    pub fn set_comment_tooltip(&self, id: &str, tooltip: &str) {
        self.set_comment_field(id, "tooltip", Any::String(tooltip.into()));
    }
}

impl MissionDocCore {
    /// Set comment position using the supplied domain data.
    pub fn set_comment_position(&self, id: &str, x: f64, z: f64) {
        let mut pos: HashMap<String, Any> = HashMap::new();
        pos.insert("x".to_string(), Any::Number(x));
        pos.insert("z".to_string(), Any::Number(z));
        self.set_comment_field(id, "position", Any::Map(Arc::new(pos)));
    }
}

impl MissionDocCore {
    /// Set comment field using the supplied domain data.
    pub(super) fn set_comment_field(&self, id: &str, key: &str, value: Any) {
        let mut txn = self.begin();
        let Some(mut row) = read_comment_map(&txn, &self.comments, id) else {
            return;
        };
        row.insert(key.to_string(), value);
        self.comments.insert(&mut txn, id, Any::Map(Arc::new(row)));
    }
}

impl MissionDocCore {
    /// Duplicate comment using the supplied domain data.
    pub fn duplicate_comment(&self, src_id: &str, new_id: &str, dx: f64, dz: f64) -> bool {
        let mut txn = self.begin();
        let Some(row) = read_comment_map(&txn, &self.comments, src_id) else {
            return false;
        };
        let (x, z) = comment_xz(&row);
        let title = comment_str(&row, "title");
        let tooltip = comment_str(&row, "tooltip");
        self.comments.insert(
            &mut txn,
            new_id,
            Any::Map(Arc::new(comment_row(
                new_id,
                &title,
                &tooltip,
                x + dx,
                z + dz,
            ))),
        );
        true
    }
}

impl MissionDocCore {
    /// Remove comment using the supplied domain data.
    pub fn remove_comment(&self, id: &str) {
        let mut txn = self.begin();
        self.comments.remove(&mut txn, id);
        remove_id_from_all_layers(&mut txn, &self.editor_layers, id);
    }
}

impl MissionDocCore {
    /// Move comment to layer using the supplied domain data.
    pub fn move_comment_to_layer(&self, comment_id: &str, layer_id: &str) {
        self.move_slot_to_layer(comment_id, layer_id);
    }
}

impl MissionDocCore {
    /// Comments json using the supplied domain data.
    #[must_use]
    pub fn comments_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.comments.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// Comment count using the supplied domain data.
    #[must_use]
    pub fn comment_count(&self) -> usize {
        self.comments.len(&self.doc.transact()) as usize
    }
}

impl MissionDocCore {
    /// Callers must bracket this with `set_origin_init(true)` so a template is not an undo step (the boot/seed contract of [`Self::set_origin_init`]); the editor's boot does.
    pub fn seed_template_comments(&self) -> Vec<String> {
        if self.comment_count() > 0 {
            return Vec::new();
        }
        let seeds: [(&str, &str, &str, f64, f64); 2] = [
            (
                "comment-template-1",
                "Start here",
                "Place your ORBAT first: right-click the map to add units, then drag them into \
                 folders in the Outliner. Delete this note when you no longer need it — comments \
                 are editor-only and never reach the compiled mission.",
                6_400.0,
                6_500.0,
            ),
            (
                "comment-template-2",
                "Mission notes",
                "Use comments for anything the mission file cannot carry: intent, timings, \
                 reminders for the next editor. Right-click empty ground and choose Place Comment \
                 to add another.",
                6_400.0,
                6_300.0,
            ),
        ];
        let mut ids = Vec::with_capacity(seeds.len());
        for (id, title, tooltip, x, z) in seeds {
            self.add_comment(id, title, tooltip, x, z);
            ids.push(id.to_string());
        }
        ids
    }
}
