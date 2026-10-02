//! Cached shipping status for ticket scheduling.

use serde_json::Value;
use std::path::Path;

/// Invalid input poisons all lookups; shipped and cancelled tickets count as complete.
pub struct ShippingStatus {
    poisoned: bool,
    by_id: std::collections::HashMap<String, Option<String>>,
}

impl ShippingStatus {
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
                Some(s) => s.to_string(),
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
    pub fn load_repo(root: &Path) -> ShippingStatus {
        match crate::wave_lock::load_views(root) {
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
    pub fn is_shipped(&self, id: &str) -> bool {
        if self.poisoned {
            return false;
        }
        match self.by_id.get(id) {
            // Matched, but `t[0]['status']` raised: not shipped.
            Some(None) => false,
            Some(Some(s)) => s == "shipped" || s == "cancelled",
            None => false,
        }
    }
}

#[cfg(test)]
#[path = "tests/shipping_status/mod.rs"]
mod tests;
