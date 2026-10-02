//! Generated scopes check promotion, release and seatability rules against independent oracles.

use super::*;
use crate::operations::models::reservation_quota::ReservationQuotaPool;
use proptest::prelude::*;

const ACCOUNTS: usize = 6;
const MISSIONS: [u128; 2] = [0xa0, 0xb0];

#[derive(Debug, Clone)]
struct TableOracle {
    available: Vec<bool>,
    member: Vec<bool>,
    current: Vec<Vec<bool>>,
    verified: Vec<Vec<bool>>,
    seat_ids: Vec<Uuid>,
}

fn account(index: usize) -> String {
    format!("account-{index}")
}

fn account_index(name: &str) -> usize {
    name.trim_start_matches("account-").parse().unwrap()
}

impl TableOracle {
    fn seat_index(&self, seat: Uuid) -> usize {
        self.seat_ids.iter().position(|id| *id == seat).unwrap()
    }
}

impl EligibilityOracle for TableOracle {
    fn available(&self, account: &str) -> bool {
        self.available[account_index(account)]
    }
    fn current_member(&self, account: &str) -> bool {
        self.member[account_index(account)]
    }
    fn current_admits(&self, account: &str, seat: Uuid) -> bool {
        self.current[account_index(account)][self.seat_index(seat)]
    }
    fn verified_admits(&self, account: &str, seat: Uuid) -> bool {
        self.verified[account_index(account)][self.seat_index(seat)]
    }
}

#[derive(Debug, Clone)]
struct Scope {
    oracle: TableOracle,
    seats: Vec<PlanningSeat>,
    reservations: Vec<ActiveReservation>,
    waiting: Vec<QueuedRegistration>,
    allocations: BTreeMap<String, ParticipantAllocationKind>,
    settings: QuotaSettings,
}

fn instant(offset: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000 + offset, 0).unwrap()
}

fn queued(serial: u128, account_id: usize, mission: Uuid, at: i64) -> QueuedRegistration {
    QueuedRegistration {
        registration: Uuid::from_u128(0x1000 + serial),
        account: account(account_id),
        mission,
        queue_entered_at: instant(at),
    }
}

#[allow(clippy::type_complexity)]
fn scope_strategy() -> impl Strategy<Value = Scope> {
    (
        prop::collection::vec(
            (
                0usize..2,
                0u8..2,
                0u8..2,
                0i64..3,
                prop::option::of(0usize..ACCOUNTS),
                any::<bool>(),
            ),
            1..=6,
        ),
        prop::collection::vec((0usize..ACCOUNTS, 0usize..2, 0i64..4), 0..=4),
        prop::collection::vec((0usize..ACCOUNTS, 0usize..2, 0i64..4), 0..=6),
        (
            prop::collection::vec(any::<bool>(), ACCOUNTS),
            prop::collection::vec(any::<bool>(), ACCOUNTS),
        ),
        (
            prop::collection::vec(prop::collection::vec(any::<bool>(), 6), ACCOUNTS),
            prop::collection::vec(prop::collection::vec(any::<bool>(), 6), ACCOUNTS),
        ),
        (
            prop::array::uniform3(prop::option::of(0u32..4)),
            prop::array::uniform3(-1i64..=1),
            0u32..8,
        ),
        prop::collection::vec(0u8..4, ACCOUNTS),
    )
        .prop_map(
            |(
                seat_specs,
                holder_specs,
                waiting_specs,
                (available, member),
                (current, extra),
                (limits, openings, max_slots),
                kinds,
            )| {
                let missions: Vec<Uuid> = MISSIONS.iter().map(|m| Uuid::from_u128(*m)).collect();
                let mut seats = Vec::new();
                let mut reservations = Vec::new();
                let mut participants: BTreeSet<(Uuid, String)> = BTreeSet::new();
                let mut serial = 0u128;
                for (index, (mission, faction, squad, slot_index, occupant, reserved)) in
                    seat_specs.into_iter().enumerate()
                {
                    let mission = missions[mission];
                    let id = Uuid::from_u128(0x100 + index as u128);
                    // One account occupies at most one seat per attachment.
                    let occupant = occupant
                        .map(account)
                        .filter(|a| participants.insert((mission, a.clone())));
                    if let (Some(occupant), true) = (&occupant, reserved) {
                        serial += 1;
                        reservations.push(ActiveReservation {
                            queued: queued(serial, account_index(occupant), mission, 0),
                            seat: Some(id),
                        });
                    }
                    seats.push(PlanningSeat {
                        id,
                        mission,
                        faction: format!("F{faction}"),
                        squad: format!("S{squad}"),
                        slot_index,
                        occupant,
                    });
                }
                for (holder, mission, at) in holder_specs {
                    let mission = missions[mission];
                    if participants.insert((mission, account(holder))) {
                        serial += 1;
                        reservations.push(ActiveReservation {
                            queued: queued(serial, holder, mission, at),
                            seat: None,
                        });
                    }
                }
                let mut waiting = Vec::new();
                for (waiter, mission, at) in waiting_specs {
                    let mission = missions[mission];
                    if participants.insert((mission, account(waiter))) {
                        serial += 1;
                        waiting.push(queued(serial, waiter, mission, at));
                    }
                }
                // Exactly the accounts with a reservation or an occupied seat hold an allocation.
                let present: BTreeSet<String> = reservations
                    .iter()
                    .map(|r| r.queued.account.clone())
                    .chain(seats.iter().filter_map(|s| s.occupant.clone()))
                    .collect();
                let allocations = present
                    .into_iter()
                    .map(|a| {
                        let kind = match kinds[account_index(&a)] {
                            0 => ParticipantAllocationKind::Member,
                            1 => ParticipantAllocationKind::Guest,
                            2 => ParticipantAllocationKind::Open,
                            _ => ParticipantAllocationKind::LegacyUnclassified,
                        };
                        (a, kind)
                    })
                    .collect();
                let seat_ids: Vec<Uuid> = seats.iter().map(|s| s.id).collect();
                let verified = current
                    .iter()
                    .zip(&extra)
                    .map(|(row, more)| row.iter().zip(more).map(|(c, e)| *c || *e).collect())
                    .collect();
                let pools: Vec<ReservationQuotaPool> = limits
                    .into_iter()
                    .zip(openings)
                    .map(|(seats, offset)| ReservationQuotaPool {
                        seats,
                        opens_at: instant(offset * 3600),
                    })
                    .collect();
                Scope {
                    oracle: TableOracle {
                        available,
                        member,
                        current,
                        verified,
                        seat_ids,
                    },
                    seats,
                    reservations,
                    waiting,
                    allocations,
                    settings: QuotaSettings {
                        quotas: ReservationQuotas {
                            member: pools[0].clone(),
                            guest: pools[1].clone(),
                            open: pools[2].clone(),
                        },
                        max_slots,
                        now: instant(0),
                    },
                }
            },
        )
}

fn plan(scope: &Scope) -> ReservationPlan {
    ReservationPlan::new(
        scope.seats.clone(),
        &scope.reservations,
        scope.allocations.clone(),
        scope.settings.clone(),
    )
    .unwrap()
}

fn free_seats_after(scope: &Scope, promotions: &[PlannedPromotion]) -> Vec<(Uuid, Uuid)> {
    scope
        .seats
        .iter()
        .filter(|seat| seat.occupant.is_none() && !promotions.iter().any(|p| p.seat == seat.id))
        .map(|seat| (seat.mission, seat.id))
        .collect()
}

fn mission_holders_seatable(scope: &Scope, mission: Uuid, promotions: &[PlannedPromotion]) -> bool {
    let free: Vec<Uuid> = free_seats_after(scope, promotions)
        .into_iter()
        .filter(|(seat_mission, _)| *seat_mission == mission)
        .map(|(_, seat)| seat)
        .collect();
    let mut matrix = SeatEligibility::new(free.len());
    for holder in scope
        .reservations
        .iter()
        .filter(|r| r.seat.is_none() && r.queued.mission == mission)
    {
        matrix.push_holder(|index| {
            scope
                .oracle
                .verified_admits(&holder.queued.account, free[index])
        });
    }
    matrix.all_holders_seatable()
}

#[test]
fn planned_promotions_respect_quota_capacity_eligibility_and_seatability() {
    crate::property_evidence::run_property(
        "planned_promotions_respect_quota_capacity_eligibility_and_seatability",
        512,
        &scope_strategy(),
        |scope| {
            let mut planner = plan(&scope);
            let before = planner.usage();
            let promotions = planner
                .plan_promotions(&scope.oracle, &scope.waiting)
                .unwrap();
            let mut taken = BTreeSet::new();
            let mut new_allocations = BTreeMap::new();
            for promotion in &promotions {
                let seat = scope.seats.iter().find(|s| s.id == promotion.seat).unwrap();
                prop_assert!(
                    seat.occupant.is_none() && taken.insert(seat.id),
                    "seat reused"
                );
                prop_assert_eq!(seat.mission, promotion.mission);
                prop_assert!(scope.oracle.available(&promotion.account));
                prop_assert!(
                    scope
                        .oracle
                        .current_admits(&promotion.account, promotion.seat)
                );
                let already = scope.allocations.contains_key(&promotion.account)
                    || new_allocations.contains_key(&promotion.account);
                prop_assert_eq!(promotion.new_allocation.is_none(), already);
                if let Some(kind) = promotion.new_allocation {
                    new_allocations.insert(promotion.account.clone(), kind);
                    let own_pool = if scope.oracle.current_member(&promotion.account) {
                        ReservationQuotaKind::Member
                    } else {
                        ReservationQuotaKind::Guest
                    };
                    prop_assert!(kind == ReservationQuotaKind::Open || kind == own_pool);
                }
            }
            let after = planner.usage();
            prop_assert_eq!(
                after.total().unwrap(),
                before.total().unwrap() + new_allocations.len() as u64
            );
            for kind in [
                ReservationQuotaKind::Member,
                ReservationQuotaKind::Guest,
                ReservationQuotaKind::Open,
            ] {
                if after.count(kind) > before.count(kind)
                    && let Some(limit) = scope.settings.quotas.pool(kind).seats
                {
                    prop_assert!(after.count(kind) <= u64::from(limit));
                    prop_assert!(scope.settings.now >= scope.settings.quotas.pool(kind).opens_at);
                }
            }
            if scope.settings.max_slots != 0 && !new_allocations.is_empty() {
                prop_assert!(after.total().unwrap() <= u64::from(scope.settings.max_slots));
            }
            for mission in MISSIONS.iter().map(|m| Uuid::from_u128(*m)) {
                let seats = scope.seats.iter().filter(|s| s.mission == mission).count();
                let mut participants: BTreeSet<&str> = scope
                    .seats
                    .iter()
                    .filter(|s| s.mission == mission)
                    .filter_map(|s| s.occupant.as_deref())
                    .collect();
                participants.extend(
                    scope
                        .reservations
                        .iter()
                        .filter(|r| r.queued.mission == mission)
                        .map(|r| r.queued.account.as_str()),
                );
                let before_count = participants.len();
                participants.extend(
                    promotions
                        .iter()
                        .filter(|p| p.mission == mission)
                        .map(|p| p.account.as_str()),
                );
                prop_assert!(participants.len() <= seats.max(before_count));
            }
            // Promotions never strand a holder; a mission already awaiting re-evaluation because
            // its holders cannot all be seated receives no promotion at all.
            for mission in MISSIONS.iter().map(|m| Uuid::from_u128(*m)) {
                if mission_holders_seatable(&scope, mission, &[]) {
                    prop_assert!(mission_holders_seatable(&scope, mission, &promotions));
                } else {
                    prop_assert!(!promotions.iter().any(|p| p.mission == mission));
                }
            }
            // Maximality: after one pass nobody else could be promoted.
            let promoted: BTreeSet<Uuid> = promotions.iter().map(|p| p.registration).collect();
            let remaining: Vec<QueuedRegistration> = scope
                .waiting
                .iter()
                .filter(|w| !promoted.contains(&w.registration))
                .cloned()
                .collect();
            prop_assert!(
                planner
                    .plan_promotions(&scope.oracle, &remaining)
                    .unwrap()
                    .is_empty()
            );
            Ok(())
        },
    );
}

#[test]
fn promotion_order_is_independent_of_input_permutation() {
    crate::property_evidence::run_property(
        "promotion_order_is_independent_of_input_permutation",
        512,
        &(
            scope_strategy(),
            prop::collection::vec(any::<usize>(), 0..12),
        ),
        |(scope, swaps)| {
            let expected = plan(&scope)
                .plan_promotions(&scope.oracle, &scope.waiting)
                .unwrap();
            let mut shuffled = scope.clone();
            for (index, swap) in swaps.iter().enumerate() {
                if !shuffled.seats.is_empty() {
                    let n = shuffled.seats.len();
                    shuffled.seats.swap(index % n, swap % n);
                }
                if !shuffled.reservations.is_empty() {
                    let n = shuffled.reservations.len();
                    shuffled.reservations.swap(swap % n, index % n);
                }
                if !shuffled.waiting.is_empty() {
                    let n = shuffled.waiting.len();
                    shuffled.waiting.swap(index % n, swap % n);
                }
            }
            let actual = plan(&shuffled)
                .plan_promotions(&shuffled.oracle, &shuffled.waiting)
                .unwrap();
            prop_assert_eq!(actual, expected);
            Ok(())
        },
    );
}

#[test]
fn eligibility_releases_are_confirmed_minimal_and_restore_seatability() {
    crate::property_evidence::run_property(
        "eligibility_releases_are_confirmed_minimal_and_restore_seatability",
        512,
        &scope_strategy(),
        |scope| {
            let mut planner = plan(&scope);
            let releases = planner
                .plan_eligibility_releases(&scope.oracle, &scope.reservations)
                .unwrap();
            let released: BTreeSet<Uuid> = releases.iter().map(|r| r.registration).collect();
            prop_assert_eq!(
                released.len(),
                releases.len(),
                "a reservation was released twice"
            );
            for release in &releases {
                let reservation = scope
                    .reservations
                    .iter()
                    .find(|r| r.queued.registration == release.registration)
                    .unwrap();
                match release.cause {
                    ReleaseCause::AccountUnavailable => {
                        prop_assert!(!scope.oracle.available(&release.account))
                    }
                    ReleaseCause::PolicyDenied => {
                        prop_assert!(scope.oracle.available(&release.account))
                    }
                }
                if let (Some(seat), ReleaseCause::PolicyDenied) = (reservation.seat, release.cause)
                {
                    prop_assert!(!scope.oracle.verified_admits(&release.account, seat));
                }
            }
            // Every kept reservation is admitted by last verified facts and holders stay seatable.
            let kept: Vec<&ActiveReservation> = scope
                .reservations
                .iter()
                .filter(|r| !released.contains(&r.queued.registration))
                .collect();
            for reservation in &kept {
                prop_assert!(scope.oracle.available(&reservation.queued.account));
                if let Some(seat) = reservation.seat {
                    prop_assert!(
                        scope
                            .oracle
                            .verified_admits(&reservation.queued.account, seat)
                    );
                }
            }
            for mission in MISSIONS.iter().map(|m| Uuid::from_u128(*m)) {
                let free: Vec<Uuid> = scope
                    .seats
                    .iter()
                    .filter(|s| s.mission == mission)
                    .filter(|s| {
                        s.occupant.is_none() || releases.iter().any(|r| r.seat == Some(s.id))
                    })
                    .map(|s| s.id)
                    .collect();
                let mut matrix = SeatEligibility::new(free.len());
                let mut holders: Vec<&&ActiveReservation> = kept
                    .iter()
                    .filter(|r| r.seat.is_none() && r.queued.mission == mission)
                    .collect();
                holders.sort_by(|l, r| l.queued.queue_order(&r.queued));
                for holder in &holders {
                    matrix.push_holder(|index| {
                        scope
                            .oracle
                            .verified_admits(&holder.queued.account, free[index])
                    });
                }
                prop_assert!(matrix.all_holders_seatable());
            }
            // An allocation is released exactly when its participant has nothing left in the event.
            for release in releases.iter().filter(|r| r.releases_allocation) {
                prop_assert!(!kept.iter().any(|r| r.queued.account == release.account));
                prop_assert!(
                    !scope
                        .seats
                        .iter()
                        .any(|s| s.occupant.as_deref() == Some(release.account.as_str())
                            && !scope.reservations.iter().any(|r| r.seat == Some(s.id)))
                );
            }
            let freed = releases.iter().filter(|r| r.releases_allocation).count() as u64;
            prop_assert_eq!(
                planner.usage().total().unwrap() + freed,
                plan(&scope).usage().total().unwrap()
            );
            Ok(())
        },
    );
}

#[test]
fn a_promotion_skips_the_seat_a_seatless_holder_needs() {
    let mission = Uuid::from_u128(MISSIONS[0]);
    let seats = vec![
        PlanningSeat {
            id: Uuid::from_u128(1),
            mission,
            faction: "A".into(),
            squad: "A".into(),
            slot_index: 0,
            occupant: None,
        },
        PlanningSeat {
            id: Uuid::from_u128(2),
            mission,
            faction: "A".into(),
            squad: "A".into(),
            slot_index: 1,
            occupant: None,
        },
    ];
    // account-0 holds a seatless place and only fits seat 1; account-1 waits and fits both.
    let oracle = TableOracle {
        available: vec![true; ACCOUNTS],
        member: vec![true; ACCOUNTS],
        current: vec![
            vec![true, false],
            vec![true, true],
            vec![false; 2],
            vec![false; 2],
            vec![false; 2],
            vec![false; 2],
        ],
        verified: vec![
            vec![true, false],
            vec![true, true],
            vec![false; 2],
            vec![false; 2],
            vec![false; 2],
            vec![false; 2],
        ],
        seat_ids: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
    };
    let holder = ActiveReservation {
        queued: queued(1, 0, mission, 0),
        seat: None,
    };
    let settings = QuotaSettings {
        quotas: ReservationQuotas {
            member: ReservationQuotaPool {
                seats: None,
                opens_at: instant(-10),
            },
            guest: ReservationQuotaPool {
                seats: Some(0),
                opens_at: instant(-10),
            },
            open: ReservationQuotaPool {
                seats: Some(0),
                opens_at: instant(-10),
            },
        },
        max_slots: 0,
        now: instant(0),
    };
    let mut planner = ReservationPlan::new(
        seats,
        &[holder],
        BTreeMap::from([(account(0), ParticipantAllocationKind::Member)]),
        settings,
    )
    .unwrap();
    let promotions = planner
        .plan_promotions(&oracle, &[queued(2, 1, mission, 1)])
        .unwrap();
    assert_eq!(promotions.len(), 1);
    assert_eq!(promotions[0].seat, Uuid::from_u128(2));
    assert_eq!(
        promotions[0].new_allocation,
        Some(ReservationQuotaKind::Member)
    );
}

#[test]
fn an_empty_scope_has_no_usage_promotions_or_releases() {
    let pool = ReservationQuotaPool {
        seats: None,
        opens_at: instant(0),
    };
    let settings = QuotaSettings {
        quotas: ReservationQuotas {
            member: pool.clone(),
            guest: pool.clone(),
            open: pool,
        },
        max_slots: 0,
        now: instant(0),
    };
    let oracle = TableOracle {
        available: vec![true; ACCOUNTS],
        member: vec![true; ACCOUNTS],
        current: vec![Vec::new(); ACCOUNTS],
        verified: vec![Vec::new(); ACCOUNTS],
        seat_ids: Vec::new(),
    };
    let mut plan = ReservationPlan::new(Vec::new(), &[], BTreeMap::new(), settings).unwrap();
    assert_eq!(plan.usage(), ReservationQuotaUsage::default());
    let mission = Uuid::from_u128(MISSIONS[0]);
    assert!(
        plan.plan_promotions(&oracle, &[queued(1, 0, mission, 0)])
            .unwrap()
            .is_empty()
    );
    assert!(
        plan.plan_eligibility_releases(&oracle, &[])
            .unwrap()
            .is_empty()
    );
}
