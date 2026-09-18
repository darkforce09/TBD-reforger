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
