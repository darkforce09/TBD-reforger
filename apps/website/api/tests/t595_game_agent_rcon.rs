//! T-595 — `POST /admin/servers/:id/rcon` end to end, over a **real `AF_UNIX` socket**
//! serving **T-289's real rendered agent**.
//!
//! # Why this suite exists in this shape
//!
//! T-595 turns T-269's unconditional `503` into 202 / 409 / 503 decided by a host agent. The
//! failure mode that would make that worse than the 503 is a client that reports success over
//! a verb it never confirmed — this program's signature defect, aimed at the one endpoint
//! whose whole job is now to not do that. Unit tests on the mapping function cannot catch it,
//! because a mapping function tested against a hand-written `AgentReply` never proves the API
//! can *obtain* one.
//!
//! So nothing here is mocked at the protocol boundary:
//!
//! * The agent is **rendered by `scripts/mod/deploy-staging.sh --render-agent`** — the same
//!   function that renders it onto the host. Change the reply format there and this goes red.
//! * The channel is a **real `UnixListener`**. Each accepted connection spawns the agent with
//!   the socket on stdin and stdout, which is precisely what systemd's `Accept=yes` +
//!   `StandardInput=socket` does in the shipped unit.
//! * Only `systemctl` is a stub, for the reason T-289 gives in `agent_selftest()`: a dev box
//!   has no `tbd-reforger.service`, so the real one would collapse every case to `unreachable`
//!   and prove nothing; and the one host that has it is the live staging server. The stub is
//!   what makes `active`, `failed` and `not-found` producible on demand.
//!
//! # The case that is the ticket
//!
//! [`rejected_unit_that_systemctl_exited_zero_over_is_409`]. `systemctl --user restart` **exits
//! 0 over a dead server** on this host (`docs/mod/STAGING-SERVER.md:246-250`). The stub is set
//! to exactly that — verb returns 0, unit is `failed` — and the API must answer **409** with an
//! audit row that says REFUSED and names the state. An implementation that trusted the exit
//! status, or that read the agent's `ok` field, or that dropped `state`, fails here.
//!
//! Skips without `TEST_DATABASE_URL`, like every DB-backed suite in this crate.

use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

// ───────────────────────────── the agent, rendered once ─────────────────────────────

/// Repo root, derived from this crate's manifest dir (`<root>/apps/website/api`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("<root>/apps/website/api has three ancestors")
        .to_path_buf()
}

/// Render T-289's agent + a stub `systemctl`, once per test binary.
///
/// Rendering through `deploy-staging.sh` rather than copying the script is the point: this
/// suite must fail if the shipped agent's wire format drifts from what
/// `services::game_agent` parses. A vendored copy would keep passing while the real host
/// spoke something else.
fn agent_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let root = repo_root();
        let out = std::env::temp_dir().join(format!("t595-agent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);

        // T-853: was `bash scripts/mod/deploy-staging.sh --render-agent <out>`. That script is now
        // `cargo xtask deploy staging`. This test renders the agent with the REAL renderer rather
        // than a fixture on purpose — a copy would keep passing while the thing actually deployed
        // drifted — so the invocation had to move with it, not be stubbed out.
        //
        // `cargo run` and not a prebuilt path: this test must work from a clean checkout and from
        // any worktree, and asking cargo is the only way to be sure the binary matches THIS tree.
        // CARGO_TARGET_DIR is deliberately left alone — cargo inherits whatever the caller set, so
        // the gate's own dir is used when the gate runs this, and the default otherwise.
        let status = std::process::Command::new("cargo")
            .args(["run", "-q", "-p", "xtask", "--", "deploy", "staging", "--"])
            .arg("--render-agent")
            .arg(&out)
            .current_dir(&root)
            .output()
            .unwrap_or_else(|e| panic!("T-595: cannot run `cargo xtask deploy staging`: {e}"));
        assert!(
            status.status.success(),
            "T-595: `cargo xtask deploy staging -- --render-agent` failed ({}):\n{}\n{}",
            status.status,
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
        );
        assert!(
            out.join("tbd-reforger-agent.sh").is_file(),
            "T-595: the renderer produced no agent script in {}",
            out.display()
        );

        // Stub systemctl, byte-for-byte T-289's own (`agent_selftest`). STUB_LOAD/STUB_ACTIVE
        // are the unit's state; STUB_VERB_RC is what the verb returns — deliberately
        // independent, so "verb says OK, unit is dead" is expressible.
        let bin = out.join("bin");
        std::fs::create_dir_all(&bin).expect("mkdir stub bin");
        let stub = bin.join("systemctl");
        std::fs::write(
            &stub,
            "#!/usr/bin/env bash\n\
             for a in \"$@\"; do\n\
             \x20 if [ \"$a\" = \"show\" ]; then\n\
             \x20   printf 'LoadState=%s\\nActiveState=%s\\n' \"${STUB_LOAD:-loaded}\" \
             \"${STUB_ACTIVE:-inactive}\"\n\
             \x20   exit 0\n\
             \x20 fi\n\
             done\n\
             exit \"${STUB_VERB_RC:-0}\"\n",
        )
        .expect("write stub systemctl");
        set_executable(&stub);
        out
    })
}

fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).expect("stat").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

/// What the stub `systemctl` reports, and what its verb returns.
#[derive(Clone, Copy)]
struct Stub {
    load: &'static str,
    active: &'static str,
    /// Exit status of `systemctl <verb>` — **independent of the state above**, which is the
    /// whole point.
    verb_rc: &'static str,
    /// Seconds the agent dwells before re-reading the unit.
    dwell: &'static str,
}

impl Stub {
    const fn unit(active: &'static str) -> Self {
        Stub {
            load: "loaded",
            active,
            verb_rc: "0",
            dwell: "0",
        }
    }
}

/// A live agent behind a real `AF_UNIX` socket.
struct AgentHarness {
    socket: PathBuf,
    task: tokio::task::JoinHandle<()>,
}

impl AgentHarness {
    /// Bind a socket and serve T-289's agent on it, one process per connection.
    async fn start(name: &str, stub: Stub) -> Self {
        let dir = agent_dir();
        // sun_path is ~108 bytes — keep this short and out of the (deep) worktree path.
        let socket = std::env::temp_dir().join(format!("t595-{name}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);

        let listener = tokio::net::UnixListener::bind(&socket)
            .unwrap_or_else(|e| panic!("T-595: bind {}: {e}", socket.display()));

        let script = dir.join("tbd-reforger-agent.sh");
        let path_env = format!(
            "{}:{}",
            dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );

        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                // The connection becomes the agent's stdin AND stdout — exactly what
                // `Accept=yes` + `StandardInput=socket`/`StandardOutput=socket` provide.
                let std_stream = stream.into_std().expect("into_std");
                std_stream.set_nonblocking(false).expect("blocking socket");
                let out = std_stream.try_clone().expect("dup for stdout");

                let child = tokio::process::Command::new("bash")
                    .arg(&script)
                    .env("PATH", &path_env)
                    .env("TBD_AGENT_UNIT", "tbd-reforger.service")
                    .env("TBD_AGENT_DWELL_S", stub.dwell)
                    .env("STUB_LOAD", stub.load)
                    .env("STUB_ACTIVE", stub.active)
                    .env("STUB_VERB_RC", stub.verb_rc)
                    .stdin(Stdio::from(OwnedFd::from(std_stream)))
                    .stdout(Stdio::from(OwnedFd::from(out)))
                    .stderr(Stdio::null())
                    .spawn();
                match child {
                    Ok(mut c) => {
                        tokio::spawn(async move {
                            let _ = c.wait().await;
                        });
                    }
                    Err(e) => panic!("T-595: cannot spawn the agent: {e}"),
                }
            }
        });

        AgentHarness { socket, task }
    }

    fn path(&self) -> String {
        self.socket.to_string_lossy().into_owned()
    }
}

impl Drop for AgentHarness {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket);
    }
}

// ───────────────────────────── app + fixtures ─────────────────────────────

/// Boot the API with `GAME_AGENT_SOCKET` pointed at `socket` (empty = no transport).
async fn boot(socket: &str) -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let mut cfg = Config::for_tests(url, "t595-secret");
    cfg.game_agent_socket = socket.to_string();
    let app = app::router(AppState::new(pool.clone(), cfg));
    Some((app, pool))
}

/// A `servers` row this test owns, so the audit assertion can scope by `target_id`.
async fn seed_server(pool: &PgPool, name: &str) -> String {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO servers (id, name, ip, port, is_active) \
         VALUES ($1, $2, '127.0.0.1'::inet, 2001, true)",
    )
    .bind(id)
    .bind(name)
    .execute(pool)
    .await
    .expect("seed servers row");
    id.to_string()
}

async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "t595_game_agent_rcon", "admin").await
}

async fn post_rcon(app: &Router, tok: &str, server_id: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/admin/servers/{server_id}/rcon"))
        .header(header::AUTHORIZATION, format!("Bearer {tok}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build rcon request");
    let resp = app.clone().oneshot(req).await.expect("rcon request");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("rcon body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The `server.rcon` audit row this request wrote: `(severity, message)`.
///
/// **Exactly one** row must exist per request — the assertion is on the count as much as the
/// content, because "audited the attempt, then audited the outcome" would leave two rows and
/// an audit log that contradicts itself.
async fn rcon_audit_row(pool: &PgPool, server_id: &str) -> (String, String) {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT severity::text, message FROM audit_logs \
         WHERE action = 'server.rcon' AND target_id = $1 ORDER BY created_at",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .expect("read audit rows");
    assert_eq!(
        rows.len(),
        1,
        "T-595: exactly one audit row per RCON request, got {rows:#?}"
    );
    rows.into_iter().next().expect("one row")
}

// ───────────────────────────── the matrix ─────────────────────────────

/// **202.** The agent ran `restart` and re-read the unit as `active`.
///
/// The audit row must be `info` and must say DELIVERED **and name the state**. A row that
/// recorded the attempt ("attempted RCON … NOT delivered") over a restart that worked is
/// T-269's placeholder outliving its premise, which is the defect this ticket closes.
#[tokio::test]
async fn accepted_restart_is_202_and_the_audit_row_records_the_outcome() {
    let agent = AgentHarness::start("accepted", Stub::unit("active")).await;
    let Some((app, pool)) = boot(&agent.path()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 accepted").await;

    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;

    assert_eq!(status, StatusCode::ACCEPTED, "body: {body}");
    assert_eq!(body["accepted"], Value::Bool(true));
    assert_eq!(body["delivered"], Value::Bool(true));
    assert_eq!(
        body["state"], "active",
        "the observed state must reach the client: {body}"
    );
    assert_eq!(body["action"], "restart");

    let (severity, message) = rcon_audit_row(&pool, &server).await;
    assert_eq!(
        severity, "info",
        "a delivered+accepted command is not a warning"
    );
    assert!(
        message.contains("DELIVERED and accepted"),
        "audit row must record the OUTCOME: {message}"
    );
    assert!(
        message.contains("state=active"),
        "audit row must name the state its claim rests on: {message}"
    );
    assert!(
        !message.contains("attempted"),
        "T-269's attempt-shaped wording must not survive a real delivery: {message}"
    );
}

/// **409 — the ticket.** `systemctl` exits **0** and the unit is `failed`.
///
/// `docs/mod/STAGING-SERVER.md:246-250` documents this exact host doing this: with
/// `a2sPort == bindPort` the engine logs "Unable to start replication" → "Game destroyed" and
/// exits 0, so `Restart=on-failure` does not fire. Every layer here has to refuse to believe
/// the exit status: the agent re-reads the unit, and the API branches on `result` (not `ok`,
/// not a status code) and carries `state` into the answer.
#[tokio::test]
async fn rejected_unit_that_systemctl_exited_zero_over_is_409() {
    let agent = AgentHarness::start("rejected", Stub::unit("failed")).await;
    let Some((app, pool)) = boot(&agent.path()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 rejected").await;

    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;

    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "systemctl exited 0 over a dead unit — this must NOT be a 202, and must not be \
         flattened into a 503 either: {body}"
    );
    assert_eq!(body["details"]["accepted"], Value::Bool(false));
    assert_eq!(
        body["details"]["delivered"],
        Value::Bool(true),
        "the agent DID run the verb; calling this undelivered sends the operator after a \
         network fault: {body}"
    );
    assert_eq!(body["details"]["state"], "failed", "body: {body}");

    let (severity, message) = rcon_audit_row(&pool, &server).await;
    assert_eq!(severity, "warn");
    assert!(
        message.contains("REFUSED"),
        "audit row must record the refusal: {message}"
    );
    assert!(
        message.contains("state=failed"),
        "audit row must name the state that refused it: {message}"
    );
}

/// **503, agent reachable, unit absent.** The stub reports `LoadState=not-found`, so the agent
/// answers `unreachable` — distinct from the socket being down.
#[tokio::test]
async fn agent_reporting_an_uninstalled_unit_is_503() {
    let agent = AgentHarness::start(
        "notfound",
        Stub {
            load: "not-found",
            ..Stub::unit("inactive")
        },
    )
    .await;
    let Some((app, pool)) = boot(&agent.path()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 notfound").await;

    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert_eq!(body["details"]["delivered"], Value::Bool(false));
    let (severity, message) = rcon_audit_row(&pool, &server).await;
    assert_eq!(severity, "warn");
    assert!(
        message.contains("NOT delivered"),
        "audit row must not claim delivery: {message}"
    );
}

/// **503, no listener.** The socket path is configured and nothing is behind it — a crashed
/// agent, or `TBD_INSTALL_AGENT=1` never run. `connect(2)` fails and the API says so.
#[tokio::test]
async fn a_socket_with_no_listener_is_503_not_a_success() {
    let dead = std::env::temp_dir().join(format!("t595-dead-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&dead);
    let Some((app, pool)) = boot(&dead.to_string_lossy()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 dead socket").await;

    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert_eq!(body["details"]["delivered"], Value::Bool(false));
    let (severity, message) = rcon_audit_row(&pool, &server).await;
    assert_eq!(severity, "warn");
    assert!(message.contains("NOT delivered"), "{message}");
}

/// **503, nothing configured.** `GAME_AGENT_SOCKET` unset is the state of every developer box,
/// and it must fail closed rather than connect to `Path::new("")`.
#[tokio::test]
async fn an_unconfigured_socket_is_503_and_says_which_var_is_missing() {
    let Some((app, pool)) = boot("").await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 unconfigured").await;

    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert!(
        body["details"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("GAME_AGENT_SOCKET"),
        "an unconfigured transport must name the variable, not read as an outage: {body}"
    );
    let (severity, _) = rcon_audit_row(&pool, &server).await;
    assert_eq!(severity, "warn");
}

/// **503 with a different message** for the three actions with no host representation.
///
/// `kick` is the interesting one: it is not blocked on transport at all. `RconInput` has no
/// player field and the SPA posts a bare `{"action":"kick"}`, so even a perfect channel into
/// the running server could not name a target. Reporting that as "no transport" would send an
/// operator to check a socket that is working perfectly.
#[tokio::test]
async fn kick_change_map_and_custom_are_refused_as_unsupported_not_as_no_transport() {
    let agent = AgentHarness::start("unsupported", Stub::unit("active")).await;
    let Some((app, pool)) = boot(&agent.path()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;

    for (label, body) in [
        ("kick", serde_json::json!({"action":"kick"})),
        (
            "change_map",
            serde_json::json!({"action":"change_map","map":"Everon"}),
        ),
        (
            "custom",
            serde_json::json!({"action":"custom","command":"#shutdown"}),
        ),
    ] {
        let server = seed_server(&pool, &format!("T-595 {label}")).await;
        let (status, resp) = post_rcon(&app, &tok, &server, body).await;
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{label} must still be refused: {resp}"
        );
        let err = resp["error"].as_str().unwrap_or_default();
        assert!(
            err.contains("not supported on this deployment"),
            "{label} must be refused as UNSUPPORTED, not as a transport failure — the agent is \
             up and answering in this test: {err}"
        );
        assert_eq!(resp["details"]["delivered"], Value::Bool(false));

        let (severity, message) = rcon_audit_row(&pool, &server).await;
        assert_eq!(severity, "warn");
        assert!(
            message.contains("no representation on the host agent"),
            "the audit row must record why, not just that it failed: {message}"
        );
    }
}

/// **The dwell, over the wire.** The agent sleeps before re-reading the unit; a client timeout
/// at or under that sleep would turn every honest slow answer into a false `unreachable`.
///
/// This drives the agent with a **9-second dwell** — above the shipped 8s default and below
/// [`website_api::services::game_agent::AGENT_TIMEOUT`] — and demands a 202. The unit source
/// pin proves the two numbers are ordered; this proves the client actually waits. Drop
/// `AGENT_TIMEOUT` to 5s and this is the test that goes red, with the exact symptom an
/// operator would have seen: a healthy server reported unreachable.
///
/// It is slow on purpose. That is the cost of testing a timeout instead of asserting one.
#[tokio::test]
async fn the_client_waits_out_the_agents_dwell_instead_of_calling_it_unreachable() {
    let agent = AgentHarness::start(
        "dwell",
        Stub {
            dwell: "9",
            ..Stub::unit("active")
        },
    )
    .await;
    let Some((app, pool)) = boot(&agent.path()).await else {
        panic!("T-595: TEST_DATABASE_URL required — a skip here is a failure to have tested");
    };
    let tok = admin_token(&app).await;
    let server = seed_server(&pool, "T-595 dwell").await;

    let started = std::time::Instant::now();
    let (status, body) =
        post_rcon(&app, &tok, &server, serde_json::json!({"action":"restart"})).await;
    let waited = started.elapsed();

    // The property, asserted FIRST so an impatient client is diagnosed as an impatient client.
    // (Measured with AGENT_TIMEOUT perturbed to 5s: this fires at `waited = 5.01s` with a 503.)
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "a healthy server that took {waited:?} to answer must be a 202, not an `unreachable` \
         invented by an impatient client: {body}"
    );
    // Non-vacuity: if the stub ever stopped dwelling, the assertion above would pass for the
    // wrong reason and this test would prove nothing about the timeout at all.
    assert!(
        waited >= std::time::Duration::from_secs(9),
        "the agent was configured to dwell 9s but answered in {waited:?} — the dwell did not \
         happen, so the 202 above says nothing about AGENT_TIMEOUT"
    );
    assert_eq!(body["state"], "active");
    let (severity, _) = rcon_audit_row(&pool, &server).await;
    assert_eq!(severity, "info");
}
