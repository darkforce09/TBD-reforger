//! Which tickets count as complete, for the platform wave scheduler.
//!
//! **Role:** answers "is this ticket shipped" from a snapshot of every ticket's status.
//! **Position:** built from the wave lock's ticket views ([`ShippingStatus::load_repo`]) or a
//! registry value; the xtask platform wave ledger reads it.
//! **Signals & state:** the snapshot it was built from; it never rereads the files.
//! **Invariants:** malformed input poisons the snapshot, and a poisoned snapshot answers "not
//! shipped" for every ticket; `cancelled` counts as shipped.

use serde_json::Value;
use std::path::Path;
use ticket_model::TicketId;

/// A snapshot of ticket statuses by id. Invalid input poisons every lookup; shipped and
/// cancelled tickets count as complete.
pub struct ShippingStatus {
    poisoned: bool,
    by_id: std::collections::HashMap<TicketId, Option<String>>,
}

impl ShippingStatus {
    /// The snapshot of a registry value. A missing `tickets` array, a row that is not an object
    /// or a row without an `id` poisons it; a row whose id is not a string is skipped.
    pub fn from_value(v: &Value) -> ShippingStatus {
        let mut r = ShippingStatus {
            poisoned: true,
            by_id: Default::default(),
        };
        let Some(tickets) = v.get("tickets").and_then(Value::as_array) else {
            return r;
        };
        for t in tickets {
            let Some(obj) = t.as_object() else { return r };
            let Some(id) = obj.get("id") else { return r };
            let key = match id.as_str() {
                Some(s) => TicketId::new(s),
                None => continue,
            };
            let status = obj
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string);
            r.by_id.insert(key, status);
        }
        r.poisoned = false;
        r
    }

    /// The snapshot of the registry JSON file at `path`; poisoned when the file cannot be read or
    /// parsed.
    #[allow(dead_code)]
    pub fn load(path: &Path) -> ShippingStatus {
        let Ok(body) = std::fs::read_to_string(path) else {
            return ShippingStatus {
                poisoned: true,
                by_id: Default::default(),
            };
        };
        let Ok(v) = serde_json::from_str::<Value>(&body) else {
            return ShippingStatus {
                poisoned: true,
                by_id: Default::default(),
            };
        };
        Self::from_value(&v)
    }

    /// The snapshot of the checkout at `root`, from the wave lock's ticket views; poisoned when
    /// the views cannot be loaded.
    pub fn load_repo(root: &Path) -> ShippingStatus {
        match ticket_wave_lock::load_views(root) {
            Ok(views) => ShippingStatus {
                poisoned: false,
                by_id: views
                    .into_iter()
                    .map(|v| (v.id, Some(v.status.as_str().to_string())))
                    .collect(),
            },
            Err(_) => ShippingStatus {
                poisoned: true,
                by_id: Default::default(),
            },
        }
    }

    /// Whether `id` is `shipped` or `cancelled`; `false` for an unknown id, a row without a
    /// status, or a poisoned snapshot.
    pub fn is_shipped(&self, id: &TicketId) -> bool {
        if self.poisoned {
            return false;
        }
        match self.by_id.get(id) {
            // The row exists but carries no status string: not shipped.
            Some(None) => false,
            Some(Some(s)) => s == "shipped" || s == "cancelled",
            None => false,
        }
    }
}
