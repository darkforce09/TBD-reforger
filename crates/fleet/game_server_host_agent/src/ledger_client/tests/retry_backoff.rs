use super::*;

#[test]
fn many_failures_never_overflow() {
    let mut backoff = JitteredBackoff::new(BackoffPolicy {
        initial: Duration::from_secs(1),
        maximum: Duration::from_secs(60),
    });
    for _ in 0..200 {
        assert!(backoff.next_delay() <= Duration::from_secs(60));
    }
}
