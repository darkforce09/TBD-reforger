use super::*;

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
            match verify_dump(&file, min_rows, expect) {
                Ok(rows) => {
                    print!("{rows}");
                    Ok(0)
                }
                Err(VerifyFail) => Ok(1),
            }
        }
        DeployDbCmd::Backup { args } => crate::commands::deploy::database_backup::run(&args),
        DeployDbCmd::Ct { args } => ct_exec(false, &args),
        DeployDbCmd::CtI { args } => ct_exec(true, &args),
        DeployDbCmd::Restore(args) => crate::commands::deploy::database_restore::run(args),
        DeployDbCmd::Drill { args } => crate::commands::deploy::database_restore_drill::run(&args),
    }
}

pub(crate) fn die(msg: &str) -> ! {
    eprintln!("FATAL: {msg}");
    std::process::exit(1);
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

/// Resolved container runtime argv prefix (`podman`, `docker`, or `distrobox-host-exec …`).
pub(crate) fn resolve_runtime() -> Vec<String> {
    if let Ok(override_rt) = env::var("TBD_CONTAINER_RUNTIME") {
        if override_rt.trim().is_empty() {
            // fall through to discovery
        } else {
            let parts: Vec<String> = override_rt.split_whitespace().map(str::to_string).collect();
            let head = parts.first().map(String::as_str).unwrap_or("");
            if which(head).is_none() {
                die(&format!(
                    "TBD_CONTAINER_RUNTIME='{override_rt}' but '{head}' is not executable."
                ));
            }
            return parts;
        }
    }
    if which("podman").is_some() {
        return vec!["podman".into()];
    }
    if which("docker").is_some() {
        return vec!["docker".into()];
    }
    if which("distrobox-host-exec").is_some() {
        if host_exec_has("podman") {
            return vec!["distrobox-host-exec".into(), "podman".into()];
        }
        if host_exec_has("docker") {
            return vec!["distrobox-host-exec".into(), "docker".into()];
        }
    }
    die(
        "no container runtime. Tried: $TBD_CONTAINER_RUNTIME, podman, docker, distrobox-host-exec {podman,docker}.\n\
       pg_dump does not exist on this host either, so there is no fallback path.\n\
       Refusing to report a successful backup from a tool that cannot run.",
    );
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
    Command::new("distrobox-host-exec")
        .args(["command", "-v", tool])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn require_container() -> Result<()> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..])
        .args(["inspect", "-f", "{{.State.Running}}", &container]);
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn '{}' for inspect", runtime.join(" ")))?;
    let out = String::from_utf8_lossy(&output.stdout);
    let out_trim = out.trim();
    let err = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let combined = if err.trim().is_empty() {
            out_trim.to_string()
        } else {
            err.trim().to_string()
        };
        die(&format!(
            "container '{container}' not found by '{}' (rc={}).\n\
       {combined}\n\
       Start it with: cargo xtask db up",
            runtime.join(" "),
            output.status.code().unwrap_or(1)
        ));
    }
    if out_trim != "true" {
        die(&format!(
            "container '{container}' exists but is not running (State.Running={out_trim}).\n\
       A backup taken against a stopped database is not a backup. Start it: cargo xtask db up"
        ));
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
        die(&format!(
            "'{tool}' is ABSENT inside container '{container}' (rc={rc}).\n\
       This is not a failed backup, it is a backup that never ran. Refusing to report success.\n\
       Expected a postgres image that ships {tool} (postgres:18-alpine has it at /usr/local/bin/{tool})."
        ));
    }
    Ok(path)
}

/// Inherit stdio `exec` (for `ct` / `ct-i` CLI and binary dump streams).
pub(crate) fn ct_exec(interactive: bool, args: &[String]) -> Result<u8> {
    let _ = resolve_runtime(); // die early with the same message if absent
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..]).arg("exec");
    if interactive {
        cmd.arg("-i");
    }
    cmd.arg(&container).args(args);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn '{}' exec {}", runtime.join(" "), container))?;
    Ok(status.code().unwrap_or(1) as u8)
}

/// Capture stdout/stderr from a non-interactive container exec (tools that need parsing).
pub(crate) fn ct_capture(
    interactive_stdin: bool,
    args: &[String],
) -> Result<(i32, String, String)> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..]).arg("exec");
    if interactive_stdin {
        cmd.arg("-i");
    }
    cmd.arg(&container).args(args);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn '{}' exec {}", runtime.join(" "), container))?;
    Ok((
        output.status.code().unwrap_or(1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// `exec -i` with binary stdin from `input`, capturing stdout + stderr separately.
pub(crate) fn ct_i_stdin_capture(args: &[String], input: &[u8]) -> Result<(i32, Vec<u8>, String)> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..])
        .arg("exec")
        .arg("-i")
        .arg(&container)
        .args(args);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().with_context(|| {
        format!(
            "failed to spawn '{}' exec -i {}",
            runtime.join(" "),
            container
        )
    })?;
    {
        let mut sink = child.stdin.take().context("child stdin")?;
        // Closed stdin mid-write is the child's business (matches bash pipe behaviour).
        let _ = sink.write_all(input);
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr)?;
    }
    let status = child.wait()?;
    Ok((
        status.code().unwrap_or(1),
        stdout,
        String::from_utf8_lossy(&stderr).into_owned(),
    ))
}

/// `exec -i` with stdout/stderr redirected to files (binary-safe for `pg_dump -Fc`).
pub(crate) fn ct_i_to_files(
    args: &[String],
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<i32> {
    let runtime = resolve_runtime();
    let container = db_container();
    let stdout_file = fs::File::create(stdout_path)
        .with_context(|| format!("cannot create dump stdout file '{}'", stdout_path.display()))?;
    let stderr_file = fs::File::create(stderr_path)
        .with_context(|| format!("cannot create dump stderr file '{}'", stderr_path.display()))?;
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..])
        .arg("exec")
        .arg("-i")
        .arg(&container)
        .args(args);
    // stdin null: pg_dump does not read stdin; matches a non-interactive pipe source.
    cmd.stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file);
    let status = cmd.status().with_context(|| {
        format!(
            "failed to spawn '{}' exec -i {}",
            runtime.join(" "),
            container
        )
    })?;
    Ok(status.code().unwrap_or(1))
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
        die("restore target database name is empty or unparseable.\n\
       Expected a single ASCII name, e.g. --db rust_it");
    }
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        die(&format!(
            "restore target '{name}' is not a plain ASCII database name ([A-Za-z0-9_])."
        ));
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
    eprint!(
        "\
───────────────────────────────────────────────────────────────────────
REFUSING to restore into database `{name}` (outside the restore allow-list).

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe

  The live database `tbd_reforger` is never allowed by default — a
  `pg_restore --clean --if-exists` against it DROPS EVERY OBJECT FIRST,
  so a typo here is unrecoverable without another backup.

  This is the same allow-list the integration harness carries at
  apps/website/api_v2/tests/common/mod.rs:87, which already stopped one
  exported TEST_DATABASE_URL from wiping the live database.

  If you genuinely mean it (disaster recovery), name it twice:
    cargo xtask deploy db restore --db {name} --i-understand-this-destroys={name} <dump>
───────────────────────────────────────────────────────────────────────
"
    );
    std::process::exit(1);
}
