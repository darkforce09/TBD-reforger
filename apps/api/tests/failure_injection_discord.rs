//! Failure injection around the Discord member read of a membership refresh.
//!
//! **Role:** proves that a refresh failed before the Discord request makes no request and records
//! nothing, and a refresh failed after Discord answered records nothing; in both the refresh lease
//! stays held until it expires, the refresh after the expiry applies the observation once, a late
//! completion under the failed lease cannot apply it again, and a further refresh of the same
//! answer leaves the recorded membership as it is.
//! **Position:** its own test binary over `reconcile_one` of
//! `api_identity_and_access::services::discord_rest_reconciliation`, which carries the failpoints
//! `DiscordRoleSyncBeforeEffect` and `DiscordRoleSyncAfterEffect`; Discord is a local HTTP fake
//! reached through `DiscordService::set_api_base`, counting the member reads it answers.
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case holds for its whole body; this binary's private database from `tests/common`, whose
//! membership snapshots each case resets to its one account.
//! **Invariants:** a lease and the shared request budget are made to lapse by moving
//! `lease_expires_at`, `next_refresh_at` and `next_request_at` into the past, the instants the
//! claim compares with `clock_timestamp()`, never by sleeping.

mod common;
mod failpoint_and_race_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use api_configuration::configuration::Config;
use api_discord::discord_client::GuildMember;
use api_identity_and_access::services::discord_membership_cache::{
    MembershipRefreshLease, accept_membership_observation,
};
use api_identity_and_access::services::discord_rest_reconciliation::{
    enroll_accounts, reconcile_one,
};
use api_state::AppState;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use failpoint_and_race_support::{Failpoint, FailpointArming, lock_suite};
use uuid::Uuid;

/// The role the fake Discord reports for the account.
const OBSERVED_ROLE: &str = "failure-injection-role";

/// The account's snapshot: membership status, lease token, lease held (expiry in the future) and
/// claim revision.
type Snapshot = (String, Option<Uuid>, bool, i64);

/// One account due for a refresh, and a fake Discord that answers its member read.
struct DiscordRefresh {
    state: AppState,
    discord_id: String,
    member_reads: Arc<AtomicUsize>,
    fake_discord: tokio::task::JoinHandle<()>,
}

impl Drop for DiscordRefresh {
    fn drop(&mut self) {
        self.fake_discord.abort();
    }
}

impl DiscordRefresh {
    async fn open(case: &str) -> Self {
        let url = common::require_test_database_url()
            .expect("the Discord failure-injection suite requires PostgreSQL");
        let pool = api_database::connect(&url).await.expect("connect");
        api_database::migrate(&pool).await.expect("migrate");
        let discord_id = format!("fi-{case}-{}", Uuid::new_v4().simple());
        common::seed_user(
            &pool,
            &discord_id,
            &common::unique_arma(case),
            &common::unique_arma(&format!("{case}-arma")),
            "enlisted",
        )
        .await;
        let mut config = Config::for_tests(url, "failure-injection-discord");
        config.discord_bot_token = "bot-token".into();
        let mut state = api::composition::application_state(pool, config);

        let member_reads = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&member_reads);
        let body = format!(r#"{{"roles":["{OBSERVED_ROLE}"]}}"#);
        let fake = Router::new().route(
            &format!(
                "/guilds/{}/members/{discord_id}",
                state.cfg.discord_guild_id
            ),
            get(move || {
                let counter = Arc::clone(&counter);
                let body = body.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::OK, body)
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind the fake Discord");
        let base = format!(
            "http://{}",
            listener
                .local_addr()
                .expect("the listener reports its local address")
        );
        let fake_discord = tokio::spawn(async move {
            axum::serve(listener, fake)
                .await
                .expect("the stand-in HTTP server keeps serving");
        });
        Arc::make_mut(&mut state.discord).set_api_base(&base);

        sqlx::query("DELETE FROM discord_membership_snapshots")
            .execute(&state.pool)
            .await
            .expect("the delete from discord_membership_snapshots succeeds");
        enroll_accounts(&state.pool, &state.cfg.discord_guild_id)
            .await
            .expect("the account enrollment succeeds");
        sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id <> $1")
            .bind(&discord_id)
            .execute(&state.pool)
            .await
            .expect("the delete from discord_membership_snapshots succeeds");
        let refresh = Self {
            state,
            discord_id,
            member_reads,
            fake_discord,
        };
        refresh.make_due().await;
        refresh
    }

    fn reads(&self) -> usize {
        self.member_reads.load(Ordering::SeqCst)
    }

    /// The refresh is due again and the shared request budget is free.
    async fn make_due(&self) {
        sqlx::query(
            "UPDATE discord_membership_snapshots SET next_refresh_at = clock_timestamp()
             WHERE discord_id = $1",
        )
        .bind(&self.discord_id)
        .execute(&self.state.pool)
        .await
        .expect("the update of discord_membership_snapshots succeeds");
        self.free_request_budget().await;
    }

    async fn free_request_budget(&self) {
        sqlx::query(
            "UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() WHERE singleton",
        )
        .execute(&self.state.pool)
        .await
        .expect("the update of discord_rest_schedule succeeds");
    }

    /// The held lease expires and the request budget the failed attempt reserved is spent.
    async fn expire_lease(&self) {
        sqlx::query(
            "UPDATE discord_membership_snapshots
             SET lease_expires_at = clock_timestamp() - interval '1 second'
             WHERE discord_id = $1 AND lease_token IS NOT NULL",
        )
        .bind(&self.discord_id)
        .execute(&self.state.pool)
        .await
        .expect("the update of discord_membership_snapshots succeeds");
        self.free_request_budget().await;
    }

    async fn snapshot(&self) -> Snapshot {
        sqlx::query_as(
            "SELECT membership_status, lease_token,
                 COALESCE(lease_expires_at > clock_timestamp(), false), revision
             FROM discord_membership_snapshots WHERE discord_id = $1 AND guild_id = $2",
        )
        .bind(&self.discord_id)
        .bind(&self.state.cfg.discord_guild_id)
        .fetch_one(&self.state.pool)
        .await
        .expect("read the membership snapshot")
    }

    async fn recorded_roles(&self) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT discord_role_id FROM user_discord_roles WHERE discord_id = $1
             ORDER BY discord_role_id",
        )
        .bind(&self.discord_id)
        .fetch_all(&self.state.pool)
        .await
        .expect("the read of user_discord_roles runs")
    }

    /// The refresh fails at `point`; answers the lease the failed attempt left held.
    async fn fail_at(
        &self,
        suite: &failpoint_and_race_support::FailpointSuiteLock,
        point: Failpoint,
    ) -> MembershipRefreshLease {
        let before = self.snapshot().await;
        {
            let guard = suite.fail(point);
            let error = reconcile_one(&self.state)
                .await
                .expect_err("the armed point fails the refresh");
            assert!(
                error.to_string().contains(point.name()),
                "the failure names its point: {error}"
            );
            assert_eq!(guard.arrivals(), 1);
        }
        let (status, token, held, revision) = self.snapshot().await;
        assert_eq!(status, before.0, "no observation was recorded");
        assert!(
            self.recorded_roles().await.is_empty(),
            "no role was recorded"
        );
        assert!(held, "the lease stays held until it expires");
        assert_eq!(revision, before.3 + 1);
        let lease = MembershipRefreshLease {
            discord_id: self.discord_id.clone().into(),
            guild_id: self.state.cfg.discord_guild_id.clone(),
            revision,
            lease_token: token.expect("the failed attempt holds its lease"),
        };

        // Until the lease expires no refresh claims the account, so Discord is not asked again.
        let reads = self.reads();
        self.free_request_budget().await;
        assert!(
            !reconcile_one(&self.state)
                .await
                .expect("the reconcile pass succeeds")
        );
        assert_eq!(self.reads(), reads);
        assert_eq!(self.snapshot().await, (status, token, true, revision));
        lease
    }

    /// The refresh after the lease expired applies the observation.
    async fn refresh_after_expiry(&self) {
        self.expire_lease().await;
        assert!(
            reconcile_one(&self.state)
                .await
                .expect("the reconcile pass succeeds")
        );
        let (status, token, held, _) = self.snapshot().await;
        assert_eq!(status, "member");
        assert_eq!(
            (token, held),
            (None, false),
            "the applied refresh releases its lease"
        );
        assert_eq!(self.recorded_roles().await, [OBSERVED_ROLE]);
    }

    /// A late completion under `stale`, reporting another role, changes nothing.
    async fn assert_stale_lease_applies_nothing(&self, stale: &MembershipRefreshLease) {
        let member = GuildMember {
            nick: String::new(),
            roles: vec!["late-role".to_owned()],
        };
        let applied = accept_membership_observation(
            &self.state.pool,
            stale,
            Some(&member),
            &self.state.cfg.discord_guild_id,
        )
        .await
        .expect("accepting the stale membership observation succeeds");
        assert!(!applied, "the failed attempt's lease is fenced");
        assert_eq!(self.recorded_roles().await, [OBSERVED_ROLE]);
    }
}

#[tokio::test]
async fn failure_injection_discord_role_sync_before_effect_asks_nothing_and_the_retry_applies() {
    let suite = lock_suite().await;
    let refresh = DiscordRefresh::open("before").await;

    let stale = refresh
        .fail_at(&suite, Failpoint::DiscordRoleSyncBeforeEffect)
        .await;
    assert_eq!(refresh.reads(), 0, "Discord was never asked");

    refresh.refresh_after_expiry().await;
    assert_eq!(refresh.reads(), 1, "the retry asks Discord once");
    refresh.assert_stale_lease_applies_nothing(&stale).await;
}

#[tokio::test]
async fn failure_injection_discord_role_sync_after_effect_applies_once_and_reconciliation_is_idempotent()
 {
    let suite = lock_suite().await;
    let refresh = DiscordRefresh::open("after").await;

    let stale = refresh
        .fail_at(&suite, Failpoint::DiscordRoleSyncAfterEffect)
        .await;
    assert_eq!(refresh.reads(), 1, "Discord answered before the failure");

    refresh.refresh_after_expiry().await;
    assert_eq!(
        refresh.reads(),
        2,
        "the read is repeated once, after the lease expired"
    );
    refresh.assert_stale_lease_applies_nothing(&stale).await;

    // A further refresh of the same answer leaves the recorded membership as it is.
    let applied = refresh.snapshot().await;
    refresh.make_due().await;
    assert!(reconcile_one(&refresh.state).await.unwrap());
    assert_eq!(refresh.reads(), 3);
    let again = refresh.snapshot().await;
    assert_eq!(
        (&again.0, again.1, again.2),
        (&applied.0, applied.1, applied.2)
    );
    assert_eq!(again.3, applied.3 + 1, "one more claim, no other change");
    assert_eq!(refresh.recorded_roles().await, [OBSERVED_ROLE]);
}
