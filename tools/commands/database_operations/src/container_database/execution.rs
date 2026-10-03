//! The container layer's verbs and helpers: runtime, exec, capture and the scratch allow-list.
//!
//! **Role:** the `deploy db` verb dispatch and the helpers behind it: the container runtime
//! resolution, the container and pg-tool presence checks, exec and capture inside the database
//! container, and the scratch and restore-target allow-lists.
//! **Position:** a child of [`crate::container_database`], which re-exports its items to the
//! backup, restore, drill and local database modules; the deployment crate reaches [`run`] through
//! `deploy db`.
//! **Signals & state:** none; every call resolves the runtime and spawns its child afresh.
//! **Invariants:** a missing runtime, container or tool is an operator stop, never a reported
//! success; `tbd_reforger` is never a scratch name and is a restore target only with the doubled
//! confirmation.

use super::*;

/// Runs one `cargo xtask deploy db` verb and returns its exit code.
pub fn run(cmd: DeployDbCmd) -> Result<u8> {
    match cmd {
        DeployDbCmd::RefuseUnsafe { db, confirm } => {
            refuse_unsafe_restore_target(&db, confirm.as_deref())?;
            Ok(0)
        }
        DeployDbCmd::DatabaseNameFromUrl { url } => match database_name_from_url(&url) {
            Some(name) => {
                print!("{name}");
                Ok(0)
            }
            None => Ok(1),
        },
        DeployDbCmd::IsSafeScratch { db } => {
            if is_safe_scratch_database_name(&db) {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        DeployDbCmd::RequireContainer => {
            require_container()?;
            Ok(0)
        }
        DeployDbCmd::RequirePgTool { tool } => {
            let path = require_pg_tool(&tool)?;
            print!("{path}");
            Ok(0)
        }
        DeployDbCmd::DatabaseExists { db } => {
            if database_exists(&db)? {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        DeployDbCmd::CountRows { db } => {
            let rows = count_db_rows(&db)?;
            print!("{rows}");
            Ok(0)
        }
        DeployDbCmd::VerifyDump {
            file,
            min_rows,
            expect_db,
        } => {
            let expect = if expect_db.is_empty() {
                None
            } else {
                Some(expect_db.as_str())
            };
            match verify_dump(&file, min_rows, expect)? {
                Ok(rows) => {
                    print!("{rows}");
                    Ok(0)
                }
                Err(VerifyFail) => Ok(1),
            }
        }
        DeployDbCmd::Backup { args } => crate::backup::run(&args),
        DeployDbCmd::Ct { args } => ct_exec(false, &args),
        DeployDbCmd::CtI { args } => ct_exec(true, &args),
        DeployDbCmd::Restore(args) => crate::restore::run(args),
        DeployDbCmd::Drill { args } => crate::restore_drill::run(&args),
    }
}

pub(crate) fn warn(msg: &str) {
    eprintln!("WARN: {msg}");
}

pub(crate) fn info(msg: &str) {
    println!("==> {msg}");
}

pub(crate) fn db_container() -> String {
    env::var("TBD_DB_CONTAINER").unwrap_or_else(|_| "tbd_reforger_db".into())
}

pub(crate) fn db_user() -> String {
    env::var("TBD_DB_USER").unwrap_or_else(|_| "tbd".into())
}

/// Resolved container runtime argv prefix (`podman`, `docker`, or `distrobox-host-exec …`); an
/// operator stop when none is usable.
pub(crate) fn resolve_runtime() -> Result<Vec<String>> {
    if let Ok(override_rt) = env::var("TBD_CONTAINER_RUNTIME") {
        if override_rt.trim().is_empty() {
            // fall through to discovery
        } else {
            let parts: Vec<String> = override_rt.split_whitespace().map(str::to_string).collect();
            let head = parts.first().map(String::as_str).unwrap_or("");
            if which(head).is_none() {
                stop!("TBD_CONTAINER_RUNTIME='{override_rt}' but '{head}' is not executable.");
            }
            return Ok(parts);
        }
    }
    if which("podman").is_some() {
        return Ok(vec!["podman".into()]);
    }
    if which("docker").is_some() {
        return Ok(vec!["docker".into()]);
    }
    if which("distrobox-host-exec").is_some() {
        if host_exec_has("podman") {
            return Ok(vec!["distrobox-host-exec".into(), "podman".into()]);
        }
        if host_exec_has("docker") {
            return Ok(vec!["distrobox-host-exec".into(), "docker".into()]);
        }
    }
    Err(Error::fatal(
        "no container runtime. Tried: $TBD_CONTAINER_RUNTIME, podman, docker, distrobox-host-exec {podman,docker}.\n\
       pg_dump does not exist on this host either, so there is no fallback path.\n\
       Refusing to report a successful backup from a tool that cannot run.",
    ))
}

pub(super) fn which(prog: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let p = dir.join(prog);
            if p.is_file() { Some(p) } else { None }
        })
    })
}

pub(super) fn host_exec_has(tool: &str) -> bool {
    // Both streams are drained and dropped: only the exit status answers.
    Run::new("distrobox-host-exec")
        .args(["command", "-v", tool])
        .status()
        .is_ok_and(|code| code == 0)
}

pub(crate) fn require_container() -> Result<()> {
    let runtime = resolve_runtime()?;
    let container = db_container();
    let output = Run::new(&runtime[0])
        .args(&runtime[1..])
        .args(["inspect", "-f", "{{.State.Running}}", &container])
        .output()
        .with_context(|| format!("failed to spawn '{}' for inspect", runtime.join(" ")))?;
    let out_trim = output.stdout.trim();
    let err = &output.stderr;
    if output.code != 0 {
        let combined = if err.trim().is_empty() {
            out_trim.to_string()
        } else {
            err.trim().to_string()
        };
        stop!(
            "container '{container}' not found by '{}' (rc={}).\n\
       {combined}\n\
       Start it with: cargo xtask db up",
            runtime.join(" "),
            output.code
        );
    }
    if out_trim != "true" {
        stop!(
            "container '{container}' exists but is not running (State.Running={out_trim}).\n\
       A backup taken against a stopped database is not a backup. Start it: cargo xtask db up"
        );
    }
    Ok(())
}

pub(crate) fn require_pg_tool(tool: &str) -> Result<String> {
    // bash: path="$(tbd_ct sh -c "command -v $tool" 2>/dev/null)"
    let (rc, stdout, _stderr) = ct_capture(
        false,
        &["sh".into(), "-c".into(), format!("command -v {tool}")],
    )?;
    let path = stdout.trim().to_string();
    if rc != 0 || path.is_empty() {
        let container = db_container();
        stop!(
            "'{tool}' is ABSENT inside container '{container}' (rc={rc}).\n\
       This is not a failed backup, it is a backup that never ran. Refusing to report success.\n\
       Expected a postgres image that ships {tool} (postgres:18-alpine has it at /usr/local/bin/{tool})."
        );
    }
    Ok(path)
}

/// Inherit stdio `exec` (for `ct` / `ct-i` CLI and binary dump streams).
pub(crate) fn ct_exec(interactive: bool, args: &[String]) -> Result<u8> {
    let _ = resolve_runtime()?; // stop early with the same message if absent
    let runtime = resolve_runtime()?;
    let container = db_container();
    // The child shares this terminal: stdin, stdout and stderr inherited, a TTY for `ct-i`.
    let mut child = Run::new(&runtime[0]).args(&runtime[1..]).arg("exec");
    if interactive {
        child = child.arg("-i");
    }
    let code = match child.arg(&container).args(args).terminal() {
        Ok(code) => code,
        // A signal is exit 1, as `ExitStatus::code` reads it.
        Err(verification_core::NotRun::Signalled { .. }) => 1,
        Err(cause) => {
            return Err(cause).with_context(|| {
                format!("failed to spawn '{}' exec {}", runtime.join(" "), container)
            });
        }
    };
    Ok(code as u8)
}

/// Capture stdout/stderr from a non-interactive container exec (tools that need parsing).
pub(crate) fn ct_capture(
    interactive_stdin: bool,
    args: &[String],
) -> Result<(i32, String, String)> {
    let runtime = resolve_runtime()?;
    let container = db_container();
    let mut run = Run::new(&runtime[0]).args(&runtime[1..]).arg("exec");
    if interactive_stdin {
        run = run.arg("-i");
    }
    let output =
        run.arg(&container).args(args).output().with_context(|| {
            format!("failed to spawn '{}' exec {}", runtime.join(" "), container)
        })?;
    Ok((output.code, output.stdout, output.stderr))
}

/// `exec -i` with binary stdin from `input`, capturing stdout + stderr separately.
pub(crate) fn ct_i_stdin_capture(args: &[String], input: &[u8]) -> Result<(i32, Vec<u8>, String)> {
    let runtime = resolve_runtime()?;
    let container = db_container();
    // A binary archive goes in on stdin and binary bytes come back on stdout.
    let outcome = Run::new(&runtime[0])
        .args(&runtime[1..])
        .arg("exec")
        .arg("-i")
        .arg(&container)
        .args(args)
        .stdin_bytes(input)
        .binary_output();
    let output = match outcome {
        Ok(output) => output,
        // A signal is exit 1 with nothing captured, as `ExitStatus::code` reads it.
        Err(verification_core::NotRun::Signalled { .. }) => {
            return Ok((1, Vec::new(), String::new()));
        }
        Err(cause) => {
            return Err(cause).with_context(|| {
                format!(
                    "failed to spawn '{}' exec -i {}",
                    runtime.join(" "),
                    container
                )
            });
        }
    };
    Ok((output.code, output.stdout, output.stderr))
}

/// `exec -i` with stdout/stderr redirected to files (binary-safe for `pg_dump -Fc`).
pub(crate) fn ct_i_to_files(
    args: &[String],
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<i32> {
    let runtime = resolve_runtime()?;
    let container = db_container();
    let stdout_file = fs::File::create(stdout_path)
        .with_context(|| format!("cannot create dump stdout file '{}'", stdout_path.display()))?;
    let stderr_file = fs::File::create(stderr_path)
        .with_context(|| format!("cannot create dump stderr file '{}'", stderr_path.display()))?;
    // The child's stdout and stderr go straight to the files (a binary `pg_dump -Fc` stream);
    // stdin is null: pg_dump does not read stdin, as with a non-interactive pipe source.
    let outcome = Run::new(&runtime[0])
        .args(&runtime[1..])
        .arg("exec")
        .arg("-i")
        .arg(&container)
        .args(args)
        .output_to_files(stdout_file, stderr_file);
    match outcome {
        Ok(code) => Ok(code),
        // A signal is exit 1, as `ExitStatus::code` reads it.
        Err(verification_core::NotRun::Signalled { .. }) => Ok(1),
        Err(cause) => Err(cause).with_context(|| {
            format!(
                "failed to spawn '{}' exec -i {}",
                runtime.join(" "),
                container
            )
        }),
    }
}

/// `postgres://…/rust_it?sslmode=disable` → `Some("rust_it")`. Empty / multi-segment / non-ASCII → None.
pub fn database_name_from_url(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let after_auth = rest.split_once('/')?.1;
    let name = after_auth
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_string();
    if name.is_empty() || name.contains('/') {
        return None;
    }
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(name)
}

/// Whether `name` is on the scratch allow-list (`rust_it`, `tbd_gate*`, `*_cold`, `*_it`,
/// `*_probe`); the live `tbd_reforger` never is.
pub fn is_safe_scratch_database_name(name: &str) -> bool {
    if name.is_empty() || name == "tbd_reforger" {
        return false;
    }
    name == "rust_it"
        || name.starts_with("tbd_gate")
        || name.ends_with("_cold")
        || name.ends_with("_it")
        || name.ends_with("_probe")
}

pub(crate) fn refuse_unsafe_restore_target(name: &str, confirm: Option<&str>) -> Result<()> {
    if name.is_empty() {
        stop!(
            "restore target database name is empty or unparseable.\n\
       Expected a single ASCII name, e.g. --db rust_it"
        );
    }
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        stop!("restore target '{name}' is not a plain ASCII database name ([A-Za-z0-9_]).");
    }
    if is_safe_scratch_database_name(name) {
        return Ok(());
    }
    if let Some(c) = confirm
        && !c.is_empty()
        && c == name
    {
        warn(&format!(
            "restoring over NON-scratch database '{name}' — confirmed via --i-understand-this-destroys={name}"
        ));
        return Ok(());
    }
    Err(Error::stop(format!(
        "\
───────────────────────────────────────────────────────────────────────
REFUSING to restore into database `{name}` (outside the restore allow-list).

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe

  The live database `tbd_reforger` is never allowed by default — a
  `pg_restore --clean --if-exists` against it DROPS EVERY OBJECT FIRST,
  so a typo here is unrecoverable without another backup.

  This is the same allow-list the integration harness carries at
  apps/api/tests/common/mod.rs:87, which already stopped one
  exported TEST_DATABASE_URL from wiping the live database.

  If you genuinely mean it (disaster recovery), name it twice:
    cargo xtask deploy db restore --db {name} --i-understand-this-destroys={name} <dump>
───────────────────────────────────────────────────────────────────────
"
    )))
}
