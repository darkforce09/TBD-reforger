//! Generation-specific page cache, capped by both entry count and encoded size.
use std::{cell::RefCell, collections::VecDeque};
thread_local! {static CACHE:RefCell<VecDeque<(String,String)>>=const{RefCell::new(VecDeque::new())};}
/// The cached response body for the request URL `key`, which becomes the most recently used
/// entry.
pub fn get(key: &str) -> Option<String> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let i = cache.iter().position(|(k, _)| k == key)?;
        let entry = cache.remove(i)?;
        let text = entry.1.clone();
        cache.push_back(entry);
        Some(text)
    })
}
/// Caches the response body `value` under the request URL `key`, evicting the least recently
/// used entries so at most 32 entries and 4 MiB of text remain. `status` requests and
/// `generation=latest` requests, whose answers change between exports, and bodies over 256 KiB
/// are never cached.
pub fn put(key: String, value: String) {
    if key.contains("/status?") || key.contains("generation=latest") || value.len() > 262144 {
        return;
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        while cache.len() >= 32
            || cache.iter().map(|e| e.1.len()).sum::<usize>() + value.len() > 4 * 1024 * 1024
        {
            cache.pop_front();
        }
        cache.push_back((key, value));
    });
}
