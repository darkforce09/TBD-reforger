//! Generated account and session states for the protected-action property.
//!
//! **Role:** Declares the generated state space (guild role mapping, membership snapshot,
//! grace override, session provenance and fate, account fate, serving environment) and
//! writes one drawn state into the database as a fresh account holding one access credential.
//!
//! **Position:** Test support for `tests/session_authority_properties.rs`. Sessions come from
//! the production `issue_session` and `issue_development_session`, rotation and logout from
//! `rotate_session` and `logout_session`; membership rows, session expiry, bans and deletions
//! are direct writes to the tables the account authority query reads.
//! [`super::authority_oracle`] judges each state without reading any of it back.
//!
//! **Signals & state:** none beyond the rows each call inserts for its own fresh account.
//!
//! **Invariants:** every generated snapshot age and override expiry keeps
//! [`BOUNDARY_MARGIN_SECONDS`] from the freshness and grace boundaries, so request latency
//! cannot move a state across one; bans and deletions land after issuance, the only order in
//! which a banned or deleted account holds a session.

use super::PropertyWorld;
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use proptest::prelude::*;
use sqlx::PgPool;
use website_api::core::{application_state::AppState, authentication_primitives::Claims};
use website_api::identity_and_access::models::user_account::UserRole;
use website_api::identity_and_access::services::{
    session_issuance::{issue_development_session, issue_session},
    session_rotation::{logout_session, rotate_session},
};

/// A membership snapshot younger than this many seconds is fresh.
pub const FRESHNESS_SECONDS: i64 = 60;
/// A verified membership grants its mapped rank while younger than this many seconds.
pub const GRACE_SECONDS: i64 = 48 * 60 * 60;
/// Seconds every generated age and expiry keeps from the freshness and grace boundaries.
pub const BOUNDARY_MARGIN_SECONDS: i64 = 15;

/// A site rank, declared from least to most privileged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Guest,
    Enlisted,
    Leader,
    MissionMaker,
    Admin,
}

impl Rank {
    /// The Postgres enum and JSON wire spelling.
    pub fn wire(self) -> &'static str {
        match self {
            Rank::Guest => "guest",
            Rank::Enlisted => "enlisted",
            Rank::Leader => "leader",
            Rank::MissionMaker => "mission_maker",
            Rank::Admin => "admin",
        }
    }

    fn user_role(self) -> UserRole {
        match self {
            Rank::Guest => UserRole::Guest,
            Rank::Enlisted => UserRole::Enlisted,
            Rank::Leader => UserRole::Leader,
            Rank::MissionMaker => UserRole::MissionMaker,
            Rank::Admin => UserRole::Admin,
        }
    }
}

/// The site rank the account's single guild role maps to.
#[derive(Debug, Clone, Copy)]
pub enum GuildRoleMapping {
    /// The account holds no mapped guild role.
    Unmapped,
    /// The account holds one guild role mapped to this rank.
    Mapped(Rank),
}

/// The membership status the snapshot confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipStatus {
    Member,
    ConfirmedNonmember,
}

/// When the snapshot was verified, relative to the database clock.
#[derive(Debug, Clone, Copy)]
pub enum VerificationTime {
    SecondsAgo(i64),
    SecondsAhead(i64),
}

/// The audited grace override attached to the snapshot.
#[derive(Debug, Clone, Copy)]
pub enum GraceOverride {
    Absent,
    ExpiresInSeconds(i64),
    ExpiredSecondsAgo(i64),
}

/// The account's guild membership snapshot.
#[derive(Debug, Clone, Copy)]
pub struct MembershipSnapshot {
    pub status: MembershipStatus,
    pub verified: VerificationTime,
    /// The last synchronization attempt recorded an error.
    pub sync_error: bool,
    pub grace_override: GraceOverride,
}

/// How the session was issued.
#[derive(Debug, Clone, Copy)]
pub enum SessionProvenance {
    /// The ordinary sign-in path (`issue_session`).
    Ordinary,
    /// The development login path, holding this rank.
    Development(Rank),
}

/// What happens to the session after issuance.
#[derive(Debug, Clone, Copy)]
pub enum SessionFate {
    Untouched,
    /// Its refresh token is rotated once; the original access credential is presented.
    RefreshRotated,
    LoggedOut,
    SessionExpired,
    /// The session stays live; the presented credential is a copy of its claims whose own
    /// expiry has passed.
    AccessCredentialExpired,
}

/// What happens to the account after issuance.
#[derive(Debug, Clone, Copy)]
pub enum AccountFate {
    Active,
    Banned,
    Deleted,
}

/// The configuration that serves the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingEnvironment {
    Development,
    Production,
}

/// One generated account and session state.
#[derive(Debug, Clone)]
pub struct AuthorityCase {
    pub guild_role: GuildRoleMapping,
    /// `None` when the account has no snapshot for the configured guild.
    pub snapshot: Option<MembershipSnapshot>,
    pub provenance: SessionProvenance,
    pub session_fate: SessionFate,
    pub account_fate: AccountFate,
    pub environment: ServingEnvironment,
}

fn rank() -> impl Strategy<Value = Rank> {
    prop_oneof![
        Just(Rank::Guest),
        Just(Rank::Enlisted),
        Just(Rank::Leader),
        Just(Rank::MissionMaker),
        Just(Rank::Admin),
    ]
}

fn verification_time() -> impl Strategy<Value = VerificationTime> {
    let margin = BOUNDARY_MARGIN_SECONDS;
    prop_oneof![
        3 => (0..FRESHNESS_SECONDS - margin).prop_map(VerificationTime::SecondsAgo),
        3 => (FRESHNESS_SECONDS + margin..GRACE_SECONDS - margin)
            .prop_map(VerificationTime::SecondsAgo),
        3 => (GRACE_SECONDS + margin..2 * GRACE_SECONDS).prop_map(VerificationTime::SecondsAgo),
        1 => (margin..GRACE_SECONDS).prop_map(VerificationTime::SecondsAhead),
    ]
}

fn grace_override() -> impl Strategy<Value = GraceOverride> {
    let margin = BOUNDARY_MARGIN_SECONDS;
    prop_oneof![
        Just(GraceOverride::Absent),
        (margin..GRACE_SECONDS - margin).prop_map(GraceOverride::ExpiresInSeconds),
        (margin..GRACE_SECONDS).prop_map(GraceOverride::ExpiredSecondsAgo),
    ]
}

fn membership_snapshot() -> impl Strategy<Value = Option<MembershipSnapshot>> {
    let status = prop_oneof![
        4 => Just(MembershipStatus::Member),
        1 => Just(MembershipStatus::ConfirmedNonmember),
    ];
    let snapshot = (status, verification_time(), any::<bool>(), grace_override()).prop_map(
        |(status, verified, sync_error, grace_override)| MembershipSnapshot {
            status,
            verified,
            sync_error,
            grace_override,
        },
    );
    prop_oneof![1 => Just(None), 6 => snapshot.prop_map(Some)]
}

/// Every generated account and session state.
pub fn authority_case() -> impl Strategy<Value = AuthorityCase> {
    let guild_role = prop_oneof![
        1 => Just(GuildRoleMapping::Unmapped),
        5 => rank().prop_map(GuildRoleMapping::Mapped),
    ];
    let provenance = prop_oneof![
        3 => Just(SessionProvenance::Ordinary),
        1 => rank().prop_map(SessionProvenance::Development),
    ];
    let session_fate = prop_oneof![
        4 => Just(SessionFate::Untouched),
        2 => Just(SessionFate::RefreshRotated),
        1 => Just(SessionFate::LoggedOut),
        1 => Just(SessionFate::SessionExpired),
        1 => Just(SessionFate::AccessCredentialExpired),
    ];
    let account_fate = prop_oneof![
        6 => Just(AccountFate::Active),
        1 => Just(AccountFate::Banned),
        1 => Just(AccountFate::Deleted),
    ];
    let environment = prop_oneof![
        Just(ServingEnvironment::Development),
        Just(ServingEnvironment::Production),
    ];
    (
        guild_role,
        membership_snapshot(),
        provenance,
        session_fate,
        account_fate,
        environment,
    )
        .prop_map(
            |(guild_role, snapshot, provenance, session_fate, account_fate, environment)| {
                AuthorityCase {
                    guild_role,
                    snapshot,
                    provenance,
                    session_fate,
                    account_fate,
                    environment,
                }
            },
        )
}

/// A realised state: the fresh account and the access credential its request presents.
pub struct RealisedSession {
    pub discord_id: String,
    pub access_credential: String,
}

/// Write `case` into the database as a fresh account and return the credential to present.
pub async fn realise(world: &PropertyWorld, case: &AuthorityCase) -> RealisedSession {
    let state = &world.development;
    let pool = &state.pool;
    let guild = state.cfg.discord_guild_id.as_str();
    let discord_id = super::seed_account(pool, "session-authority").await;
    if let GuildRoleMapping::Mapped(rank) = case.guild_role {
        let role_id = format!("session-authority-role:{discord_id}");
        sqlx::query(
            "INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority)
             VALUES ($1, 'Session authority property', $2::user_role, 1000)",
        )
        .bind(&role_id)
        .bind(rank.wire())
        .execute(pool)
        .await
        .expect("map the generated guild role");
        sqlx::query(
            "INSERT INTO user_discord_roles (discord_id, discord_role_id, guild_id)
             VALUES ($1, $2, $3)",
        )
        .bind(&discord_id)
        .bind(&role_id)
        .bind(guild)
        .execute(pool)
        .await
        .expect("grant the generated guild role");
    }
    if let Some(snapshot) = case.snapshot {
        write_snapshot(pool, &discord_id, guild, snapshot).await;
    }
    let (access, _, refresh) = match case.provenance {
        SessionProvenance::Ordinary => issue_session(state, &discord_id).await,
        SessionProvenance::Development(rank) => {
            issue_development_session(state, &discord_id, rank.user_role()).await
        }
    }
    .expect("a fresh, unbanned account receives a session");
    let access_credential = apply_session_fate(state, case.session_fate, access, &refresh).await;
    apply_account_fate(pool, &discord_id, case.account_fate).await;
    RealisedSession {
        discord_id,
        access_credential,
    }
}

async fn write_snapshot(
    pool: &PgPool,
    discord_id: &str,
    guild: &str,
    snapshot: MembershipSnapshot,
) {
    let status = match snapshot.status {
        MembershipStatus::Member => "member",
        MembershipStatus::ConfirmedNonmember => "nonmember",
    };
    let verified_offset = match snapshot.verified {
        VerificationTime::SecondsAgo(seconds) => -seconds,
        VerificationTime::SecondsAhead(seconds) => seconds,
    };
    sqlx::query(
        "INSERT INTO discord_membership_snapshots
         (discord_id, guild_id, membership_status, verified_at, revision, next_refresh_at,
          last_error)
         VALUES ($1, $2, $3, now() + make_interval(secs => $4), 1, now() + interval '1 day', $5)",
    )
    .bind(discord_id)
    .bind(guild)
    .bind(status)
    .bind(verified_offset as f64)
    .bind(
        snapshot
            .sync_error
            .then_some("generated synchronization failure"),
    )
    .execute(pool)
    .await
    .expect("write the generated membership snapshot");
    let (created_offset, expires_offset) = match snapshot.grace_override {
        GraceOverride::Absent => return,
        GraceOverride::ExpiresInSeconds(seconds) => (0, seconds),
        GraceOverride::ExpiredSecondsAgo(seconds) => (-(seconds + 3600), -seconds),
    };
    sqlx::query(
        "INSERT INTO discord_membership_grace_overrides
         (discord_id, guild_id, authorized_by, reason, created_at, expires_at)
         VALUES ($1, $2, $1, 'Session authority property',
                 now() + make_interval(secs => $3), now() + make_interval(secs => $4))",
    )
    .bind(discord_id)
    .bind(guild)
    .bind(created_offset as f64)
    .bind(expires_offset as f64)
    .execute(pool)
    .await
    .expect("write the generated grace override");
}

async fn apply_session_fate(
    state: &AppState,
    fate: SessionFate,
    access: String,
    refresh: &str,
) -> String {
    match fate {
        SessionFate::Untouched => access,
        SessionFate::RefreshRotated => {
            rotate_session(state, refresh)
                .await
                .expect("a live session rotates its refresh token");
            access
        }
        SessionFate::LoggedOut => {
            logout_session(state, refresh)
                .await
                .expect("a live session logs out");
            access
        }
        SessionFate::SessionExpired => {
            let session = state.jwt.parse(&access).expect("issued credential").sid;
            sqlx::query(
                "UPDATE authentication_sessions SET expires_at = now() - interval '1 second'
                 WHERE id = $1",
            )
            .bind(session)
            .execute(&state.pool)
            .await
            .expect("expire the generated session");
            access
        }
        SessionFate::AccessCredentialExpired => {
            let claims = state.jwt.parse(&access).expect("issued credential");
            let now = Utc::now().timestamp();
            let expired = Claims {
                iat: now - 960,
                exp: now - 60,
                ..claims
            };
            jsonwebtoken::encode(
                &Header::new(Algorithm::HS256),
                &expired,
                &EncodingKey::from_secret(super::JWT_SECRET.as_bytes()),
            )
            .expect("sign the expired access credential")
        }
    }
}

async fn apply_account_fate(pool: &PgPool, discord_id: &str, fate: AccountFate) {
    let statement = match fate {
        AccountFate::Active => return,
        AccountFate::Banned => {
            "UPDATE users SET is_banned = true, ban_reason = 'Session authority property'
             WHERE discord_id = $1"
        }
        AccountFate::Deleted => "UPDATE users SET deleted_at = now() WHERE discord_id = $1",
    };
    sqlx::query(statement)
        .bind(discord_id)
        .execute(pool)
        .await
        .expect("apply the generated account fate");
}
