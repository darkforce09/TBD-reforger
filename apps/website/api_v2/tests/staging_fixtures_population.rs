//! The `staging-fixtures` host tool's load population subcommands, `seed-load-population` and
//! `clean-load-population`, run as a process.
//!
//! Every test runs the built binary (`CARGO_BIN_EXE_staging-fixtures`) against this suite's own
//! database through an API env file the test writes, then reads the database, the account file and
//! the real router back: seeding refuses while the bot token is set, while any reserved account
//! exists and for a role above member authority, writing nothing; an applied seeding makes
//! verified members whose refresh token signs in and registers for an event slot; a failed seeding
//! deletes the accounts it created; a cleanup leaves no row naming a reserved account outside the
//! audit tables; no token and no connection string reaches stdout or stderr. The reserved range is
//! shared, so the tests run one at a time behind [`POPULATION_LOCK`], each starting from a
//! database without reserved accounts.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::http::StatusCode;
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::http_router;

mod common;
mod events_support;

/// The main guild: the router's test configuration judges event access in this guild.
const GUILD: &str = "test-tbd-guild";
/// The Discord role the population holds, mapped to the member role.
const PLAYER_ROLE: (&str, &str) = ("741099000000000001", "Staging Load Player");
/// A Discord role mapped to the administrator role, which a population must never hold.
const COMMAND_ROLE: (&str, &str) = ("741099000000000002", "Staging Load Command");
/// The first reserved id, where a population starts by default.
const FIRST_RESERVED: &str = "9100000000000000000";
/// The SQL pattern of every reserved id, anchored nowhere so it also finds ids inside JSON.
const RESERVED_ANYWHERE: &str = "91000000000000[0-9]{5}";
/// The tables whose rows outlive a cleanup on purpose: the append-only audit log and its
/// publication bookkeeping.
const AUDIT_TABLES: [&str; 4] = [
    "audit_logs",
    "audit_publications",
    "audit_publication_pending",
    "audit_publication_state",
];

/// The reserved range is shared by every test of this suite.
static POPULATION_LOCK: Mutex<()> = Mutex::const_new(());

/// One finished run of the tool.
struct ToolRun {
    code: i32,
    stdout: String,
    stderr: String,
}

/// A test's database, API env file and workspace; dropping it removes the workspace.
struct Fixture {
    pool: PgPool,
    database: String,
    database_url: String,
    workspace: PathBuf,
    env_file: PathBuf,
    account_file: PathBuf,
    _population: MutexGuard<'static, ()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace);
    }
}

async fn fixture() -> Fixture {
    let population = POPULATION_LOCK.lock().await;
    let database_url = common::require_test_database_url().expect("the suite's database");
    let pool = PgPool::connect(&database_url).await.expect("connect");
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&pool)
        .await
        .expect("database name");
    for statement in [
        "DROP TRIGGER IF EXISTS staging_fixtures_population_refuse_account ON users",
        "DELETE FROM event_registration_history WHERE registration_id IN (SELECT id FROM \
         event_registrations WHERE discord_id ~ '^91000000000000[0-9]{5}$')",
        "DELETE FROM event_registrations WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
        "DELETE FROM fire_missions WHERE created_by ~ '^91000000000000[0-9]{5}$'",
        "DELETE FROM users WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .unwrap_or_else(|error| panic!("{statement}: {error}"));
    }
    for ((role_id, name), mapped) in [(PLAYER_ROLE, "enlisted"), (COMMAND_ROLE, "admin")] {
        sqlx::query(
            "INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority)
             VALUES ($1, $2, $3::user_role, 10)
             ON CONFLICT (discord_role_id) DO UPDATE SET name = EXCLUDED.name,
                 mapped_role = EXCLUDED.mapped_role",
        )
        .bind(role_id)
        .bind(name)
        .bind(mapped)
        .execute(&pool)
        .await
        .expect("discord role");
    }
    let workspace = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("staging_fixtures_population")
        .join(Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&workspace).expect("workspace");
    let env_file = workspace.join("api.env");
    fs::write(
        &env_file,
        format!("DATABASE_URL={database_url}\nDISCORD_GUILD_ID={GUILD}\nDISCORD_BOT_TOKEN=\n"),
    )
    .expect("API env file");
    Fixture {
        pool,
        database,
        database_url,
        account_file: workspace.join("load").join("accounts.json"),
        env_file,
        workspace,
        _population: population,
    }
}

impl Fixture {
    /// Run the tool with `arguments`, then the confirmation and the env file.
    async fn run(&self, arguments: &[&str]) -> ToolRun {
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_staging-fixtures"))
            .args(arguments)
            .arg("--confirm-database")
            .arg(&self.database)
            .arg("--api-env-file")
            .arg(&self.env_file)
            .output()
            .await
            .expect("run staging-fixtures");
        let run = ToolRun {
            code: output.status.code().expect("exit code"),
            stdout: String::from_utf8(output.stdout).expect("stdout"),
            stderr: String::from_utf8(output.stderr).expect("stderr"),
        };
        self.assert_no_secret_printed(&run);
        run
    }

    /// `seed-load-population` of `accounts` accounts with the player role, plus `extra`.
    async fn seed(&self, accounts: &str, extra: &[&str]) -> ToolRun {
        let account_file = self.account_file.to_str().expect("UTF-8 path");
        let mut arguments = vec![
            "seed-load-population",
            "--accounts",
            accounts,
            "--role",
            PLAYER_ROLE.1,
            "--account-file",
            account_file,
        ];
        arguments.extend_from_slice(extra);
        self.run(&arguments).await
    }

    /// Neither a refresh token of the account file, nor the connection string, nor a bot token
    /// of the env file is printed.
    fn assert_no_secret_printed(&self, run: &ToolRun) {
        let printed = format!("{}\n{}", run.stdout, run.stderr);
        assert!(
            !printed.contains(&self.database_url),
            "DATABASE_URL printed"
        );
        let env = fs::read_to_string(&self.env_file).expect("env file");
        if let Some(token) = env
            .lines()
            .find_map(|line| line.strip_prefix("DISCORD_BOT_TOKEN="))
            .filter(|token| !token.is_empty())
        {
            assert!(!printed.contains(token), "the bot token was printed");
        }
        if let Ok(text) = fs::read_to_string(&self.account_file) {
            for account in account_entries(&text) {
                let token = account["refresh_token"].as_str().expect("token");
                assert!(!printed.contains(token), "a refresh token was printed");
            }
        }
    }

    async fn reserved_accounts(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM users WHERE discord_id ~ '^91000000000000[0-9]{5}$'",
        )
        .fetch_one(&self.pool)
        .await
        .expect("reserved census")
    }

    async fn router(&self) -> Router {
        common::fixtures::verify_dev_login_members(&self.pool, GUILD).await;
        http_router::router(AppState::new(
            self.pool.clone(),
            Config::for_tests(
                self.database_url.clone(),
                "staging-fixtures-population-secret",
            ),
        ))
    }
}

fn account_entries(text: &str) -> Vec<Value> {
    let document: Value = serde_json::from_str(text).expect("the account file is JSON");
    document["accounts"]
        .as_array()
        .expect("an account list")
        .clone()
}

fn assert_refused(run: &ToolRun, reason: &str) {
    assert_eq!(run.code, 2, "refused with exit 2: {}", run.stderr);
    assert!(
        run.stderr.contains(reason),
        "names `{reason}`: {}",
        run.stderr
    );
}

/// An event with one attached mission and a one-slot ORBAT, created by the development
/// administrator; returns the event mission and its slot.
async fn event_with_one_slot(app: &Router) -> (String, String) {
    let admin = common::dev_login_token(app, "staging_fixtures_population", "admin").await;
    let mission_body =
        r#"{"title":"Load Op","terrain":"everon","game_mode":"pve_coop","max_players":8}"#;
    let (status, mission) =
        events_support::call(app, "POST", "/api/v1/missions", &admin, Some(mission_body)).await;
    assert_eq!(status, StatusCode::CREATED, "mission: {mission}");
    let event_body = r#"{"start_time":"2027-01-01T00:00:00Z"}"#;
    let (status, event) =
        events_support::call(app, "POST", "/api/v1/events", &admin, Some(event_body)).await;
    assert_eq!(status, StatusCode::CREATED, "event: {event}");
    let attach = format!(
        r#"{{"mission_id":"{}","start_time":"2027-01-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Alpha","slots":[{{"role":"SL"}}]}}]}}"#,
        mission["id"].as_str().expect("mission id")
    );
    let attach_uri = format!(
        "/api/v1/events/{}/missions",
        event["id"].as_str().expect("event id")
    );
    let (status, attached) =
        events_support::call(app, "POST", &attach_uri, &admin, Some(&attach)).await;
    assert_eq!(status, StatusCode::CREATED, "attach: {attached}");
    let event_mission = attached["id"]
        .as_str()
        .expect("event mission id")
        .to_owned();
    let orbat_uri = format!("/api/v1/event-missions/{event_mission}/orbat");
    let (status, orbat) = events_support::call(app, "GET", &orbat_uri, &admin, None).await;
    assert_eq!(status, StatusCode::OK, "orbat: {orbat}");
    let slot = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .expect("slot id")
        .to_owned();
    (event_mission, slot)
}

/// Sign the population's first account in with its refresh token and register it for the slot.
async fn first_member_registers(fixture: &Fixture, app: &Router) -> Value {
    let accounts = account_entries(&fs::read_to_string(&fixture.account_file).expect("file"));
    let refresh = accounts[0]["refresh_token"].as_str().expect("token");
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(format!(
            r#"{{"refresh_token":"{refresh}"}}"#
        )))
        .expect("refresh request");
    let response = tower::ServiceExt::oneshot(app.clone(), request)
        .await
        .expect("refresh");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the seeded refresh token signs in"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("refresh body");
    let pair: Value = serde_json::from_slice(&bytes).expect("token pair");
    let access = pair["access_token"].as_str().expect("access token");
    let (event_mission, slot) = event_with_one_slot(app).await;
    let (status, registration) = events_support::call(
        app,
        "POST",
        &format!("/api/v1/event-missions/{event_mission}/register"),
        access,
        Some(&format!(r#"{{"slot_id":"{slot}"}}"#)),
    )
    .await;
    assert!(
        status.is_success(),
        "the synthetic member registers: {status} {registration}"
    );
    assert_eq!(registration["state"], "registered", "{registration}");
    registration
}

#[tokio::test]
async fn staging_fixtures_population_seeding_refuses_while_the_bot_token_is_set() {
    let fixture = fixture().await;
    fs::write(
        &fixture.env_file,
        format!(
            "DATABASE_URL={}\nDISCORD_GUILD_ID={GUILD}\nDISCORD_BOT_TOKEN=staging-fixtures.bot-token.value\n",
            fixture.database_url
        ),
    )
    .expect("API env file with a token");
    let run = fixture.seed("3", &["--apply"]).await;
    assert_refused(&run, "DISCORD_BOT_TOKEN is set");
    assert_eq!(fixture.reserved_accounts().await, 0);
    assert!(!fixture.account_file.exists(), "no account file");
}

#[tokio::test]
async fn staging_fixtures_population_seeding_refuses_while_a_reserved_account_exists() {
    let fixture = fixture().await;
    common::seed_user(
        &fixture.pool,
        "9100000000000000500",
        "Leftover",
        &common::unique_arma("population"),
        "enlisted",
    )
    .await;
    let run = fixture.seed("3", &["--apply"]).await;
    assert_refused(&run, "1 accounts of the reserved range exist");
    assert_eq!(fixture.reserved_accounts().await, 1);
    assert!(!fixture.account_file.exists(), "no account file");
}

#[tokio::test]
async fn staging_fixtures_population_seeding_refuses_roles_above_member_authority() {
    let fixture = fixture().await;
    let account_file = fixture.account_file.to_str().expect("UTF-8").to_owned();
    for (role, reason) in [
        (COMMAND_ROLE.1, "maps to the site role admin"),
        ("No Such Role", "0 Discord roles are named"),
    ] {
        let run = fixture
            .run(&[
                "seed-load-population",
                "--accounts",
                "3",
                "--role",
                role,
                "--account-file",
                &account_file,
                "--apply",
            ])
            .await;
        assert_refused(&run, reason);
    }
    let outside = fixture
        .seed("3", &["--id-base", "9100000000000099998"])
        .await;
    assert_refused(&outside, "leave the reserved range");
    assert_eq!(fixture.reserved_accounts().await, 0);
}

#[tokio::test]
async fn staging_fixtures_population_dry_run_writes_nothing() {
    let fixture = fixture().await;
    let run = fixture.seed("3", &[]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout
            .contains("seed plan: accounts=3 first=9100000000000000000"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains("dry run: nothing written"),
        "{}",
        run.stdout
    );
    assert_eq!(fixture.reserved_accounts().await, 0);
    assert!(!fixture.account_file.parent().expect("directory").exists());
}

#[tokio::test]
async fn staging_fixtures_population_seeded_member_signs_in_and_registers_for_a_slot() {
    let fixture = fixture().await;
    let run = fixture.seed("3", &["--apply"]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("seeded: accounts=3"), "{}", run.stdout);
    let file_mode = fs::metadata(&fixture.account_file)
        .expect("file")
        .permissions()
        .mode();
    assert_eq!(file_mode & 0o777, 0o600, "the account file is mode 600");
    let directory = fixture.account_file.parent().expect("directory");
    assert_eq!(
        fs::metadata(directory).expect("dir").permissions().mode() & 0o777,
        0o700
    );
    let accounts = account_entries(&fs::read_to_string(&fixture.account_file).expect("file"));
    let ids: Vec<&str> = accounts
        .iter()
        .map(|a| a["discord_id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        [FIRST_RESERVED, "9100000000000000001", "9100000000000000002"]
    );
    let verified: Vec<(String, String, bool, String)> = sqlx::query_as(
        "SELECT u.discord_id, u.role::text, s.verified_at > now() - interval '5 minutes',
                (SELECT string_agg(r.discord_role_id, ',') FROM user_discord_roles r
                 WHERE r.discord_id = u.discord_id AND r.guild_id = $1)
         FROM users u JOIN discord_membership_snapshots s
           ON s.discord_id = u.discord_id AND s.guild_id = $1 AND s.membership_status = 'member'
         WHERE u.discord_id ~ '^91000000000000[0-9]{5}$' ORDER BY u.discord_id",
    )
    .bind(GUILD)
    .fetch_all(&fixture.pool)
    .await
    .expect("verified members");
    assert_eq!(verified.len(), 3, "every account is a verified member");
    for (_, role, fresh, roles) in &verified {
        assert_eq!(
            (role.as_str(), *fresh, roles.as_str()),
            ("enlisted", true, PLAYER_ROLE.0)
        );
    }
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action = 'staging.load_population_seeded'",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("seeding audit");
    assert!(audits >= 1, "the seeding is audited");
    let app = fixture.router().await;
    first_member_registers(&fixture, &app).await;
}

#[tokio::test]
async fn staging_fixtures_population_failed_seeding_deletes_the_accounts_it_created() {
    let fixture = fixture().await;
    sqlx::query(
        "CREATE OR REPLACE FUNCTION staging_fixtures_population_refuse_account() RETURNS trigger
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'the test refuses this account'; END $$",
    )
    .execute(&fixture.pool)
    .await
    .expect("refusing function");
    sqlx::query(
        "CREATE TRIGGER staging_fixtures_population_refuse_account BEFORE INSERT ON users
         FOR EACH ROW WHEN (NEW.discord_id = '9100000000000000002')
         EXECUTE FUNCTION staging_fixtures_population_refuse_account()",
    )
    .execute(&fixture.pool)
    .await
    .expect("refusing trigger");
    let run = fixture.seed("3", &["--apply"]).await;
    sqlx::query("DROP TRIGGER staging_fixtures_population_refuse_account ON users")
        .execute(&fixture.pool)
        .await
        .expect("drop the trigger");
    assert_eq!(run.code, 1, "a failed seeding exits 1: {}", run.stderr);
    assert!(
        run.stderr
            .contains("(after 2 of 3 accounts); the 2 accounts it created were deleted"),
        "{}",
        run.stderr
    );
    assert_eq!(
        fixture.reserved_accounts().await,
        0,
        "the created accounts are gone"
    );
    assert!(!fixture.account_file.exists(), "no account file");
}

#[tokio::test]
async fn staging_fixtures_population_cleanup_leaves_only_audit_rows() {
    let fixture = fixture().await;
    assert_eq!(fixture.seed("3", &["--apply"]).await.code, 0);
    let app = fixture.router().await;
    let registration = first_member_registers(&fixture, &app).await;
    let member_audits = audit_rows_naming(&fixture.pool, FIRST_RESERVED).await;
    assert!(member_audits > 0, "the member's registration is audited");

    let dry = fixture.run(&["clean-load-population"]).await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(
        dry.stdout.contains("clean plan: accounts=3"),
        "{}",
        dry.stdout
    );
    assert!(dry.stdout.contains("registrations=1"), "{}", dry.stdout);
    assert_eq!(
        fixture.reserved_accounts().await,
        3,
        "a dry run deletes nothing"
    );

    let run = fixture.run(&["clean-load-population", "--apply"]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("cleaned: accounts=3"), "{}", run.stdout);
    let remaining = rows_naming_reserved_ids_outside_audit(&fixture.pool).await;
    assert!(
        remaining.is_empty(),
        "rows still name reserved accounts: {remaining:?}"
    );
    assert!(
        audit_rows_naming(&fixture.pool, FIRST_RESERVED).await >= member_audits,
        "the audit rows stay"
    );
    let slot_holder: Option<String> =
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1::uuid")
            .bind(registration["slot_id"].as_str().expect("slot id"))
            .fetch_one(&fixture.pool)
            .await
            .expect("slot");
    assert_eq!(slot_holder, None, "the slot is free again");
    let again = fixture.run(&["clean-load-population", "--apply"]).await;
    assert!(
        again.stdout.contains("nothing to clean"),
        "{}",
        again.stdout
    );
}

async fn audit_rows_naming(pool: &PgPool, discord_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs
         WHERE actor_id = $1 OR target_id = $1 OR metadata::text LIKE '%' || $1 || '%'",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await
    .expect("audit rows")
}

/// Every `table.column` outside the audit tables holding a value that names a reserved id, with
/// its row count.
async fn rows_naming_reserved_ids_outside_audit(pool: &PgPool) -> Vec<(String, i64)> {
    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.table_name::text, c.column_name::text FROM information_schema.columns c
         JOIN information_schema.tables t USING (table_schema, table_name)
         WHERE c.table_schema = 'public' AND t.table_type = 'BASE TABLE'
           AND c.data_type IN ('text', 'character varying', 'jsonb', 'json', 'ARRAY')
           AND NOT (c.table_name::text = ANY($1)) AND c.table_name::text <> '_sqlx_migrations'
         ORDER BY 1, 2",
    )
    .bind(AUDIT_TABLES.as_slice())
    .fetch_all(pool)
    .await
    .expect("identity columns");
    assert!(
        columns.len() > 50,
        "the scan covers the schema: {}",
        columns.len()
    );
    let mut found = Vec::new();
    for (table, column) in columns {
        // AssertSqlSafe: the identifiers come from information_schema, quoted; the pattern binds.
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT count(*) FROM \"{table}\" WHERE \"{column}\"::text ~ $1"
        )))
        .bind(RESERVED_ANYWHERE)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("scan {table}.{column}: {error}"));
        if count > 0 {
            found.push((format!("{table}.{column}"), count));
        }
    }
    found
}
