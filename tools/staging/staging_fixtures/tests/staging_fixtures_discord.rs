//! The `staging-fixtures` host tool's Discord procedure subcommands, `age-membership-snapshot`,
//! `observe-discord-member` and `spend-discord-member-bucket`, run as a process.
//!
//! Every test runs the built binary (`CARGO_BIN_EXE_staging-fixtures`) against this suite's own
//! database through an API env file the test writes. The aging tests read the snapshot and the
//! audit log back. The member reads go to a fake Discord this suite serves on a loopback port
//! through the tool's test-only `--discord-api-base`, which counts every request and records the
//! `Authorization` header it received: no test reaches Discord. Every run is checked for the bot
//! token and the connection string on stdout and stderr.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

mod common;

/// The main guild of the API env file.
const GUILD: &str = "741000000000000900";
/// A partner guild.
const PARTNER_GUILD: &str = "1400000000000000002";
/// The operator whose snapshot ages and whose membership the fake reports.
const OPERATOR: &str = "741000000000000901";
/// An account with no membership snapshot.
const UNVERIFIED: &str = "741000000000000902";
/// The role the fake reports the member holding.
const PARTNER_ROLE: &str = "741000000000000777";
/// The bot token of the API env file; the fake expects exactly `Bot <this>`.
const BOT_TOKEN: &str = "c3RhZ2luZy1maXh0dXJlcw.fake-bot.token-value";

/// The aging tests share the operator's rows.
static SNAPSHOT_LOCK: Mutex<()> = Mutex::const_new(());

/// One finished run of the tool.
struct ToolRun {
    code: i32,
    stdout: String,
    stderr: String,
}

impl ToolRun {
    /// The JSON records of the lines starting with `prefix`.
    fn records(&self, prefix: &str) -> Vec<Value> {
        self.stdout
            .lines()
            .filter_map(|line| line.strip_prefix(prefix)?.strip_prefix(' '))
            .map(|json| serde_json::from_str(json).expect("a JSON record"))
            .collect()
    }
}

/// A test's database and API env file; dropping it removes the workspace.
struct Fixture {
    pool: PgPool,
    database: String,
    database_url: String,
    workspace: PathBuf,
    env_file: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace);
    }
}

async fn fixture(bot_token: &str) -> Fixture {
    let database_url = common::require_test_database_url().expect("the suite's database");
    let pool = PgPool::connect(&database_url).await.expect("connect");
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&pool)
        .await
        .expect("database name");
    let workspace = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("staging_fixtures_discord")
        .join(Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&workspace).expect("workspace");
    let env_file = workspace.join("api.env");
    fs::write(
        &env_file,
        format!(
            "DATABASE_URL={database_url}\nDISCORD_GUILD_ID={GUILD}\nDISCORD_BOT_TOKEN={bot_token}\n"
        ),
    )
    .expect("API env file");
    Fixture {
        pool,
        database,
        database_url,
        workspace,
        env_file,
    }
}

impl Fixture {
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
        let printed = format!("{}\n{}", run.stdout, run.stderr);
        assert!(
            !printed.contains(BOT_TOKEN),
            "the bot token was printed:\n{printed}"
        );
        assert!(
            !printed.contains("fake-bot"),
            "part of the bot token was printed"
        );
        assert!(
            !printed.contains(&self.database_url),
            "DATABASE_URL printed"
        );
        run
    }

    async fn snapshot(&self, discord_id: &str) -> (String, DateTime<Utc>, i64) {
        sqlx::query_as(
            "SELECT membership_status, verified_at, revision FROM discord_membership_snapshots
             WHERE discord_id = $1 AND guild_id = $2",
        )
        .bind(discord_id)
        .bind(GUILD)
        .fetch_one(&self.pool)
        .await
        .expect("snapshot")
    }
}

/// How the fake Discord answers a member read.
#[derive(Debug, Clone, Copy)]
enum FakeMode {
    /// A bucket of `limit` reads per `window_ms`, answering a member; 429 once spent.
    Bucket { limit: u32, window_ms: u64 },
    /// Unknown Member.
    Nonmember,
    /// 401: the token is refused.
    RefusesToken,
}

/// What the fake has seen.
struct FakeState {
    mode: FakeMode,
    requests: AtomicU32,
    seen: StdMutex<Vec<(String, String)>>,
    window: StdMutex<(Option<Instant>, u32)>,
}

/// A fake Discord API on a loopback port.
struct FakeDiscord {
    state: Arc<FakeState>,
    base: String,
}

impl FakeDiscord {
    async fn serve(mode: FakeMode) -> Self {
        let state = Arc::new(FakeState {
            mode,
            requests: AtomicU32::new(0),
            seen: StdMutex::new(Vec::new()),
            window: StdMutex::new((None, 0)),
        });
        let app = Router::new()
            .route("/api/v10/guilds/{guild}/members/{member}", get(member_read))
            .with_state(Arc::clone(&state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let base = format!("http://{}/api/v10", listener.local_addr().expect("address"));
        tokio::spawn(async move { axum::serve(listener, app).await.expect("fake Discord") });
        Self { state, base }
    }

    fn requests(&self) -> u32 {
        self.state.requests.load(Ordering::SeqCst)
    }

    /// The `(path, Authorization)` of every request, in order.
    fn seen(&self) -> Vec<(String, String)> {
        self.state.seen.lock().expect("seen").clone()
    }
}

async fn member_read(
    State(state): State<Arc<FakeState>>,
    UrlPath((guild, member)): UrlPath<(String, String)>,
    headers: HeaderMap,
) -> Response {
    state.requests.fetch_add(1, Ordering::SeqCst);
    let authorization = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let path = format!("/guilds/{guild}/members/{member}");
    state.seen.lock().expect("seen").push((path, authorization));
    let member_body = json!({"roles": [PARTNER_ROLE], "nick": null, "user": {"id": member}});
    match state.mode {
        FakeMode::Nonmember => (
            StatusCode::NOT_FOUND,
            Json(json!({"message": "Unknown Member", "code": 10007})),
        )
            .into_response(),
        FakeMode::RefusesToken => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"message": "401: Unauthorized", "code": 0})),
        )
            .into_response(),
        FakeMode::Bucket { limit, window_ms } => {
            let window = Duration::from_millis(window_ms);
            let now = Instant::now();
            let mut guard = state.window.lock().expect("window");
            let started = match guard.0 {
                Some(started) if now < started + window => started,
                _ => {
                    *guard = (Some(now), 0);
                    now
                }
            };
            let reset_after = (started + window - now).as_secs_f64();
            let headers = |remaining: u32| {
                [
                    ("x-ratelimit-bucket", "fakebucket".to_owned()),
                    ("x-ratelimit-limit", limit.to_string()),
                    ("x-ratelimit-remaining", remaining.to_string()),
                    ("x-ratelimit-reset-after", format!("{reset_after:.3}")),
                ]
            };
            if guard.1 < limit {
                guard.1 += 1;
                (StatusCode::OK, headers(limit - guard.1), Json(member_body)).into_response()
            } else {
                let body = json!({"message": "You are being rate limited.",
                                  "retry_after": reset_after, "global": false});
                (StatusCode::TOO_MANY_REQUESTS, headers(0), Json(body)).into_response()
            }
        }
    }
}

fn unix_now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis(),
    )
    .expect("milliseconds")
}

async fn seed_verified_operator(pool: &PgPool) {
    common::seed_user(
        pool,
        OPERATOR,
        "Staging Operator",
        &common::unique_arma("discord"),
        "admin",
    )
    .await;
    common::fixtures::seed_membership(pool, OPERATOR, GUILD, "admin").await;
}

#[tokio::test]
async fn staging_fixtures_discord_aging_sets_verified_at() {
    let _rows: MutexGuard<'_, ()> = SNAPSHOT_LOCK.lock().await;
    let fixture = fixture("").await;
    seed_verified_operator(&fixture.pool).await;
    let (_, verified_before, revision_before) = fixture.snapshot(OPERATOR).await;
    let age = [
        "age-membership-snapshot",
        "--discord-id",
        OPERATOR,
        "--hours",
        "49",
    ];

    let dry = fixture.run(&age).await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(
        dry.stdout
            .contains("age plan: discord_id=741000000000000901"),
        "{}",
        dry.stdout
    );
    assert_eq!(
        fixture.snapshot(OPERATOR).await.1,
        verified_before,
        "a dry run writes nothing"
    );

    let run = fixture.run(&[&age[..], &["--apply"]].concat()).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout.contains("membership snapshot aged"),
        "{}",
        run.stdout
    );
    let (status, verified_after, revision_after) = fixture.snapshot(OPERATOR).await;
    let age = Utc::now() - verified_after;
    assert!(
        age > chrono::Duration::hours(49) - chrono::Duration::minutes(1)
            && age < chrono::Duration::hours(49) + chrono::Duration::minutes(1),
        "verified_at is 49 hours old: {verified_after}"
    );
    assert_eq!(
        (status.as_str(), revision_after),
        ("member", revision_before)
    );
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs
         WHERE action = 'staging.membership_snapshot_aged' AND target_id = $1",
    )
    .bind(OPERATOR)
    .fetch_one(&fixture.pool)
    .await
    .expect("aging audit");
    assert!(audited >= 1, "the aging is audited");
}

#[tokio::test]
async fn staging_fixtures_discord_aging_refuses_what_it_cannot_stage() {
    let _rows: MutexGuard<'_, ()> = SNAPSHOT_LOCK.lock().await;
    let fixture = fixture("").await;
    seed_verified_operator(&fixture.pool).await;
    common::seed_user(
        &fixture.pool,
        UNVERIFIED,
        "Unverified",
        &common::unique_arma("discord"),
        "enlisted",
    )
    .await;
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id = $1")
        .bind(UNVERIFIED)
        .execute(&fixture.pool)
        .await
        .expect("no snapshot");
    for (id, hours, reason) in [
        ("741000000000000999", "49", "no account has the Discord id"),
        (UNVERIFIED, "49", "has no membership snapshot"),
        (OPERATOR, "0", "--hours takes 1 to 168"),
        (OPERATOR, "169", "--hours takes 1 to 168"),
    ] {
        let run = fixture
            .run(&[
                "age-membership-snapshot",
                "--discord-id",
                id,
                "--hours",
                hours,
                "--apply",
            ])
            .await;
        assert_eq!(run.code, 2, "{id} {hours}: {}", run.stderr);
        assert!(run.stderr.contains(reason), "{reason}: {}", run.stderr);
    }
    sqlx::query(
        "UPDATE discord_membership_snapshots SET lease_token = gen_random_uuid(),
         lease_expires_at = clock_timestamp() + interval '45 seconds'
         WHERE discord_id = $1 AND guild_id = $2",
    )
    .bind(OPERATOR)
    .bind(GUILD)
    .execute(&fixture.pool)
    .await
    .expect("a live lease");
    let (_, verified_before, _) = fixture.snapshot(OPERATOR).await;
    let run = fixture
        .run(&[
            "age-membership-snapshot",
            "--discord-id",
            OPERATOR,
            "--hours",
            "49",
            "--apply",
        ])
        .await;
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("holds the snapshot's lease"),
        "{}",
        run.stderr
    );
    assert_eq!(fixture.snapshot(OPERATOR).await.1, verified_before);
}

#[tokio::test]
async fn staging_fixtures_discord_observe_reports_the_member_read() {
    let fixture = fixture(BOT_TOKEN).await;
    let fake = FakeDiscord::serve(FakeMode::Bucket {
        limit: 5,
        window_ms: 60_000,
    })
    .await;
    let observe = |guild: &'static [&'static str]| {
        let base = fake.base.clone();
        let fixture = &fixture;
        async move {
            let mut arguments = vec!["observe-discord-member", "--discord-id", OPERATOR];
            arguments.extend_from_slice(guild);
            arguments.extend_from_slice(&["--discord-api-base", &base]);
            fixture.run(&arguments).await
        }
    };
    let dry = observe(&["--guild", "main"]).await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(
        dry.stdout.contains("dry run: no request sent to Discord"),
        "{}",
        dry.stdout
    );
    assert_eq!(fake.requests(), 0, "a dry run sends nothing");

    let main = observe(&["--guild", "main", "--apply"]).await;
    assert_eq!(main.code, 0, "{}", main.stderr);
    let partner = observe(&[
        "--guild",
        "partner",
        "--partner-guild-id",
        PARTNER_GUILD,
        "--apply",
    ])
    .await;
    assert_eq!(partner.code, 0, "{}", partner.stderr);
    let records = [
        main.records("discord-member-read"),
        partner.records("discord-member-read"),
    ];
    for (record, (guild, guild_id)) in records
        .iter()
        .zip([("main", GUILD), ("partner", PARTNER_GUILD)])
    {
        assert_eq!(record.len(), 1, "one read");
        let read = &record[0];
        assert_eq!(read["guild"], guild);
        assert_eq!(read["guild_id"], guild_id);
        assert_eq!(read["discord_id"], OPERATOR);
        assert_eq!(read["http_status"], 200);
        assert_eq!(read["outcome"], "member");
        assert_eq!(read["roles"], json!([PARTNER_ROLE]));
        assert_eq!(read["rate_limit"]["limit"], 5);
        assert_eq!(read["rate_limit"]["bucket"], "fakebucket");
    }
    assert_eq!(
        fake.seen(),
        [
            (
                format!("/guilds/{GUILD}/members/{OPERATOR}"),
                format!("Bot {BOT_TOKEN}")
            ),
            (
                format!("/guilds/{PARTNER_GUILD}/members/{OPERATOR}"),
                format!("Bot {BOT_TOKEN}")
            ),
        ]
    );
    let nonmember = FakeDiscord::serve(FakeMode::Nonmember).await;
    let base = nonmember.base.clone();
    let run = fixture
        .run(&[
            "observe-discord-member",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--discord-api-base",
            &base,
            "--apply",
        ])
        .await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.records("discord-member-read")[0]["outcome"],
        "nonmember"
    );
}

#[tokio::test]
async fn staging_fixtures_discord_probe_refusals_send_no_request() {
    let fixture = fixture(BOT_TOKEN).await;
    let refused = FakeDiscord::serve(FakeMode::RefusesToken).await;
    let base = refused.base.clone();
    let run = fixture
        .run(&[
            "observe-discord-member",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--discord-api-base",
            &base,
            "--apply",
        ])
        .await;
    assert_eq!(run.code, 1, "an unavailable read fails: {}", run.stderr);
    let read = &run.records("discord-member-read")[0];
    assert_eq!(
        (read["outcome"].as_str(), read["reason"].as_str()),
        (Some("unavailable"), Some("Discord refused the bot token"))
    );
    assert_eq!(refused.requests(), 1);

    let remote = fixture
        .run(&[
            "observe-discord-member",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--discord-api-base",
            "https://discord.com/api/v10",
            "--apply",
        ])
        .await;
    assert_eq!(remote.code, 2, "{}", remote.stderr);
    assert!(remote.stderr.contains("loopback"), "{}", remote.stderr);
    let unset = fixture_without_token().await;
    let run = unset
        .run(&[
            "observe-discord-member",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--discord-api-base",
            &base,
            "--apply",
        ])
        .await;
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("DISCORD_BOT_TOKEN is not set"),
        "{}",
        run.stderr
    );
    assert_eq!(refused.requests(), 1, "no refused run reached the fake");
}

async fn fixture_without_token() -> Fixture {
    fixture("").await
}

/// `spend-discord-member-bucket` against `fake`, starting 2.5 s from now.
async fn spend(fixture: &Fixture, fake: &FakeDiscord, hold: &str, max: &str) -> (ToolRun, u64) {
    let start = unix_now_ms() + 2500;
    let start_text = start.to_string();
    let run = fixture
        .run(&[
            "spend-discord-member-bucket",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--start-at-unix-ms",
            &start_text,
            "--hold-seconds",
            hold,
            "--max-requests",
            max,
            "--discord-api-base",
            &fake.base,
            "--apply",
        ])
        .await;
    (run, start)
}

#[tokio::test]
async fn staging_fixtures_discord_bucket_spend_spends_and_holds_the_bucket() {
    let fixture = fixture(BOT_TOKEN).await;
    let fake = FakeDiscord::serve(FakeMode::Bucket {
        limit: 5,
        window_ms: 400,
    })
    .await;
    let (run, start) = spend(&fixture, &fake, "2", "50").await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    let reads = run.records("discord-member-read");
    let summary = &run.records("discord-bucket-spend")[0];
    assert_eq!(summary["bucket_spent"], true, "{summary}");
    assert_eq!(summary["stopped_because"], "hold_ended", "{summary}");
    assert_eq!(summary["requests"], json!(reads.len()));
    assert_eq!(
        fake.requests() as usize,
        reads.len(),
        "every request is reported"
    );
    assert!(reads.len() <= 50, "at most 50 requests: {}", reads.len());
    assert!(
        reads.len() >= 10,
        "the hold re-spends the bucket after its resets: {}",
        reads.len()
    );
    assert_eq!(
        summary["rate_limited_responses"], 0,
        "no request lands on a spent bucket"
    );
    let hold_until = start + 2000;
    for read in &reads {
        let sent = read["sent_at_unix_ms"].as_u64().expect("sent at");
        assert!(
            sent >= start && sent < hold_until,
            "sent within the hold: {read}"
        );
    }
}

#[tokio::test]
async fn staging_fixtures_discord_bucket_spend_never_passes_its_request_cap() {
    let fixture = fixture(BOT_TOKEN).await;
    let endless = FakeDiscord::serve(FakeMode::Bucket {
        limit: 1000,
        window_ms: 60_000,
    })
    .await;
    let (run, _) = spend(&fixture, &endless, "2", "7").await;
    assert_eq!(run.code, 1, "an unspent bucket fails: {}", run.stderr);
    assert!(
        run.stderr.contains("not seen spent after 7 requests"),
        "{}",
        run.stderr
    );
    assert_eq!(
        run.records("discord-bucket-spend")[0]["stopped_because"],
        "request_cap"
    );
    assert_eq!(endless.requests(), 7, "exactly the cap");

    let (over, _) = spend(&fixture, &endless, "2", "51").await;
    assert_eq!(over.code, 2, "{}", over.stderr);
    assert!(
        over.stderr.contains("--max-requests takes 1 to 50"),
        "{}",
        over.stderr
    );
    let past = (unix_now_ms() - 5000).to_string();
    let late = fixture
        .run(&[
            "spend-discord-member-bucket",
            "--discord-id",
            OPERATOR,
            "--guild",
            "main",
            "--start-at-unix-ms",
            &past,
            "--hold-seconds",
            "2",
            "--max-requests",
            "5",
            "--discord-api-base",
            &endless.base,
            "--apply",
        ])
        .await;
    assert_eq!(late.code, 2, "{}", late.stderr);
    assert!(late.stderr.contains("passed"), "{}", late.stderr);
    assert_eq!(endless.requests(), 7, "refused runs send nothing");

    let refused = FakeDiscord::serve(FakeMode::RefusesToken).await;
    let (run, _) = spend(&fixture, &refused, "2", "50").await;
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(
        run.records("discord-bucket-spend")[0]["stopped_because"],
        "discord_unavailable"
    );
    assert_eq!(
        refused.requests(),
        1,
        "a refused token ends the spend at once"
    );
}
