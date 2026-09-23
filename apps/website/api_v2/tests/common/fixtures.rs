//! Row fixtures a suite owns outright, and the unique `arma_id` mint they need.

use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::PgPool;
use uuid::Uuid;

/// Process-local counter for [`unique_arma`]. Starts at 1 so a mint never looks like a bare
/// prefix.
static ARMA_SEQ: AtomicU64 = AtomicU64::new(1);

/// Mint an `arma_id` that cannot collide with a parallel [`seed_user`] in this process.
///
/// `idx_users_arma_id` is a global non-partial unique index. A fixed string like
/// `events-arma-{discord_id}` is a single slot: two concurrent seeds (same or different
/// discord rows racing through release/upsert) can still trip it. Prefer this over sleep.
///
/// Format: `{prefix}-{seq}-{uuid}` — seq is monotonic in-process; uuid covers cross-binary
/// overlap on a shared gate DB (each test binary gets its own `ARMA_SEQ`).
pub fn unique_arma(prefix: &str) -> String {
    let n = ARMA_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}-{}", Uuid::new_v4())
}

/// Seed one user row this suite owns outright.
///
/// `arma_id` is a required argument and must be **unique across the whole database**:
/// `idx_users_arma_id` (`migrations/0001_initial_schema.sql`) is a plain
/// `CREATE UNIQUE INDEX`, not a partial one, so `''` is a *value* and only one row in the
/// entire integration database may hold it. Passing `''` here is the collision waiting to
/// happen — two suites doing it for different `discord_id`s is a unique violation on the
/// second, and `ON CONFLICT (discord_id)` does not catch it because the conflict is on the
/// other index. NULL would coexist, but a distinct string is cheaper to trace in a failure.
///
/// # Idempotent on `arma_id`
///
/// `ON CONFLICT (discord_id)` only absorbs a discord_id clash. If **another** row already
/// holds `arma_id`, the `DO UPDATE SET arma_id = EXCLUDED.arma_id` branch raises
/// `unique_violation` on `idx_users_arma_id`. The upsert is therefore preceded — inside the
/// same transaction — by a release of any foreign holder of this `arma_id`, so a leftover or
/// racing placeholder cannot poison the seed. Suites that share fixed placeholders across
/// parallel tests should still serialise behind a mutex **or** mint via [`unique_arma`].
///
/// `ON CONFLICT DO UPDATE`, not `DO NOTHING`: a suite that owns its ids wants the fixture it
/// asked for on every run, not whatever the previous run happened to leave behind.
pub async fn seed_user(pool: &PgPool, discord_id: &str, username: &str, arma_id: &str, role: &str) {
    let mut tx = pool.begin().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): begin: {e}")
    });

    // Free the arma slot from any *other* discord_id so the upsert cannot unique-violate.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, updated_at = now() \
         WHERE arma_id = $1 AND discord_id IS DISTINCT FROM $2",
    )
    .bind(arma_id)
    .bind(discord_id)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}): release foreign holder: {e}")
    });

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, $2, $2, '', $3, '', $4::user_role, false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, arma_id = EXCLUDED.arma_id, role = EXCLUDED.role, \
          is_banned = false, updated_at = now()",
    )
    .bind(discord_id)
    .bind(username)
    .bind(arma_id)
    .bind(role)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): {e}"));

    tx.commit().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): commit: {e}")
    });
}

/// Seed an authoritative guild snapshot independently of account creation.
///
/// Guest represents verified nonmembership. Member fixtures receive an actor-specific Discord
/// role mapping so changing one actor's permissions does not change another actor's role mapping.
/// Other guilds, existing Arma identity, and account ban state remain untouched.
pub async fn seed_membership(pool: &PgPool, discord_id: &str, guild_id: &str, role: &str) {
    assert!(!guild_id.is_empty(), "membership fixture requires a guild");
    assert!(
        matches!(
            role,
            "guest" | "enlisted" | "leader" | "mission_maker" | "admin"
        ),
        "unknown membership fixture role: {role}"
    );
    let mut tx = pool.begin().await.expect("membership fixture transaction");
    let account: Option<String> =
        sqlx::query_scalar("SELECT discord_id FROM users WHERE discord_id = $1 FOR UPDATE")
            .bind(discord_id)
            .fetch_optional(&mut *tx)
            .await
            .expect("lock membership fixture account");
    assert!(
        account.is_some(),
        "seed account {discord_id} before membership"
    );

    sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1 AND guild_id = $2")
        .bind(discord_id)
        .bind(guild_id)
        .execute(&mut *tx)
        .await
        .expect("clear membership fixture guild roles");

    if role != "guest" {
        let role_id = format!("test-role:{}:{discord_id}:{guild_id}", discord_id.len());
        sqlx::query(
            "INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority) \
             VALUES ($1, 'Integration membership', $2::user_role, 1000) \
             ON CONFLICT (discord_role_id) DO UPDATE SET mapped_role = EXCLUDED.mapped_role, \
             priority = EXCLUDED.priority",
        )
        .bind(&role_id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .expect("map membership fixture role");
        sqlx::query(
            "INSERT INTO user_discord_roles (discord_id, discord_role_id, guild_id, synced_at) \
             VALUES ($1, $2, $3, now())",
        )
        .bind(discord_id)
        .bind(&role_id)
        .bind(guild_id)
        .execute(&mut *tx)
        .await
        .expect("assign membership fixture role");
    }

    sqlx::query(
        "INSERT INTO discord_membership_snapshots \
         (discord_id, guild_id, membership_status, verified_at, revision, next_refresh_at) \
         VALUES ($1, $2, $3, now(), 1, now() + interval '60 seconds') \
         ON CONFLICT (discord_id, guild_id) DO UPDATE SET \
         membership_status = EXCLUDED.membership_status, verified_at = EXCLUDED.verified_at, \
         revision = discord_membership_snapshots.revision + 1, \
         next_refresh_at = EXCLUDED.next_refresh_at, lease_token = NULL, \
         lease_expires_at = NULL, last_error = NULL",
    )
    .bind(discord_id)
    .bind(guild_id)
    .bind(if role == "guest" {
        "nonmember"
    } else {
        "member"
    })
    .execute(&mut *tx)
    .await
    .expect("verify membership fixture snapshot");
    tx.commit().await.expect("commit membership fixture");
}

/// The participant's active event allocation, created as a member place when none exists.
/// Fixtures inserting an active registration directly pass this as `allocation_id`, as every
/// production reservation writer does.
pub async fn participant_allocation(
    connection: &mut sqlx::PgConnection,
    event_mission: Uuid,
    discord_id: &str,
) -> Uuid {
    sqlx::query_scalar(
        "WITH created AS (
             INSERT INTO event_participant_allocations (event_id, discord_id, quota_kind)
             SELECT event_id, $2, 'member' FROM event_missions WHERE id = $1
             ON CONFLICT (event_id, discord_id) WHERE released_at IS NULL DO NOTHING
             RETURNING id)
         SELECT id FROM created
         UNION ALL SELECT allocation.id FROM event_participant_allocations allocation
         JOIN event_missions mission ON mission.event_id = allocation.event_id
         WHERE mission.id = $1 AND allocation.discord_id = $2 AND allocation.released_at IS NULL",
    )
    .bind(event_mission)
    .bind(discord_id)
    .fetch_one(connection)
    .await
    .unwrap_or_else(|error| panic!("allocate {discord_id} for {event_mission}: {error}"))
}

/// The fixed account dev-login mints for each role other than guest
/// (`identity_and_access::handlers::developer_login::discord_id_for_role`).
pub const DEV_LOGIN_MEMBER_IDENTITIES: [(&str, &str); 4] = [
    ("000000000000000001", "admin"),
    ("000000000000000002", "enlisted"),
    ("000000000000000003", "leader"),
    ("000000000000000004", "mission_maker"),
];

/// Make every non-guest dev-login identity a verified TBD member of `guild`, as the default
/// event access policy requires. The guest identity stays an unverified account.
pub async fn verify_dev_login_members(pool: &PgPool, guild: &str) {
    for (discord_id, role) in DEV_LOGIN_MEMBER_IDENTITIES {
        sqlx::query(
            "INSERT INTO users (discord_id, username, role, created_at, updated_at)
             VALUES ($1, 'Dev Operator', $2::user_role, now(), now())
             ON CONFLICT (discord_id) DO NOTHING",
        )
        .bind(discord_id)
        .bind(role)
        .execute(pool)
        .await
        .unwrap_or_else(|error| panic!("seed dev-login account {discord_id}: {error}"));
        seed_membership(pool, discord_id, guild, role).await;
    }
}

/// Register a server, bind `event` to it and return a `mod_runtime` machine credential secret
/// for it, authored by `author`. The credential is stored hashed exactly as the issue route
/// stores it.
pub async fn event_runtime_credential(pool: &PgPool, event: Uuid, author: &str) -> String {
    let mut transaction = pool
        .begin()
        .await
        .expect("begin runtime credential fixture");
    let server: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('Fixture runtime host', '127.0.0.1'::inet, 2302, true) RETURNING id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("register fixture server");
    sqlx::query("UPDATE events SET server_id = $2 WHERE id = $1")
        .bind(event)
        .bind(server)
        .execute(&mut *transaction)
        .await
        .expect("bind event to fixture server");
    let credential = Uuid::new_v4();
    let secret = format!(
        "tbdm_{}_{}",
        credential.simple(),
        website_api::core::authentication_primitives::random_token(32)
    );
    sqlx::query(
        "INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'mod_runtime', $3, 'Fixture runtime', $4)",
    )
    .bind(credential)
    .bind(server)
    .bind(website_api::core::authentication_primitives::hash_token(&secret))
    .bind(author)
    .execute(&mut *transaction)
    .await
    .expect("store fixture credential");
    transaction
        .commit()
        .await
        .expect("commit runtime credential fixture");
    secret
}

/// An editor payload the compiler turns into a schema-valid mod document: one faction, one squad
/// and one placed slot.
pub const COMPILABLE_EDITOR_PAYLOAD: &str = r#"{"editor":{
    "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
    "squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A 1-1","slotIds":["s1"]}],
    "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL",
        "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
    "editorLayers":[]}}"#;
