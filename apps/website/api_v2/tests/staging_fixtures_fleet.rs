//! The `staging-fixtures` host tool's fleet subcommands, `provision-fleet` and
//! `rotate-credential`, run as a process.
//!
//! Every test runs the built binary (`CARGO_BIN_EXE_staging-fixtures`) against this suite's own
//! database through an API env file the test writes, then reads the database and the secrets root
//! back: a run without `--apply` writes nothing, a refused run writes nothing, an applied run
//! writes its rows, audit rows and mode-600 files, and no secret reaches stdout or stderr. The
//! fleet's server names are fixed, so the tests run one at a time behind [`FLEET_LOCK`], and each
//! starts from a database without fleet servers.

use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sqlx::PgPool;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;
use website_api::server_infrastructure::models::machine_credential::ExecutorKind;
use website_api::server_infrastructure::services::machine_credentials::{
    authenticate_machine, revoke_machine_credential,
};

mod common;

/// The administrator the fleet is provisioned for.
const OPERATOR: &str = "741000000000000001";
/// An enlisted member, who holds no administrator authority.
const MEMBER: &str = "741000000000000002";
/// The main guild the tool judges the actor in.
const GUILD: &str = "staging-fixtures-fleet-guild";
/// The address the fleet's servers are registered at.
const ADDRESS: &str = "192.0.2.10";

/// The fleet's server names are fixed, so one test at a time owns them.
static FLEET_LOCK: Mutex<()> = Mutex::const_new(());

/// One finished run of the tool.
struct ToolRun {
    code: i32,
    stdout: String,
    stderr: String,
}

/// A test's database, API env file and secrets root; dropping it removes its workspace.
struct Fixture {
    pool: PgPool,
    database: String,
    database_url: String,
    workspace: PathBuf,
    env_file: PathBuf,
    secrets_root: PathBuf,
    audit_mark: i64,
    _fleet: MutexGuard<'static, ()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace);
    }
}

async fn fixture() -> Fixture {
    let fleet = FLEET_LOCK.lock().await;
    let database_url = common::require_test_database_url().expect("the suite's database");
    let pool = PgPool::connect(&database_url).await.expect("connect");
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&pool)
        .await
        .expect("database name");
    sqlx::query(
        "DELETE FROM server_machine_credentials WHERE server_id IN
         (SELECT id FROM servers WHERE name LIKE 'TBD Staging %')",
    )
    .execute(&pool)
    .await
    .expect("clear fleet credentials");
    sqlx::query("DELETE FROM servers WHERE name LIKE 'TBD Staging %'")
        .execute(&pool)
        .await
        .expect("clear fleet servers");
    for (account, name, role) in [
        (OPERATOR, "Fleet Operator", "admin"),
        (MEMBER, "Fleet Member", "enlisted"),
    ] {
        common::seed_user(&pool, account, name, &common::unique_arma("fleet"), role).await;
        common::fixtures::seed_membership(&pool, account, GUILD, role).await;
    }
    let audit_mark: i64 = sqlx::query_scalar("SELECT COALESCE(max(id), 0) FROM audit_logs")
        .fetch_one(&pool)
        .await
        .expect("audit mark");
    let workspace = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("staging_fixtures_fleet")
        .join(Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&workspace).expect("workspace");
    let env_file = workspace.join("api.env");
    fs::write(
        &env_file,
        format!("DATABASE_URL={database_url}\nDISCORD_GUILD_ID={GUILD}\n"),
    )
    .expect("API env file");
    Fixture {
        pool,
        database,
        database_url,
        env_file,
        secrets_root: workspace.join("fleet"),
        workspace,
        audit_mark,
        _fleet: fleet,
    }
}

impl Fixture {
    /// Run the tool with `arguments`, then the confirmation and the env file.
    async fn run(&self, arguments: &[&str]) -> ToolRun {
        self.run_confirming(arguments, &self.database).await
    }

    async fn run_confirming(&self, arguments: &[&str], database: &str) -> ToolRun {
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_staging-fixtures"))
            .args(arguments)
            .arg("--confirm-database")
            .arg(database)
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

    /// `provision-fleet` for five instances as the operator, plus `extra`.
    async fn provision(&self, extra: &[&str]) -> ToolRun {
        let root = self.root();
        let mut arguments = vec![
            "provision-fleet",
            "--instances",
            "5",
            "--actor",
            OPERATOR,
            "--ip",
            ADDRESS,
            "--secrets-root",
            root.as_str(),
        ];
        arguments.extend_from_slice(extra);
        self.run(&arguments).await
    }

    /// `rotate-credential` on `instance` for `executor`, plus `step`.
    async fn rotate(&self, instance: &str, executor: &str, step: &[&str]) -> ToolRun {
        let root = self.root();
        let mut arguments = vec![
            "rotate-credential",
            "--instance",
            instance,
            "--executor",
            executor,
            "--secrets-root",
            root.as_str(),
        ];
        arguments.extend_from_slice(step);
        self.run(&arguments).await
    }

    fn root(&self) -> String {
        self.secrets_root.display().to_string()
    }

    fn secrets_directory(&self, instance: u32) -> PathBuf {
        self.secrets_root
            .join(format!("instance-{instance}"))
            .join("secrets")
    }

    fn secret_file(&self, instance: u32, name: &str) -> PathBuf {
        self.secrets_directory(instance).join(name)
    }

    /// Every secret file under the secrets root, with its trimmed contents.
    fn secrets_on_disk(&self) -> Vec<(PathBuf, String)> {
        let mut secrets = Vec::new();
        for instance in fs::read_dir(&self.secrets_root)
            .into_iter()
            .flatten()
            .flatten()
        {
            let directory = instance.path().join("secrets");
            for file in fs::read_dir(&directory).into_iter().flatten().flatten() {
                if let Ok(text) = fs::read_to_string(file.path()) {
                    secrets.push((file.path(), text.trim().to_owned()));
                }
            }
        }
        secrets.sort();
        secrets
    }

    fn assert_no_secret_printed(&self, run: &ToolRun) {
        for output in [&run.stdout, &run.stderr] {
            assert!(
                !output.contains("tbdm_"),
                "a machine secret was printed:\n{output}"
            );
            assert!(
                !output.contains(&self.database_url),
                "DATABASE_URL was printed"
            );
            for (path, secret) in self.secrets_on_disk() {
                assert!(
                    secret.is_empty() || !output.contains(&secret),
                    "the secret of {} was printed",
                    path.display()
                );
            }
        }
    }

    /// `(id, name, host(ip), port, is_active)` of every fleet server, by port.
    async fn fleet_servers(&self) -> Vec<(Uuid, String, String, i64, bool)> {
        sqlx::query_as(
            "SELECT id, name, host(ip), port, is_active FROM servers
             WHERE name LIKE 'TBD Staging %' ORDER BY port",
        )
        .fetch_all(&self.pool)
        .await
        .expect("fleet servers")
    }

    /// `(id, server name, executor, label, revoked)` of every fleet credential, oldest first.
    async fn fleet_credentials(&self) -> Vec<(Uuid, String, String, String, bool)> {
        sqlx::query_as(
            "SELECT c.id, s.name, c.executor_kind, c.label, c.revoked_at IS NOT NULL
             FROM server_machine_credentials c JOIN servers s ON s.id = c.server_id
             WHERE s.name LIKE 'TBD Staging %' ORDER BY c.created_at, c.id",
        )
        .fetch_all(&self.pool)
        .await
        .expect("fleet credentials")
    }

    /// `(action, actor, target type, target id)` of the audit rows written since the fixture began.
    async fn new_audits(&self) -> Vec<(String, Option<String>, String, String)> {
        sqlx::query_as(
            "SELECT action, actor_id, target_type, target_id FROM audit_logs
             WHERE id > $1 ORDER BY id",
        )
        .bind(self.audit_mark)
        .fetch_all(&self.pool)
        .await
        .expect("new audit rows")
    }

    /// Refused, with `expected` in the message, and nothing written.
    async fn assert_refused_without_writes(&self, run: &ToolRun, expected: &str) {
        assert_eq!(
            run.code, 2,
            "stdout:\n{}\nstderr:\n{}",
            run.stdout, run.stderr
        );
        assert!(
            run.stderr.contains(expected),
            "expected `{expected}` in:\n{}",
            run.stderr
        );
        assert!(
            self.fleet_servers().await.is_empty(),
            "a refused run registered servers"
        );
        assert!(
            self.new_audits().await.is_empty(),
            "a refused run wrote audit rows"
        );
        assert!(
            self.secrets_on_disk().is_empty(),
            "a refused run wrote secret files"
        );
    }
}

/// An applying `provision-fleet` command line without the confirmation and the env file.
fn provision_arguments<'a>(
    actor: &'a str,
    instances: &'a str,
    ip: &'a str,
    root: &'a str,
) -> Vec<&'a str> {
    vec![
        "provision-fleet",
        "--instances",
        instances,
        "--actor",
        actor,
        "--ip",
        ip,
        "--secrets-root",
        root,
        "--apply",
    ]
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

async fn authenticated(pool: &PgPool, secret: &str) -> (Uuid, Uuid, ExecutorKind) {
    let caller = authenticate_machine(pool, secret)
        .await
        .expect("the secret authenticates");
    (caller.credential_id, caller.server_id, caller.executor)
}

#[tokio::test]
async fn staging_fixtures_provision_fleet_dry_run_writes_nothing() {
    let fixture = fixture().await;
    let run = fixture.provision(&[]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("dry run;"), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("reserved synthetic accounts present: 0"),
        "{}",
        run.stdout
    );
    assert_eq!(
        run.stdout.matches("plan instance=").count(),
        5,
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains("plan instance=5 server=\"TBD Staging 5\" address=192.0.2.10:2005")
    );
    assert!(
        run.stdout.contains("dry run: nothing written"),
        "{}",
        run.stdout
    );
    assert!(fixture.fleet_servers().await.is_empty());
    assert!(fixture.fleet_credentials().await.is_empty());
    assert!(fixture.new_audits().await.is_empty());
    assert!(
        !fixture.secrets_root.exists(),
        "a dry run created the secrets root"
    );
}

#[tokio::test]
async fn staging_fixtures_provision_fleet_apply_registers_servers_credentials_and_files() {
    let fixture = fixture().await;
    let run = fixture.provision(&["--apply"]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout
            .contains("provisioned 5 servers and 10 credentials"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("dry run"), "{}", run.stdout);

    let servers = fixture.fleet_servers().await;
    let expected: Vec<(String, String, i64, bool)> = (1..=5)
        .map(|n| {
            (
                format!("TBD Staging {n}"),
                ADDRESS.to_owned(),
                2000 + n,
                true,
            )
        })
        .collect();
    let stored: Vec<(String, String, i64, bool)> = servers
        .iter()
        .map(|(_, name, ip, port, active)| (name.clone(), ip.clone(), *port, *active))
        .collect();
    assert_eq!(stored, expected);

    let credentials = fixture.fleet_credentials().await;
    assert_eq!(credentials.len(), 10);
    for (instance, (server_id, name, ..)) in (1u32..).zip(&servers) {
        for (executor, file, label) in [
            (
                ExecutorKind::HostAgent,
                "host-agent-credential",
                "host agent",
            ),
            (
                ExecutorKind::ModRuntime,
                "mod-runtime-credential",
                "mod runtime",
            ),
        ] {
            let path = fixture.secret_file(instance, file);
            assert_eq!(mode(&path), 0o600, "{}", path.display());
            let secret = fs::read_to_string(&path).expect("secret file");
            let (credential, server, kind) = authenticated(&fixture.pool, secret.trim()).await;
            assert_eq!((server, kind), (*server_id, executor), "{}", path.display());
            let row = credentials
                .iter()
                .find(|row| row.0 == credential)
                .expect("credential row");
            assert_eq!(row.3, format!("{name} {label}"));
            assert!(!row.4, "a provisioned credential is live");
            assert!(
                run.stdout
                    .contains(&format!("id={credential} file={}", path.display()))
            );
        }
        let secrets = fixture.secrets_directory(instance);
        assert_eq!(mode(&secrets), 0o700, "{}", secrets.display());
        assert_eq!(mode(secrets.parent().expect("instance")), 0o700);
    }

    let audits = fixture.new_audits().await;
    let created: Vec<String> = audits
        .iter()
        .filter(|audit| audit.0 == "server.create")
        .map(|audit| audit.3.clone())
        .collect();
    let server_ids: Vec<String> = servers.iter().map(|server| server.0.to_string()).collect();
    assert_eq!(created, server_ids);
    let issued: Vec<String> = audits
        .iter()
        .filter(|audit| audit.0 == "server.credential_issued")
        .map(|audit| audit.3.clone())
        .collect();
    let credential_ids: Vec<String> = credentials.iter().map(|row| row.0.to_string()).collect();
    assert_eq!(issued, credential_ids);
    assert_eq!(audits.len(), 15);
    assert!(
        audits
            .iter()
            .all(|audit| audit.1.as_deref() == Some(OPERATOR))
    );
}

#[tokio::test]
async fn staging_fixtures_provision_fleet_never_overwrites_files_or_reregisters_servers() {
    let fixture = fixture().await;
    let occupied = fixture.secret_file(3, "mod-runtime-credential");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(occupied.parent().expect("secrets directory"))
        .expect("instance 3 secrets");
    fs::write(&occupied, "operator file").expect("occupying file");
    let run = fixture.provision(&["--apply"]).await;
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(run.stderr.contains("already exists"), "{}", run.stderr);
    assert!(fixture.fleet_servers().await.is_empty());
    assert!(fixture.new_audits().await.is_empty());
    assert_eq!(
        fs::read_to_string(&occupied).expect("occupying file"),
        "operator file"
    );
    assert!(
        !fixture.secrets_directory(1).exists(),
        "a refused run created instance 1"
    );

    fs::remove_file(&occupied).expect("free the path");
    let run = fixture.provision(&["--apply"]).await;
    assert_eq!(run.code, 0, "{}", run.stderr);
    let before = fixture.secrets_on_disk();
    assert_eq!(before.len(), 10);

    let again = fixture.provision(&["--apply"]).await;
    assert_eq!(again.code, 2, "{}", again.stderr);
    assert!(
        again.stderr.contains("already registered"),
        "{}",
        again.stderr
    );
    assert_eq!(fixture.fleet_servers().await.len(), 5);
    assert_eq!(fixture.fleet_credentials().await.len(), 10);
    assert_eq!(
        fixture.secrets_on_disk(),
        before,
        "a refused run changed a secret file"
    );
}

#[tokio::test]
async fn staging_fixtures_guards_refuse_before_any_write() {
    let fixture = fixture().await;
    let root = fixture.root();
    let wrong = fixture
        .run_confirming(
            &provision_arguments(OPERATOR, "5", ADDRESS, &root),
            "tbd_reforger",
        )
        .await;
    fixture
        .assert_refused_without_writes(&wrong, "does not match the connected database")
        .await;

    let refusals: [(Vec<&str>, &str); 9] = [
        (
            provision_arguments(MEMBER, "5", ADDRESS, &root),
            "does not hold administrator authority",
        ),
        (
            provision_arguments("741000000000000999", "5", ADDRESS, &root),
            "does not hold administrator authority",
        ),
        (
            provision_arguments("9100000000000000001", "5", ADDRESS, &root),
            "synthetic staging account",
        ),
        (
            provision_arguments(OPERATOR, "5", "tbd.example.com", &root),
            "literal IPv4 or IPv6",
        ),
        (
            provision_arguments(OPERATOR, "5", ADDRESS, "fleet"),
            "must be an absolute path",
        ),
        (
            provision_arguments(OPERATOR, "11", ADDRESS, &root),
            "--instances takes 1 to 10",
        ),
        (
            [
                provision_arguments(OPERATOR, "5", ADDRESS, &root),
                vec!["--bogus", "1"],
            ]
            .concat(),
            "takes no --bogus",
        ),
        (
            [
                provision_arguments(OPERATOR, "5", ADDRESS, &root),
                vec!["yes"],
            ]
            .concat(),
            "--apply is a switch",
        ),
        (
            [
                provision_arguments(OPERATOR, "5", ADDRESS, &root),
                vec!["--game-port-base", "65535"],
            ]
            .concat(),
            "port must be between",
        ),
    ];
    for (arguments, expected) in refusals {
        let run = fixture.run(&arguments).await;
        fixture.assert_refused_without_writes(&run, expected).await;
    }

    let shared = fixture.secrets_directory(1);
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&shared)
        .expect("instance 1 secrets directory");
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).expect("mode 755");
    let run = fixture.provision(&["--apply"]).await;
    fixture
        .assert_refused_without_writes(&run, "admits its owner only")
        .await;

    let unknown = fixture.run(&["retire-fleet"]).await;
    fixture
        .assert_refused_without_writes(&unknown, "unknown subcommand")
        .await;
    let unconfirmed = tokio::process::Command::new(env!("CARGO_BIN_EXE_staging-fixtures"))
        .args(provision_arguments(OPERATOR, "5", ADDRESS, &root))
        .output()
        .await
        .expect("run staging-fixtures");
    assert_eq!(unconfirmed.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&unconfirmed.stderr);
    assert!(
        stderr.contains("--confirm-database is required"),
        "{stderr}"
    );
    assert!(fixture.fleet_servers().await.is_empty());
}

#[tokio::test]
async fn staging_fixtures_rotate_credential_stages_then_promotes_a_new_secret() {
    let fixture = fixture().await;
    assert_eq!(fixture.provision(&["--apply"]).await.code, 0);
    let live = fixture.secret_file(1, "host-agent-credential");
    let staged = fixture.secret_file(1, "host-agent-credential.staged");
    let old_secret = fs::read_to_string(&live).expect("live secret");
    let old_secret = old_secret.trim();
    let (old_id, server, _) = authenticated(&fixture.pool, old_secret).await;
    let mark = fixture.new_audits().await.len();

    let dry = fixture
        .rotate("1", "host_agent", &["--stage", "--actor", OPERATOR])
        .await;
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(
        dry.stdout
            .contains(&format!("revoke-after-staging credential id={old_id}"))
    );
    assert!(!staged.exists(), "a dry run staged a file");
    assert_eq!(fixture.fleet_credentials().await.len(), 10);

    let stage = fixture
        .rotate(
            "1",
            "host_agent",
            &["--stage", "--actor", OPERATOR, "--apply"],
        )
        .await;
    assert_eq!(stage.code, 0, "{}", stage.stderr);
    assert_eq!(mode(&staged), 0o600);
    let new_secret = fs::read_to_string(&staged).expect("staged secret");
    let (new_id, new_server, executor) = authenticated(&fixture.pool, &new_secret).await;
    assert_eq!((new_server, executor), (server, ExecutorKind::HostAgent));
    assert!(stage.stdout.contains(&format!(
        "id={new_id} file={} state=staged",
        staged.display()
    )));
    let credentials = fixture.fleet_credentials().await;
    let label = &credentials
        .iter()
        .find(|row| row.0 == new_id)
        .expect("row")
        .3;
    assert!(
        label.starts_with("TBD Staging 1 host agent rotation "),
        "{label}"
    );
    let audits = fixture.new_audits().await;
    assert_eq!(audits.len(), mark + 1);
    let last = audits.last().expect("issue audit");
    assert_eq!(
        (last.0.as_str(), last.3.as_str()),
        ("server.credential_issued", new_id.to_string().as_str())
    );
    assert_eq!(
        fs::read_to_string(&live).expect("live"),
        old_secret,
        "staging changed the live file"
    );

    let preview = fixture.rotate("1", "host_agent", &["--promote"]).await;
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    assert!(
        preview
            .stdout
            .contains(&format!("still-active credential id={old_id}"))
    );
    assert!(staged.exists(), "a dry run promoted the file");

    let mut transaction = fixture.pool.begin().await.expect("begin");
    revoke_machine_credential(&mut transaction, server, old_id, OPERATOR, "rotation test")
        .await
        .expect("revoke the old credential");
    transaction.commit().await.expect("commit");
    let promote = fixture
        .rotate("1", "host_agent", &["--promote", "--apply"])
        .await;
    assert_eq!(promote.code, 0, "{}", promote.stderr);
    assert!(
        !promote.stdout.contains("still-active"),
        "{}",
        promote.stdout
    );
    assert!(promote.stdout.contains(&format!(
        "id={new_id} file={} state=promoted",
        live.display()
    )));
    assert!(!staged.exists(), "the staged file stayed");
    assert_eq!(mode(&live), 0o600);
    assert_eq!(fs::read_to_string(&live).expect("live"), new_secret);
    let old = authenticate_machine(&fixture.pool, old_secret)
        .await
        .expect_err("revoked");
    assert_eq!(old.message, "machine credential revoked");

    let runtime = fixture
        .rotate(
            "2",
            "mod_runtime",
            &["--stage", "--actor", OPERATOR, "--apply"],
        )
        .await;
    assert_eq!(runtime.code, 0, "{}", runtime.stderr);
    let promoted = fixture
        .rotate("2", "mod_runtime", &["--promote", "--apply"])
        .await;
    assert_eq!(promoted.code, 0, "{}", promoted.stderr);
    let secret = fs::read_to_string(fixture.secret_file(2, "mod-runtime-credential")).unwrap();
    let (_, _, kind) = authenticated(&fixture.pool, &secret).await;
    assert_eq!(kind, ExecutorKind::ModRuntime);
}

#[tokio::test]
async fn staging_fixtures_rotate_credential_refuses_what_it_cannot_do_safely() {
    let fixture = fixture().await;
    let early = fixture
        .rotate(
            "1",
            "host_agent",
            &["--stage", "--actor", OPERATOR, "--apply"],
        )
        .await;
    assert_eq!(early.code, 2);
    assert!(
        early
            .stderr
            .contains("no server is named \"TBD Staging 1\""),
        "{}",
        early.stderr
    );

    assert_eq!(fixture.provision(&["--apply"]).await.code, 0);
    let credentials = fixture.fleet_credentials().await.len();
    let usage: [(&[&str], &str); 4] = [
        (&["--promote"], "no staged credential"),
        (&["--stage"], "--actor is required"),
        (
            &["--stage", "--promote", "--actor", OPERATOR],
            "exactly one of --stage and --promote",
        ),
        (&["--promote", "--actor", OPERATOR], "takes no --actor"),
    ];
    for (step, expected) in usage {
        let run = fixture.rotate("1", "host_agent", step).await;
        assert_eq!(run.code, 2, "{step:?}: {}", run.stderr);
        assert!(run.stderr.contains(expected), "{step:?}: {}", run.stderr);
    }

    let staged = fixture.secret_file(1, "host-agent-credential.staged");
    let stage = fixture
        .rotate(
            "1",
            "host_agent",
            &["--stage", "--actor", OPERATOR, "--apply"],
        )
        .await;
    assert_eq!(stage.code, 0, "{}", stage.stderr);
    let staged_secret = fs::read_to_string(&staged).expect("staged");
    let twice = fixture
        .rotate(
            "1",
            "host_agent",
            &["--stage", "--actor", OPERATOR, "--apply"],
        )
        .await;
    assert_eq!(twice.code, 2);
    assert!(twice.stderr.contains("already exists"), "{}", twice.stderr);
    assert_eq!(fs::read_to_string(&staged).expect("staged"), staged_secret);
    assert_eq!(fixture.fleet_credentials().await.len(), credentials + 1);

    let (staged_id, server, _) = authenticated(&fixture.pool, &staged_secret).await;
    let mut transaction = fixture.pool.begin().await.expect("begin");
    revoke_machine_credential(
        &mut transaction,
        server,
        staged_id,
        OPERATOR,
        "refusal test",
    )
    .await
    .expect("revoke the staged credential");
    transaction.commit().await.expect("commit");
    let live = fixture.secret_file(1, "host-agent-credential");
    let live_secret = fs::read_to_string(&live).expect("live");
    let revoked = fixture
        .rotate("1", "host_agent", &["--promote", "--apply"])
        .await;
    assert_eq!(revoked.code, 2);
    assert!(revoked.stderr.contains("is revoked"), "{}", revoked.stderr);
    assert_eq!(fs::read_to_string(&live).expect("live"), live_secret);

    let foreign = fixture.secret_file(1, "mod-runtime-credential.staged");
    fs::copy(&live, &foreign).expect("a host agent secret staged for the mod runtime");
    let crossed = fixture
        .rotate("1", "mod_runtime", &["--promote", "--apply"])
        .await;
    assert_eq!(crossed.code, 2);
    assert!(
        crossed.stderr.contains("another server or executor"),
        "{}",
        crossed.stderr
    );
    assert!(foreign.exists(), "a refused promotion moved the file");
    assert_eq!(fixture.fleet_credentials().await.len(), credentials + 1);
}
