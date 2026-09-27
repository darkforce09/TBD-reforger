//! Generation-specific page cache, capped by both entry count and encoded size.
use std::{cell::RefCell, collections::VecDeque};
thread_local! {static CACHE:RefCell<VecDeque<(String,String)>>=const{RefCell::new(VecDeque::new())};}
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
