//! The `staging-fixtures` host tool's load fixture subcommands, `seed-load-fixture-events` and
//! `clean-load-fixture-events`, run as a process.
//!
//! Every test runs the built binary (`CARGO_BIN_EXE_staging-fixtures`) against this suite's own
//! database through an API env file the test writes, then reads the database back: a dry run and a
//! refusal write nothing; a seeding writes ten member-admitting events of 128 slots with their
//! audit rows; the population's share of accounts registers onto its slots through the API; and
//! the cleaning removes the fixture events with their registrations, sparing every other event,
//! the accounts, the mission and every audit row. The fixture titles and the reserved range are
//! shared state, so the tests run one at a time behind [`FIXTURE_LOCK`], and each starts from a
//! database without fixture events or reserved accounts.

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use chrono::{TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::{Mutex, MutexGuard};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::{database, http_router};
use website_api::operations::services::event_authoring::event_creation::{
    EventCreation, EventCreationRequest, create_event,
};

mod common;

/// The first id of the reserved range: the load population's first account, the fixture author.
const AUTHOR: &str = "9100000000000000000";
/// The reserved range's first id as a number; population account k holds `FIRST_RESERVED + k`.
const FIRST_RESERVED: u64 = 9_100_000_000_000_000_000;
/// The SQL pattern of the reserved range.
const RESERVED_PATTERN: &str = "^91000000000000[0-9]{5}$";
/// An administrator outside the reserved range, who authors an event the cleaning must spare.
const OPERATOR: &str = "742000000000000001";
/// The title prefix of the fixture events.
const PREFIX: &str = "[Load fixture]";
/// A bot token value that must never reach the tool's output.
const BOT_TOKEN: &str = "fixture-events-bot-token-value";

/// The fixture titles and the reserved range are shared, so one test at a time owns them.
static FIXTURE_LOCK: Mutex<()> = Mutex::const_new(());

/// One finished run of the tool.
struct ToolRun {
    code: i32,
    stdout: String,
    stderr: String,
}

/// A test's database, router, API env file and host mission; dropping it removes its workspace.
struct Fixture {
    pool: PgPool,
    state: AppState,
    app: Router,
    database: String,
    workspace: PathBuf,
    env_file: PathBuf,
    mission: Uuid,
    audit_mark: i64,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace);
    }
}

async fn fixture() -> Fixture {
    let lock = FIXTURE_LOCK.lock().await;
    let url = common::require_test_database_url().expect("the suite's database");
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&pool)
        .await
        .expect("database name");
    for statement in [
        "DELETE FROM event_registration_history WHERE registration_id IN (SELECT r.id FROM \
         event_registrations r JOIN event_missions m ON m.id = r.event_mission_id JOIN events e \
         ON e.id = m.event_id WHERE starts_with(e.name_override, '[Load fixture]'))",
        "DELETE FROM event_registrations WHERE event_mission_id IN (SELECT m.id FROM \
         event_missions m JOIN events e ON e.id = m.event_id \
         WHERE starts_with(e.name_override, '[Load fixture]'))",
        "DELETE FROM events WHERE starts_with(name_override, '[Load fixture]') \
         OR created_by ~ '^91000000000000[0-9]{5}$'",
        "DELETE FROM refresh_tokens WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
        "DELETE FROM user_discord_roles WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
        "DELETE FROM users WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .unwrap_or_else(|error| panic!("reset `{statement}`: {error}"));
    }
    common::seed_user(
        &pool,
        OPERATOR,
        "Fixture Operator",
        &common::unique_arma("fixture-events"),
        "admin",
    )
    .await;
    let mission = insert_mission(&pool, "live").await;
    let audit_mark: i64 = sqlx::query_scalar("SELECT COALESCE(max(id), 0) FROM audit_logs")
        .fetch_one(&pool)
        .await
        .expect("audit mark");
    let workspace = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("staging_fixtures_fixture_events")
        .join(Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&workspace).expect("workspace");
    let env_file = workspace.join("api.env");
    fs::write(&env_file, format!("DATABASE_URL={url}\n")).expect("API env file");
    let state = AppState::new(
        pool.clone(),
        Config::for_tests(url, "staging-fixtures-fixture-events"),
    );
    let app = http_router::router(state.clone());
    Fixture {
        pool,
        state,
        app,
        database,
        workspace,
        env_file,
        mission,
        audit_mark,
        _lock: lock,
    }
}

/// A mission of the operator's in `status`, for the fixture events to attach.
async fn insert_mission(pool: &PgPool, status: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, \
         created_at, updated_at) \
         VALUES ('Load fixture host mission', $1, 'everon', 'pve_coop', 128, \
         $2::mission_status, now(), now()) RETURNING id",
    )
    .bind(OPERATOR)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("insert mission")
}

impl Fixture {
    /// Run the tool with `arguments`, then the confirmation and `env_file`.
    async fn run_with(&self, arguments: &[&str], env_file: &Path) -> ToolRun {
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_staging-fixtures"))
            .args(arguments)
            .arg("--confirm-database")
            .arg(&self.database)
            .arg("--api-env-file")
            .arg(env_file)
            .output()
            .await
            .expect("run staging-fixtures");
        let run = ToolRun {
            code: output.status.code().expect("exit code"),
            stdout: String::from_utf8(output.stdout).expect("stdout"),
            stderr: String::from_utf8(output.stderr).expect("stderr"),
        };
        assert!(
            !run.stdout.contains(BOT_TOKEN) && !run.stderr.contains(BOT_TOKEN),
            "the bot token reached the output"
        );
        run
    }

    async fn run(&self, arguments: &[&str]) -> ToolRun {
        self.run_with(arguments, &self.env_file).await
    }

    async fn seed(&self, apply: bool) -> ToolRun {
        let mission = self.mission.to_string();
        let mut arguments = vec!["seed-load-fixture-events", "--mission", mission.as_str()];
        if apply {
            arguments.push("--apply");
        }
        self.run(&arguments).await
    }

    async fn clean(&self, apply: bool) -> ToolRun {
        let mut arguments = vec!["clean-load-fixture-events"];
        if apply {
            arguments.push("--apply");
        }
        self.run(&arguments).await
    }

    /// Create the population account `index` as a verified member and return its bearer.
    async fn member(&self, index: u64) -> (String, String) {
        let account = (FIRST_RESERVED + index).to_string();
        let token = common::access_token(
            &self.state,
            "staging_fixtures_fixture_events",
            &account,
            "enlisted",
            true,
        )
        .await;
        (account, token)
    }

    async fn scalar(&self, query: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(query.to_owned()))
            .fetch_one(&self.pool)
            .await
            .unwrap_or_else(|error| panic!("`{query}`: {error}"))
    }

    async fn fixture_events(&self) -> i64 {
        self.scalar(
            "SELECT count(*) FROM events WHERE starts_with(name_override, '[Load fixture]')",
        )
        .await
    }

    /// The audit rows written since the fixture was built with `action`.
    async fn audits(&self, action: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE id > $1 AND action = $2")
            .bind(self.audit_mark)
            .bind(action)
            .fetch_one(&self.pool)
            .await
            .expect("audit count")
    }

    /// The attachment of fixture event `number` and its slots in `(faction, squad, slot_index)`
    /// order.
    async fn fixture_slots(&self, number: u32) -> (Uuid, Vec<Uuid>) {
        let title = format!("{PREFIX} {number:02}");
        let attachment: Uuid = sqlx::query_scalar(
            "SELECT m.id FROM event_missions m JOIN events e ON e.id = m.event_id \
             WHERE e.name_override = $1 AND m.deleted_at IS NULL",
        )
        .bind(&title)
        .fetch_one(&self.pool)
        .await
        .expect("the fixture event's attachment");
        let slots = sqlx::query_scalar(
            "SELECT id FROM orbat_slots WHERE event_mission_id = $1 \
             ORDER BY faction, squad, slot_index",
        )
        .bind(attachment)
        .fetch_all(&self.pool)
        .await
        .expect("the fixture event's slots");
        (attachment, slots)
    }
}

/// `POST /api/v1/event-missions/:id/register` for `slot` as the bearer of `token`, arriving from
/// population account `account`'s own address, as the load run spreads its clients over
/// addresses: the per-address limiter would otherwise hold a burst of registrations at 20 a
/// second.
async fn register(
    app: &Router,
    account: u64,
    token: &str,
    attachment: Uuid,
    slot: Uuid,
) -> (StatusCode, Value) {
    let [.., high, low] = u16::try_from(account)
        .expect("a population index")
        .to_be_bytes();
    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/event-missions/{attachment}/register"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(r#"{{"slot_id":"{slot}"}}"#)))
        .expect("register request");
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([10, 0, high, low], 40_000))));
    let response = app.clone().oneshot(request).await.expect("register");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn staging_fixtures_seed_fixture_events_refuses_and_writes_nothing() {
    let fixture = fixture().await;
    let refused = fixture.seed(true).await;
    assert_eq!(refused.code, 2, "{}", refused.stderr);
    assert!(
        refused
            .stderr
            .contains("no account of the reserved range exists"),
        "{}",
        refused.stderr
    );

    fixture.member(0).await;
    let token_env = fixture.workspace.join("token.env");
    fs::write(
        &token_env,
        format!(
            "{}DISCORD_BOT_TOKEN={BOT_TOKEN}\n",
            fs::read_to_string(&fixture.env_file).expect("env file")
        ),
    )
    .expect("token env file");
    let mission = fixture.mission.to_string();
    let seeding = [
        "seed-load-fixture-events",
        "--mission",
        mission.as_str(),
        "--apply",
    ];
    let refused = fixture.run_with(&seeding, &token_env).await;
    assert_eq!(refused.code, 2, "{}", refused.stderr);
    assert!(
        refused.stderr.contains("DISCORD_BOT_TOKEN is set"),
        "{}",
        refused.stderr
    );

    let archived = insert_mission(&fixture.pool, "archived").await.to_string();
    let unknown = Uuid::new_v4().to_string();
    for (mission, reason) in [
        (archived.as_str(), "is archived"),
        (unknown.as_str(), "names no mission"),
        ("not-a-mission", "takes a mission id"),
    ] {
        let refused = fixture
            .run(&["seed-load-fixture-events", "--mission", mission, "--apply"])
            .await;
        assert_eq!(refused.code, 2, "{mission}: {}", refused.stderr);
        assert!(
            refused.stderr.contains(reason),
            "{mission}: {}",
            refused.stderr
        );
    }
    let missing = fixture.run(&["seed-load-fixture-events", "--apply"]).await;
    assert_eq!(missing.code, 2, "{}", missing.stderr);

    let dry = fixture.seed(false).await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert_eq!(
        dry.stdout.matches("plan fixture=").count(),
        10,
        "{}",
        dry.stdout
    );
    assert!(
        dry.stdout.contains("plan author=9100000000000000000"),
        "{}",
        dry.stdout
    );
    assert!(
        dry.stdout.contains("dry run: nothing written"),
        "{}",
        dry.stdout
    );
    assert_eq!(fixture.fixture_events().await, 0);
    assert_eq!(fixture.audits("event.created").await, 0);
}

#[tokio::test]
async fn staging_fixtures_seed_and_clean_fixture_events() {
    let fixture = fixture().await;
    fixture.member(0).await;
    let seeded = fixture.seed(true).await;
    assert_eq!(seeded.code, 0, "{}", seeded.stderr);
    assert_eq!(
        seeded.stdout.matches(" event_mission=").count(),
        10,
        "{}",
        seeded.stdout
    );
    assert!(
        seeded.stdout.contains(
            "seeded 10 fixture events with 128 slots each, authored by 9100000000000000000"
        ),
        "{}",
        seeded.stdout
    );

    let rows: Vec<(String, String, String, bool, i64, bool)> = sqlx::query_as(
        "SELECT e.name_override, e.created_by, e.status::text, e.registration_locked, e.max_slots, \
         e.start_time > now() + interval '13 days' \
         AND e.access_policy = '{\"grants\":[{\"conditions\":[{\"kind\":\"tbd_member\"}]}]}'::jsonb \
         AND EXISTS (SELECT 1 FROM event_reservation_quota_pools p WHERE p.event_id = e.id \
             AND p.quota_kind = 'member' AND p.seat_limit IS NULL) \
         FROM events e WHERE starts_with(e.name_override, '[Load fixture]') ORDER BY 1",
    )
    .fetch_all(&fixture.pool)
    .await
    .expect("fixture events");
    assert_eq!(rows.len(), 10);
    for (index, (title, author, status, locked, places, admits)) in rows.iter().enumerate() {
        assert_eq!(*title, format!("{PREFIX} {:02}", index + 1));
        assert_eq!((author.as_str(), status.as_str()), (AUTHOR, "open"));
        assert!(!locked && *places == 128 && *admits, "{title}");
    }
    let shape: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT count(*), count(DISTINCT s.faction), count(DISTINCT (s.faction, s.squad)), \
         count(DISTINCT s.slot_index) \
         FROM events e JOIN event_missions m ON m.event_id = e.id AND m.start_time = e.start_time \
         JOIN orbat_slots s ON s.event_mission_id = m.id \
         WHERE starts_with(e.name_override, '[Load fixture]') AND m.mission_id = $1 \
         GROUP BY e.id",
    )
    .bind(fixture.mission)
    .fetch_all(&fixture.pool)
    .await
    .expect("fixture ORBATs");
    assert_eq!(
        shape,
        vec![(128, 2, 16, 8); 10],
        "2 factions x 8 squads x 8 slots"
    );
    assert_eq!(fixture.audits("event.created").await, 10);
    assert_eq!(fixture.audits("event.mission_attached").await, 10);

    let again = fixture.seed(true).await;
    assert_eq!(again.code, 2, "{}", again.stderr);
    assert!(again.stderr.contains("already exist"), "{}", again.stderr);
    let dry = fixture.clean(false).await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(
        dry.stdout.contains(
            "plan remove 10 fixture events, 0 registrations and 0 registration history rows"
        ),
        "{}",
        dry.stdout
    );
    assert_eq!(fixture.fixture_events().await, 10);

    let cleaned = fixture.clean(true).await;
    assert_eq!(cleaned.code, 0, "{}", cleaned.stderr);
    assert_eq!(fixture.fixture_events().await, 0);
    let attachments =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM event_missions WHERE mission_id = $1")
            .bind(fixture.mission)
            .fetch_one(&fixture.pool)
            .await
            .expect("attachments");
    assert_eq!(attachments, 0);
    let mission_kept = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM missions WHERE id = $1")
        .bind(fixture.mission)
        .fetch_one(&fixture.pool)
        .await
        .expect("mission");
    assert_eq!(mission_kept, 1);
    assert_eq!(fixture.audits("event.load_fixture_removed").await, 10);
    assert_eq!(fixture.audits("event.created").await, 10, "audit rows stay");
    assert_eq!(
        fixture
            .scalar("SELECT count(*) FROM users WHERE discord_id = '9100000000000000000'")
            .await,
        1
    );
}

/// Each event seats its share of the 1,100-account population: account k registers on event
/// k mod 10 and slot k div 10 through the API, and the cleaning removes those registrations with
/// their history while the accounts stay.
#[tokio::test]
async fn staging_fixtures_fixture_events_seat_their_population_share() {
    let fixture = fixture().await;
    fixture.member(0).await;
    let seeded = fixture.seed(true).await;
    assert_eq!(seeded.code, 0, "{}", seeded.stderr);

    let (first_attachment, first_slots) = fixture.fixture_slots(1).await;
    assert_eq!(first_slots.len(), 128);
    for account in (0..1_100).step_by(10) {
        let (discord_id, token) = fixture.member(account).await;
        let slot = first_slots[usize::try_from(account / 10).unwrap()];
        let (status, body) = register(&fixture.app, account, &token, first_attachment, slot).await;
        assert!(
            status.is_success(),
            "account {discord_id} on slot {slot}: {status} {body}"
        );
    }
    let (second_attachment, second_slots) = fixture.fixture_slots(2).await;
    let (_, token) = fixture.member(1).await;
    let (status, body) =
        register(&fixture.app, 1, &token, second_attachment, second_slots[0]).await;
    assert!(
        status.is_success(),
        "account 1 on event 02: {status} {body}"
    );

    let seated = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 \
         AND reservation_state = 'registered' AND slot_id IS NOT NULL",
    )
    .bind(first_attachment)
    .fetch_one(&fixture.pool)
    .await
    .expect("seated registrations");
    assert_eq!(
        seated, 110,
        "event 01 seats its 110 accounts and keeps 18 slots free"
    );

    let cleaned = fixture.clean(true).await;
    assert_eq!(cleaned.code, 0, "{}", cleaned.stderr);
    assert!(
        cleaned
            .stdout
            .contains("removed 10 fixture events, 111 registrations and "),
        "{}",
        cleaned.stdout
    );
    assert_eq!(fixture.fixture_events().await, 0);
    let remaining = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM event_registrations WHERE discord_id ~ $1",
    )
    .bind(RESERVED_PATTERN)
    .fetch_one(&fixture.pool)
    .await
    .expect("registrations");
    assert_eq!(remaining, 0);
    let accounts = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE discord_id ~ $1")
        .bind(RESERVED_PATTERN)
        .fetch_one(&fixture.pool)
        .await
        .expect("accounts");
    assert_eq!(accounts, 111, "the cleaning deletes no account");
}

/// The cleaning deletes only `[Load fixture]` events of reserved authors: an operator's event with
/// the same prefix and a reserved author's other event stay.
#[tokio::test]
async fn staging_fixtures_clean_fixture_events_spares_every_other_event() {
    let fixture = fixture().await;
    fixture.member(0).await;
    let mut transaction = fixture.pool.begin().await.expect("begin");
    let mut spared = Vec::new();
    for (title, author) in [
        ("[Load fixture] operator copy", OPERATOR),
        ("Operation Keep", AUTHOR),
    ] {
        let creation = EventCreation::new(EventCreationRequest {
            start_time: Some(Utc::now() + TimeDelta::days(30)),
            name_override: title.to_owned(),
            ..EventCreationRequest::default()
        })
        .expect("a valid event");
        let event = create_event(&mut transaction, &creation, author)
            .await
            .expect("create event");
        spared.push(event.id);
    }
    transaction.commit().await.expect("commit");

    let seeded = fixture.seed(true).await;
    assert_eq!(seeded.code, 0, "{}", seeded.stderr);
    let cleaned = fixture.clean(true).await;
    assert_eq!(cleaned.code, 0, "{}", cleaned.stderr);
    assert!(
        cleaned.stdout.contains("removed 10 fixture events"),
        "{}",
        cleaned.stdout
    );
    let kept = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM events WHERE id = ANY($1)")
        .bind(&spared)
        .fetch_one(&fixture.pool)
        .await
        .expect("spared events");
    assert_eq!(kept, 2);
    assert_eq!(
        fixture.fixture_events().await,
        1,
        "only the operator's copy keeps the prefix"
    );
    sqlx::query("DELETE FROM events WHERE id = ANY($1)")
        .bind(&spared)
        .execute(&fixture.pool)
        .await
        .expect("remove the spared events");
}
