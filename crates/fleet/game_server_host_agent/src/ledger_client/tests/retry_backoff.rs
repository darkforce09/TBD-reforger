use super::*;

const POLICY: BackoffPolicy = BackoffPolicy {
    initial: Duration::from_millis(100),
    maximum: Duration::from_millis(1_000),
};

fn within(delay: Duration, step_milliseconds: u64) -> bool {
    let step = Duration::from_millis(step_milliseconds);
    delay >= step / 2 && delay <= step
}

#[test]
fn delays_double_until_the_maximum() {
    let mut backoff = JitteredBackoff::new(POLICY);
    for step in [100, 200, 400, 800, 1_000, 1_000, 1_000] {
        let delay = backoff.next_delay();
        assert!(within(delay, step), "{delay:?} outside the {step} ms step");
    }
}

#[test]
fn delays_are_jittered_within_the_upper_half_of_the_step() {
    let delays: Vec<Duration> = (0..64)
        .map(|_| JitteredBackoff::new(POLICY).next_delay())
        .collect();
    assert!(delays.iter().all(|delay| within(*delay, 100)));
    assert!(
        delays.iter().any(|delay| *delay != delays[0]),
        "64 draws from a 50 ms span are not all equal"
    );
}

#[test]
fn a_success_starts_the_steps_again() {
    let mut backoff = JitteredBackoff::new(POLICY);
    for _ in 0..10 {
        backoff.next_delay();
    }
    backoff.reset();
    assert!(within(backoff.next_delay(), 100));
}

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
