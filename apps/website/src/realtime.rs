//! In-process publish/subscribe hub — Rust port of `internal/realtime`.
//!
//! Fans server-status updates out to SSE clients. Single-instance only (same caveat
//! as the Go original — scale-out would back this with Postgres LISTEN/NOTIFY or
//! Redis). Backed by `tokio::sync::broadcast`: a bounded ring buffer per topic gives
//! Go's "buffer 16, non-blocking, drop slow subscribers" behavior for free, and a
//! dropped receiver auto-unsubscribes (no explicit cancel needed).

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

/// Per-topic ring-buffer capacity (matches Go's `make(chan []byte, 16)`).
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
    /// publisher. When no subscribers remain, the topic is pruned (mirrors Go's
    /// empty-topic delete on the last unsubscribe).
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_delivers() {
        let h = Hub::new();
        let mut rx = h.subscribe("topic-a");
        h.publish("topic-a", b"hello".to_vec());
        assert_eq!(rx.recv().await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn topic_isolation() {
        let h = Hub::new();
        let mut rx = h.subscribe("topic-a");
        h.publish("topic-b", b"nope".to_vec());
        assert!(rx.try_recv().is_err(), "no cross-topic delivery");
    }

    #[tokio::test]
    async fn unsubscribe_stops_delivery() {
        let h = Hub::new();
        let rx = h.subscribe("topic-a");
        drop(rx); // cancel
        h.publish("topic-a", b"x".to_vec()); // must not panic
    }
}
