//! In-process publish/subscribe hub fanning messages out to SSE clients.
//!
//! Single-instance only — scaling out would back this with Postgres LISTEN/NOTIFY or Redis.
//! Backed by `tokio::sync::broadcast`: a bounded ring buffer per topic makes publishing
//! non-blocking (a subscriber too slow to drain lags and drops rather than stalling the
//! publisher), and a dropped receiver auto-unsubscribes, so no explicit cancel is needed.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

/// Per-topic ring-buffer capacity.
const TOPIC_BUFFER: usize = 16;

/// Fans messages out to subscribers grouped by topic.
pub struct Hub {
    topics: Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>,
}

impl Hub {
    /// Create an empty hub.
    pub fn new() -> Self {
        Self {
            topics: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a topic. The returned receiver auto-unsubscribes when dropped
    /// (the SSE handler holds it for the life of the connection).
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<Vec<u8>> {
        let mut topics = self.topics.lock().expect("hub lock");
        topics
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(TOPIC_BUFFER).0)
            .subscribe()
    }

    /// Deliver `msg` to all current subscribers of `topic`. Non-blocking: a slow
    /// subscriber whose buffer is full lags and drops rather than blocking the
    /// publisher. When no subscribers remain, the topic is pruned.
    pub fn publish(&self, topic: &str, msg: Vec<u8>) {
        let mut topics = self.topics.lock().expect("hub lock");
        if let Some(tx) = topics.get(topic)
            && tx.send(msg).is_err()
        {
            topics.remove(topic);
        }
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/hub.rs"]
mod tests;
