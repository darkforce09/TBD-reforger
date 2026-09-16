//! Role: metadata.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MissionDocCore;
use super::json_str_to_any;
use super::read_env_map;

impl MissionDocCore {
    /// Set the mission title (mirrors `ydoc.setTitle`).
    pub fn set_title(&self, title: &str) {
        let mut txn = self.begin();
        self.meta.insert(&mut txn, "title", title);
    }
}

impl MissionDocCore {
    /// Merge an environment patch (a JSON object) onto the existing `meta.environment`, mirroring `ydoc.updateEnvironment` (`{...env, ...patch}`). Absent env → the patch becomes the env.
    pub fn update_environment(&self, patch_json: &str) {
        let mut txn = self.begin();
        let mut env = read_env_map(&txn, &self.meta);
        if let Any::Map(patch) = json_str_to_any(patch_json) {
            for (k, v) in patch.iter() {
                env.insert(k.clone(), v.clone());
            }
        }
        self.meta
            .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
    }
}

impl MissionDocCore {
    /// Apply row meta using the supplied domain data.
    pub fn apply_row_meta(
        &self,
        title: &str,
        terrain: &str,
        time_of_day: Option<String>,
        weather: Option<String>,
        briefing: Option<String>,
    ) {
        let mut txn = self.begin();
        let title = title.trim();
        if !title.is_empty() {
            self.meta.insert(&mut txn, "title", title);
        }
        if matches!(terrain, "everon" | "arland" | "custom") {
            self.meta.insert(&mut txn, "terrain", terrain);
        }
        if time_of_day.is_some() || weather.is_some() {
            let mut env = read_env_map(&txn, &self.meta);
            if let Some(t) = time_of_day {
                env.insert("time".to_string(), Any::String(t.as_str().into()));
            }
            if let Some(w) = weather {
                env.insert("weather".to_string(), Any::String(w.as_str().into()));
            }
            self.meta
                .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
        }
        if let Some(b) = briefing {
            let b = b.trim();
            if !b.is_empty() {
                self.meta.insert(&mut txn, "briefing", b);
            }
        }
    }
}

impl MissionDocCore {
    /// [`Self::apply_row_meta`] treats blank / whitespace briefing as "not supplied" so boot hydrate cannot wipe a good value with an empty row. That guard is load-bearing and stays. This mutator is the explicit "set to empty" arm the editor uses after a successful PATCH of `missions.briefing` to `""` — without it, same-session Export ships the deleted text.
    pub fn clear_meta_briefing(&self) {
        let mut txn = self.begin();
        self.meta.remove(&mut txn, "briefing");
    }
}

impl MissionDocCore {
    /// Seed default meta if empty (mirrors `ydoc.seedMeta` + `DEFAULT_META`). No-op if meta exists.
    pub fn seed_meta(&self, id: &str, title: &str) {
        let mut txn = self.begin();
        if self.meta.len(&txn) > 0 {
            return;
        }
        self.meta.insert(&mut txn, "id", id);
        self.meta.insert(&mut txn, "title", title);
        self.meta.insert(&mut txn, "terrain", "everon");
        let mut env: HashMap<String, Any> = HashMap::new();
        env.insert("time".to_string(), Any::String("06:00".into()));
        env.insert("weather".to_string(), Any::String("clear".into()));
        self.meta
            .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
    }
}
