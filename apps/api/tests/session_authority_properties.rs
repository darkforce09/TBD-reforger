//! Generated session states and refresh-family races hold the production session authority to
//! an independent oracle.
//!
//! * `protected_actions_require_effective_session_authority` — every generated account and
//!   session state is presented to one route per rank gate through the production-built
//!   router: the production `authorize_session` and role gates allow exactly when the oracle's
//!   session is live and its effective rank reaches the gate, refuse with 401 or 403 exactly
//!   as the oracle says, and the current profile reports the oracle's rank and membership
//!   standing.
//! * `refresh_replay_revokes_concurrently_issued_successor` — generated rotate, replay and
//!   logout sequences over two families of one account, with pairs released together behind a
//!   barrier, run through the production `rotate_session` and `logout_session`: every
//!   outcome fits a serialisation of the family model, no family ever holds more than one live
//!   successor, a replay revokes every family including successors issued concurrently, and
//!   every access credential of a revoked family answers 401.
//!
//! Each case seeds fresh accounts with unique ids; one database, one pair of application
//! states and one pair of routers serve every case of a property.

mod common;
mod session_authority_support;

use api_identity_and_access::services::{
    session_issuance::issue_session,
    session_rotation::{logout_session, rotate_session},
};
use api_state::AppState;
use axum::http::StatusCode;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use session_authority_support::{
    PropertyWorld,
    authority_cases::{self, AuthorityCase, Rank, ServingEnvironment},
    authority_oracle::ExpectedAuthority,
    persisted_families::PersistedFamilies,
    refresh_families::{
        self, AccountModel, FAMILY_COUNT, FamilyModel, ObservedOutcome, OperationKind,
        PresentedOperation, RefreshStep,
    },
    send_get,
};
use std::sync::Arc;
use tokio::sync::Barrier;

/// Cases each property executes.
const CASES: u32 = 256;

/// One side-effect-free route per rank gate. The authenticated gate admits guests; no route
/// gates on the enlisted rank.
const RANK_GATED_ROUTES: [(Rank, &str); 4] = [
    (Rank::Guest, "/api/v1/me"),
    (Rank::Leader, "/api/v1/members?limit=1"),
    (Rank::MissionMaker, "/api/v1/factions"),
    (Rank::Admin, "/api/v1/admin/audit-logs?limit=1"),
];

#[test]
fn protected_actions_require_effective_session_authority() {
    let runtime = tokio::runtime::Runtime::new().expect("build the test runtime");
    let world = runtime.block_on(PropertyWorld::open());
    api_property_evidence::run_property(
        "protected_actions_require_effective_session_authority",
        CASES,
        &authority_cases::authority_case(),
        |case| runtime.block_on(check_protected_actions(&world, case)),
    );
}

#[test]
fn refresh_replay_revokes_concurrently_issued_successor() {
    let runtime = tokio::runtime::Runtime::new().expect("build the test runtime");
    let world = runtime.block_on(PropertyWorld::open());
    api_property_evidence::run_property(
        "refresh_replay_revokes_concurrently_issued_successor",
        CASES,
        &refresh_families::refresh_steps(),
        |steps| runtime.block_on(check_refresh_families(&world, steps)),
    );
}

async fn check_protected_actions(world: &PropertyWorld, case: AuthorityCase) -> TestCaseResult {
    let session = authority_cases::realise(world, &case).await;
    let expected = ExpectedAuthority::of(&case);
    let router = world.router(case.environment);
    for (gate, uri) in RANK_GATED_ROUTES {
        let (status, body) = send_get(router, uri, &session.access_credential).await;
        prop_assert_eq!(
            status,
            expected.status_for(gate),
            "GET {} as {} (expected {:?}): {}",
            uri,
            session.discord_id,
            expected,
            body
        );
        if status != StatusCode::OK {
            prop_assert!(
                body["error"]
                    .as_str()
                    .is_some_and(|message| !message.is_empty()),
                "GET {} refused without the error envelope: {}",
                uri,
                body
            );
        } else if gate == Rank::Guest {
            prop_assert_eq!(body["user"]["role"].as_str(), Some(expected.rank.wire()));
            prop_assert_eq!(
                body["membership_stale"].as_bool(),
                Some(expected.membership_stale)
            );
            prop_assert_eq!(
                body["membership_override_active"].as_bool(),
                Some(expected.override_active)
            );
        }
    }
    Ok(())
}

async fn check_refresh_families(world: &PropertyWorld, steps: Vec<RefreshStep>) -> TestCaseResult {
    let state = &world.development;
    let discord_id = session_authority_support::seed_account(&state.pool, "refresh-family").await;
    common::fixtures::seed_membership(
        &state.pool,
        &discord_id,
        state.cfg.discord_guild_id.as_str(),
        "enlisted",
    )
    .await;
    let mut families = Vec::with_capacity(FAMILY_COUNT);
    for _ in 0..FAMILY_COUNT {
        let (access, _, refresh) = issue_session(
            state,
            &api_identifiers::DiscordUserId::new(discord_id.as_str()),
        )
        .await
        .expect("a fresh member signs in");
        let session_id = state.jwt.parse(&access).expect("issued credential").sid;
        families.push(FamilyModel::issued(
            session_id.into_inner(),
            access,
            refresh,
        ));
    }
    let mut candidates = vec![AccountModel { families }];
    for step in steps {
        let model = &candidates[0];
        let (explained, record) = match step {
            RefreshStep::Alone(operation) => {
                let operation = model.present(operation);
                let outcome = perform(state, &operation).await;
                (
                    AccountModel::after_alone(&candidates, &operation, &outcome),
                    format!("{operation:?} -> {outcome:?}"),
                )
            }
            RefreshStep::ConcurrentPair(first, second) => {
                let (first, second) = (model.present(first), model.present(second));
                let (first_outcome, second_outcome) =
                    perform_concurrently(state, first.clone(), second.clone()).await;
                (
                    AccountModel::after_either_order(
                        &candidates,
                        (&first, &first_outcome),
                        (&second, &second_outcome),
                    ),
                    format!("{first:?} -> {first_outcome:?} | {second:?} -> {second_outcome:?}"),
                )
            }
        };
        prop_assert!(
            !explained.is_empty(),
            "no serialisation of the family model explains {}",
            record
        );
        candidates = explained;
        let persisted = PersistedFamilies::load(&state.pool, &discord_id).await;
        prop_assert!(
            persisted.sessions_breaking_single_successor.is_empty(),
            "after {}, sessions hold more than one live successor or a live token while revoked: {:?}",
            record,
            persisted
        );
        candidates.retain(|model| {
            model.live_sessions() == persisted.live_sessions
                && model.live_token_hashes() == persisted.live_token_hashes
        });
        prop_assert!(
            !candidates.is_empty(),
            "after {}, the persisted families match no model state: {:?}",
            record,
            persisted
        );
    }
    let router = world.router(ServingEnvironment::Development);
    for family in &candidates[0].families {
        let expected = if family.revoked {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::OK
        };
        for access_credential in &family.access_credentials {
            let (status, body) = send_get(router, "/api/v1/me", access_credential).await;
            prop_assert_eq!(
                status,
                expected,
                "access credential of family {} (revoked: {}): {}",
                family.session_id,
                family.revoked,
                body
            );
        }
    }
    Ok(())
}

/// Present `operation` to the production session service it names.
async fn perform(state: &AppState, operation: &PresentedOperation) -> ObservedOutcome {
    match operation.kind {
        OperationKind::Rotate => match rotate_session(state, &operation.token).await {
            Ok((access_credential, _, refresh_token)) => ObservedOutcome::Rotated {
                access_credential,
                refresh_token,
            },
            Err(refusal) => ObservedOutcome::Refused(refusal.status),
        },
        OperationKind::Logout => match logout_session(state, &operation.token).await {
            Ok(()) => ObservedOutcome::LoggedOut,
            Err(failure) => ObservedOutcome::Refused(failure.status),
        },
    }
}

/// Release both operations together behind a two-party barrier and collect both outcomes.
async fn perform_concurrently(
    state: &AppState,
    first: PresentedOperation,
    second: PresentedOperation,
) -> (ObservedOutcome, ObservedOutcome) {
    let barrier = Arc::new(Barrier::new(2));
    let launch = |operation: PresentedOperation| {
        let (state, barrier) = (state.clone(), barrier.clone());
        tokio::spawn(async move {
            barrier.wait().await;
            perform(&state, &operation).await
        })
    };
    let (first, second) = (launch(first), launch(second));
    (
        first
            .await
            .expect("the first concurrent operation completes"),
        second
            .await
            .expect("the second concurrent operation completes"),
    )
}
