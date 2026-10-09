//! The runner that plays a two-party race once in each order.
//!
//! **Role:** makes "each race in both interleavings" one call: the scenario receives which
//! contender leads, arranges its contenders with [`Interleaving::arrange`], and the runner plays
//! it for both values and hands back both outcomes.
//! **Position:** used by the `controlled_races*` suites; the ordering itself comes from a failpoint
//! pause, a [`super::row_lock_barrier::RowLockHolder`] or a `tokio::sync::Barrier` inside the
//! scenario.
//! **Signals & state:** none; the scenario builds a fresh setup for each order.
//! **Invariants:** the orders run one after the other, `FirstLeads` then `SecondLeads`; each run
//! prints its order first, so a failure names the interleaving it happened in.

use std::future::Future;

/// Which of two contenders reaches the contested boundary first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interleaving {
    /// The first contender leads; the second follows.
    FirstLeads,
    /// The second contender leads; the first follows.
    SecondLeads,
}

impl Interleaving {
    /// Both orders, in the order [`run_in_both_orders`] plays them.
    pub(crate) const BOTH: [Self; 2] = [Self::FirstLeads, Self::SecondLeads];

    /// `(leader, follower)` for the contenders `(first, second)`; applied to a
    /// `(leader, follower)` pair it gives `(first, second)` back.
    pub(crate) fn arrange<T>(self, first: T, second: T) -> (T, T) {
        match self {
            Self::FirstLeads => (first, second),
            Self::SecondLeads => (second, first),
        }
    }

    /// The order's name, as failure output prints it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::FirstLeads => "first contender leads",
            Self::SecondLeads => "second contender leads",
        }
    }
}

/// Plays `scenario` once per [`Interleaving`] and returns the outcomes in [`Interleaving::BOTH`]
/// order.
///
/// ```ignore
/// let [first_leads, second_leads] = run_in_both_orders(|order| last_seat_race(order)).await;
/// assert_ne!(first_leads.winner, second_leads.winner);
/// ```
pub(crate) async fn run_in_both_orders<Scenario, Played, Outcome>(
    mut scenario: Scenario,
) -> [Outcome; 2]
where
    Scenario: FnMut(Interleaving) -> Played,
    Played: Future<Output = Outcome>,
{
    eprintln!("interleaving: {}", Interleaving::FirstLeads.name());
    let first_leads = scenario(Interleaving::FirstLeads).await;
    eprintln!("interleaving: {}", Interleaving::SecondLeads.name());
    let second_leads = scenario(Interleaving::SecondLeads).await;
    [first_leads, second_leads]
}
