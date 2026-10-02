//! Bounded session state retains exact source identities and reading positions.
use std::{
    cell::RefCell,
    collections::{BTreeSet, VecDeque},
};
/// The remembered reading position of one resource data surface: the card grid's scroll offset
/// in pixels, the measured height of each card row, the column count those heights belong to,
/// and the `(container, property)` pairs whose fields are expanded.
#[derive(Clone, Default, Debug)]
pub struct ResourceMemory {
    pub scroll_top: f64,
    pub row_heights: std::collections::BTreeMap<usize, f64>,
    pub columns: usize,
    pub expanded: BTreeSet<(String, String)>,
}
thread_local! {
    static RESOURCES: RefCell<VecDeque<(String,ResourceMemory)>>=const{RefCell::new(VecDeque::new())};
    static CATALOGS: RefCell<VecDeque<(String,i32)>>=const{RefCell::new(VecDeque::new())};
}
/// The reading position remembered under `key`, or an empty [`ResourceMemory`] when none is.
pub fn restore(key: &str) -> ResourceMemory {
    RESOURCES.with(|memory| {
        memory
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    })
}
/// Remembers `value` under `key` as the most recent entry, keeping at most eight reading
/// positions by forgetting the least recently saved.
pub fn save(key: String, value: ResourceMemory) {
    RESOURCES.with(|memory| {
        let mut memory = memory.borrow_mut();
        memory.retain(|(k, _)| k != &key);
        memory.push_back((key, value));
        while memory.len() > 8 {
            memory.pop_front();
        }
    });
}
/// The scroll offset in pixels remembered for the catalog list under `key`, or `0` when none is.
pub fn catalog_scroll(key: &str) -> i32 {
    CATALOGS.with(|m| {
        m.borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    })
}
/// Remembers `value` as the scroll offset of the catalog list under `key`, keeping at most eight
/// lists by forgetting the least recently saved.
pub fn save_catalog_scroll(key: String, value: i32) {
    CATALOGS.with(|m| {
        let mut m = m.borrow_mut();
        m.retain(|(k, _)| k != &key);
        m.push_back((key, value));
        while m.len() > 8 {
            m.pop_front();
        }
    });
}
#[cfg(test)]
#[path = "tests/browsing_state.rs"]
mod tests;
