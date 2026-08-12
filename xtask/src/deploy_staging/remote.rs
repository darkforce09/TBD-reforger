//! ssh / rsync / compose / systemd transport and the deploy pipeline (bash lines 1524–1889).
//!
//! ── NOTHING IN THIS FILE HAS BEEN EXECUTED ───────────────────────────────────────────────────
//!
//! `scripts/deploy/deploy.env` is absent on every machine this port was written on — it is
//! gitignored AND rsync-excluded by design, so the credential exists only on a developer's PC.
//! Every function below that spawns `ssh`, `rsync`, `docker compose`, `systemctl` or `curl` is
//! therefore **structurally faithful and live-unverified**. What IS verified:
//!
//! * the exact program + argument vector, in order, for every spawn — [`tests`];
//! * the exact stdin payload for every `ssh … bash -s` heredoc — [`tests`];
//! * the four-outcome exit-code read of `mod remote-logs` — [`v6_verdict`] + [`tests`];
//! * the whole `--dry-run` walk, which is byte-diffed against the bash baseline and never opens a
//!   socket (see the note on [`Runner`]).
//!
//! What is NOT verified: whether a real `ssh` accepts these argv, whether the remote `bash -s`
//! payloads behave on the host, whether `docker compose` is reachable there, and whether the boot
//! wait loop's timing assumptions hold. Those were never true of the bash either — the bash was
//! only ever exercised by the operator running a live deploy — so this is a statement about the
//! test environment, not a regression.
//!
//! ── THE EXCLUDE LIST IS A LICENCE BOUNDARY, NOT AN OPTIMISATION ──────────────────────────────
//!
//! T-181.52: EXCLUDE EVERY ORACLE LANE, not just CRF. These are read-only reference trees; the
//! server only ever runs `apps/mod/tbd-framework` (see the addon symlink), so shipping them is
//! pure licence exposure for zero benefit. `crf_framework` was already excluded, but
//! `vanilla_reference` and `playable_selector` were NOT — and in the MAIN checkout (which is what
//! deploys) they are real directories, not the worktree symlinks, so ~30 MB of carved Bohemia game
//! source was being rsynced to staging on every deploy. `playable_selector` has NO LICENCE AT ALL,
//! so copying it to a server is redistribution we have no permission for. Anyone adding a fourth
//! oracle lane adds it here too.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use tbd_gate::proc::{self, Run};
use tbd_gate::verdict::NotRun;

use super::agent::{self, AgentEnv};
use super::boot::{self, Out};
use super::config::Env;
use super::payloads::{AGENT_INSTALL_PAYLOAD, profile_payload, smoke_payload, unit_payload};
use super::{Cli, Paths};

/// How `ssh` is invoked: plain, via `sshpass`, or with an identity file.
///
/// ODDITY PRESERVED: the precedence is `TBD_SSH_PASS` first, `TBD_SSH_IDENTITY_FILE` second — a
/// deploy.env holding both silently ignores the key. And in the `rsync -e` string the password is
/// interpolated UNQUOTED, so a password containing a space would split into extra argv words for
/// the inner ssh. Reproduced rather than fixed: the fix (quoting) changes what a working
/// configuration does, and no configuration with a spaced password can currently be working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshBase {
    Plain,
    Pass(String),
    Identity(String),
}

impl SshBase {
    pub fn from_env(env: &Env) -> SshBase {
        if let Some(p) = env.ssh_pass.as_deref() {
            SshBase::Pass(p.to_string())
        } else if let Some(i) = env.ssh_identity_file.as_deref() {
            SshBase::Identity(i.to_string())
        } else {
            SshBase::Plain
        }
    }

    /// `SSH_BASE=(...)` — the program and its leading arguments.
    pub fn program_args(&self) -> (String, Vec<String>) {
        match self {
            SshBase::Plain => (
                "ssh".into(),
                vec!["-o".into(), "StrictHostKeyChecking=no".into()],
            ),
            SshBase::Pass(p) => (
                "sshpass".into(),
                vec![
                    "-p".into(),
                    p.clone(),
                    "ssh".into(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            ),
            SshBase::Identity(i) => (
                "ssh".into(),
                vec![
                    "-i".into(),
                    i.clone(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            ),
        }
    }

    /// The `rsync -e <string>` transport. One shell word per space, unquoted, as in the bash.
    pub fn rsync_e(&self) -> String {
        match self {
            SshBase::Plain => "ssh -o StrictHostKeyChecking=no".into(),
            SshBase::Pass(p) => format!("sshpass -p {p} ssh -o StrictHostKeyChecking=no"),
            SshBase::Identity(i) => format!("ssh -i {i} -o StrictHostKeyChecking=no"),
        }
    }
}

/// The bash `run()` wrapper: echo under `--dry-run`, execute otherwise.
///
/// LATENT FINDING, reported not fixed: in the shell script this dry-run branch was **dead code**.
/// Every call site of `ssh_cmd` / `rsync_to_remote` already sat inside an explicit
/// `if [ "$DRY_RUN" -eq 1 ]` arm that printed a bespoke `[dry-run] …` line and skipped the call, so
/// `run`'s generic `echo "[dry-run] $*"` could never fire. It is kept — with the same shape — so
/// that a call site added later without its own guard still cannot reach the network. Removing it
/// would convert a latent no-op into a live `ssh`.
pub struct Runner {
    dry_run: bool,
}

impl Runner {
    /// `ssh_cmd` — `run "${SSH_BASE[@]}" "$TBD_SSH_HOST" "$@"`, with optional stdin.
    ///
    /// Returns the raw exit status. `NotRun` (ssh absent, killed by a signal, timed out) is mapped
    /// through [`not_run_exit`] so it can never fold into "the step succeeded".
    fn ssh(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
        stdin: Option<String>,
    ) -> Result<i32, u8> {
        let argv = ssh_argv(base, host, remote);
        if self.dry_run {
            println!("[dry-run] {}", argv.join(" "));
            return Ok(0);
        }
        let (program, _) = base.program_args();
        if let Err(e) = proc::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = Run::new(&program);
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        if let Some(body) = stdin {
            run = run.stdin(body);
        }
        // `merged_output` is real `2>&1`: the bash let both streams reach the terminal, and a
        // remote step's diagnosis is usually split across them.
        match run.timeout(Duration::from_secs(3600)).merged_output() {
            Ok(out) => {
                let _ = io::stdout().write_all(out.text.as_bytes());
                let _ = io::stdout().flush();
                Ok(out.code)
            }
            Err(e) => Err(not_run_exit(&e)),
        }
    }

    /// `ssh_cmd` where a non-zero status must abort the deploy, as `set -e` did.
    fn ssh_ok(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
        stdin: Option<String>,
    ) -> Result<(), u8> {
        match self.ssh(base, host, remote, stdin)? {
            0 => Ok(()),
            code => Err(code as u8),
        }
    }

    /// `ssh_cmd "…"` capturing stdout, for the boot-verify probes.
    fn ssh_capture(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
    ) -> Result<(i32, String), u8> {
        let argv = ssh_argv(base, host, remote);
        if self.dry_run {
            println!("[dry-run] {}", argv.join(" "));
            return Ok((0, String::new()));
        }
        let (program, _) = base.program_args();
        if let Err(e) = proc::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = Run::new(&program);
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        match run.timeout(Duration::from_secs(600)).output() {
            Ok(out) => Ok((out.code, out.stdout)),
            Err(e) => Err(not_run_exit(&e)),
        }
    }
}

/// The full `ssh` argv, program included at index 0. Pure, so it can be asserted without spawning.
pub fn ssh_argv(base: &SshBase, host: &str, remote: &[String]) -> Vec<String> {
    let (program, mut args) = base.program_args();
    let mut argv = vec![program];
    argv.append(&mut args);
    argv.push(host.to_string());
    argv.extend(remote.iter().cloned());
    argv
}

/// The full `rsync` argv. The exclude list is the licence boundary described in the module header;
/// the ORDER is the bash's, because a wave log diff should not show reordered flags.
pub fn rsync_argv(base: &SshBase, mono_root: &Path, host: &str, remote_dir: &str) -> Vec<String> {
    vec![
        "rsync".into(),
        "-e".into(),
        base.rsync_e(),
        "-avz".into(),
        "--delete".into(),
        "--exclude=.git/".into(),
        "--exclude=apps/mod/crf_framework/".into(),
        "--exclude=apps/mod/vanilla_reference/".into(),
        "--exclude=apps/mod/playable_selector/".into(),
        "--exclude=apps/mod/Tbd_framework/".into(),
        "--exclude=apps/mod/.local-test-profile/".into(),
        "--exclude=**/node_modules/".into(),
        "--exclude=apps/website/api/.tools/".into(),
        "--exclude=apps/website/api/.env".into(),
        "--exclude=apps/mod/tbd-framework/Scripts/WorkbenchGame/".into(),
        "--exclude=scripts/deploy/deploy.env".into(),
        format!("{}/", mono_root.display()),
        format!("{host}:{remote_dir}/"),
    ]
}

/// `ExecStart` per mode.
///
/// `-config` is mutually exclusive with **`-addons`** — NOT with `-addonsDir`. Those are two
/// different flags and the distinction is the whole of T-604. `-addons <GUID>` asks the engine to
/// activate a mod id and is refused alongside `-config` ("-config cannot be used together with
/// addons!"); `-addonsDir <dir>` only tells it where to LOOK, and combines with `-config` fine.
///
/// config mode therefore carries BOTH, which is what makes it simultaneously joinable and honest:
/// `-config` registers the backend room and supplies `game.admins[]`, `-addonsDir` makes the
/// checkout this deploy just rsynced the copy that actually loads. Without `-addonsDir` the engine
/// satisfies `game.mods[]` from the Workshop instead — same GUID, different code — and staging
/// reports on a build it never deployed. `assert_local_addon_won` proves which one won; it is not
/// decoration, it is the acceptance criterion.
///
/// Flag ORDER matches the playtest runner deliberately. The engine does not care, but two launch
/// lines that mean the same thing should read the same, or the next person diffs them and finds a
/// difference that isn't one.
pub fn exec_start(env: &Env) -> String {
    if env.server_mode == "config" {
        format!(
            "{}/ArmaReforgerServer -addonsDir {} -config {} -profile {} -maxFPS 60 -logStats 30000 -nothrow",
            env.server_dir, env.addons_staging, env.server_config_remote, env.profile_dir
        )
    } else {
        format!(
            "{}/ArmaReforgerServer -profile {} -addonsDir {} -addons {} -server \"{}\" -bindIP 0.0.0.0 -bindPort {} -a2sPort {} -maxFPS 60 -logStats 30000 -nothrow",
            env.server_dir,
            env.profile_dir,
            env.addons_staging,
            env.addon_guid,
            env.scenario,
            env.game_port,
            env.a2s_port
        )
    }
}

fn not_run_exit(e: &NotRun) -> u8 {
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

/// T-607: READ THE EXIT CODE of `mod remote-logs`, do not just inherit it.
///
/// `remote-log-grep` is a FOUR-outcome check and this script is the consumer that pinned `2`:
///
/// ```text
/// 0 HEALTHY  ·  1 FAIL  ·  2 PARTIAL (booted, nobody joined yet)  ·  3 ENVIRONMENT
/// ```
///
/// This used to be the last statement in the file, so under `set -e` the deploy simply exited with
/// whatever it returned. `2` is the NORMAL state immediately after a deploy — nobody has had time
/// to join — so every healthy deploy reported failure to any caller reading `!= 0`, and the fix
/// people reach for when a green run keeps "failing" is to stop believing the gate. `3` is the
/// opposite hazard and must never be soft: it means no log was examined at all, so it says nothing
/// about the mod and cannot be allowed to read as success.
///
/// The same contract applies to `cargo xtask mcp wb-logs` (T-857) and `cargo xtask mod spawn-verify`
/// (T-873) — both were inverted and passed ONLY on the stale June build. Do not build a staging
/// check on a `!= 0` reading of any of the three.
pub fn v6_verdict(code: i32) -> u8 {
    match code {
        0 => {
            println!(
                "V6 HEALTHY — current build, mission loaded, reached LOBBY, a player was seated."
            );
            0
        }
        2 => {
            println!(
                "V6 PARTIAL — boot is healthy, no player has joined yet. This is the expected result"
            );
            println!("   for a fresh deploy and is NOT a failure.");
            0
        }
        1 => {
            eprintln!(
                "V6 FAIL — a required structural line is missing, or an error class is present."
            );
            1
        }
        3 => {
            eprintln!(
                "V6 ENVIRONMENT — the log could not be obtained, so nothing was examined. This says"
            );
            eprintln!("   NOTHING about the mod, and is not a pass.");
            1
        }
        other => {
            eprintln!(
                "V6 returned an unexpected status {other} — treating as failure rather than guessing."
            );
            1
        }
    }
}

/// The whole deploy, in the bash's order. `Ok(0)` only when every step held.
pub fn deploy(paths: &Paths, cli: &Cli) -> Result<u8> {
    let env = match Env::load(&paths.deploy_env) {
        Ok(e) => e,
        Err(code) => return Ok(code),
    };
    if let Err(code) = env.validate(&paths.mono_root) {
        return Ok(code);
    }
    // --render-only sits HERE, after the env is loaded and validated — see the ordering note in
    // the parent module. It never reaches a socket.
    if let Some(out) = cli.render_only_out.as_deref() {
        return Ok(super::render::render_only(&env, out));
    }

    let base = SshBase::from_env(&env);
    let runner = Runner {
        dry_run: cli.dry_run,
    };
    let host = env.ssh_host.clone();
    let agent_env = AgentEnv::from_env();
    if let Err(code) = agent_env.validate_names() {
        return Ok(code);
    }

    // ── V1 ──────────────────────────────────────────────────────────────────────────────────
    println!("==> V1 validate mission JSON");
    if !cli.dry_run {
        let mission = paths
            .schema
            .join(format!("golden-missions/{}.json", env.mission_id));
        // Spawned as a subprocess rather than called in-process, because that is what the bash
        // did and because the schema validator's exit status is its contract.
        let out = Run::new("cargo")
            .arg("run")
            .arg("-q")
            .arg("-p")
            .arg("xtask")
            .arg("--")
            .arg("schema")
            .arg("validate-file")
            .arg(&mission)
            .cwd(&paths.mono_root)
            .timeout(Duration::from_secs(1800))
            .merged_output();
        match out {
            Ok(o) => {
                let _ = io::stdout().write_all(o.text.as_bytes());
                if o.code != 0 {
                    return Ok(o.code as u8);
                }
            }
            Err(e) => return Ok(not_run_exit(&e)),
        }
    }

    // ── rsync ───────────────────────────────────────────────────────────────────────────────
    println!("==> rsync to {}", env.remote_dir);
    if cli.dry_run {
        println!(
            "[dry-run] rsync -avz --delete ... {host}:{}/",
            env.remote_dir
        );
    } else {
        let argv = rsync_argv(&base, &paths.mono_root, &host, &env.remote_dir);
        if let Err(e) = proc::which("rsync") {
            return Ok(not_run_exit(&e));
        }
        let mut run = Run::new("rsync");
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        match run.timeout(Duration::from_secs(7200)).merged_output() {
            Ok(o) => {
                let _ = io::stdout().write_all(o.text.as_bytes());
                if o.code != 0 {
                    return Ok(o.code as u8);
                }
            }
            Err(e) => return Ok(not_run_exit(&e)),
        }
    }

    // ── remote profile + addon symlink ──────────────────────────────────────────────────────
    println!("==> remote profile + addon symlink");
    if cli.dry_run {
        println!("[dry-run] setup server-profile + patch TBD_BackendConfig.json");
    } else if let Err(code) = runner.ssh_ok(
        &base,
        &host,
        &["bash".to_string(), "-s".to_string()],
        Some(profile_payload(&env)),
    ) {
        return Ok(code);
    }

    // ── docker compose ──────────────────────────────────────────────────────────────────────
    // T-438: the compose file lives at apps/website/docker-compose.staging.yml (T-251), not under
    // apps/website/api/. Match `cargo xtask deploy website`.
    println!("==> docker compose (API + Postgres)");
    if cli.dry_run {
        println!(
            "[dry-run] cd $TBD_REMOTE_DIR && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
        );
    } else if let Err(code) = runner.ssh_ok(
        &base,
        &host,
        &[format!(
            "cd '{}' && docker compose -f apps/website/docker-compose.staging.yml up -d --build",
            env.remote_dir
        )],
        None,
    ) {
        return Ok(code);
    }

    // ── V2–V4 API smoke ─────────────────────────────────────────────────────────────────────
    println!("==> API smoke (V2–V4)");
    if !env.run_t092_smoke {
        println!(
            "[SKIP] V2–V4 API smoke — routes BLOCKED on T-092 (not in current backend; would 404)."
        );
        println!(
            "       Set TBD_RUN_T092_SMOKE=1 to force once T-092 ships. See docs/mod/STAGING-SERVER.md."
        );
    } else if cli.dry_run {
        println!("[dry-run] curl mission + roster + 401 on server localhost");
    } else if let Err(code) = runner.ssh_ok(
        &base,
        &host,
        &["bash".to_string(), "-s".to_string()],
        Some(smoke_payload(&env)),
    ) {
        return Ok(code);
    }

    // ── systemd unit + restart, then ASSERT the boot ────────────────────────────────────────
    let exec = exec_start(&env);
    println!(
        "==> systemd user service + restart game server (mode: {})",
        env.server_mode
    );
    if cli.dry_run {
        println!("[dry-run] mode={}", env.server_mode);
        if env.server_mode == "config" {
            println!(
                "[dry-run] render server config -> {}",
                env.server_config_remote
            );
            println!("[dry-run]   game.mods[] from: {}", env.mod_source_label());
            println!("[dry-run]   preview the exact bytes with: --render-only <path>");
        }
        println!("[dry-run] ExecStart={exec}");
        println!("[dry-run] install tbd-reforger.service and restart");
    } else {
        // In config mode, render the server config JSON LOCALLY, validate it, and only then push
        // it. T-288 split render from push: an invalid or empty mod list now fails here, on the dev
        // machine, instead of landing on the server and failing at boot.
        if env.server_mode == "config" {
            let local =
                std::env::temp_dir().join(format!("tbd-server.config.{}.json", std::process::id()));
            if let Err(code) = super::render::render_server_config(&env, &local) {
                return Ok(code);
            }
            let body = fs::read_to_string(&local).unwrap_or_default();
            let res = runner.ssh_ok(
                &base,
                &host,
                &[format!("cat > '{}'", env.server_config_remote)],
                Some(body),
            );
            let _ = fs::remove_file(&local);
            if let Err(code) = res {
                return Ok(code);
            }
        }
        if let Err(code) = runner.ssh_ok(
            &base,
            &host,
            &["bash".to_string(), "-s".to_string()],
            Some(unit_payload(&env, &exec)),
        ) {
            return Ok(code);
        }
        if let Some(code) = verify_boot_remote(&runner, &base, &host, &env)? {
            return Ok(code);
        }
    }

    // ── T-289: install the host control agent ───────────────────────────────────────────────
    //
    // OFF BY DEFAULT. The render above is proven by --agent-selftest; THIS step is not, because
    // exercising it means mutating the live staging host, which T-289 was not permitted to touch.
    // It also buys nothing until the API side lands — nothing would connect to the socket.
    //
    // The agent is enabled via its SOCKET, never its service: socket activation means the agent
    // process only exists for the lifetime of one connection, so there is no long-lived listener
    // to leak, wedge, or restart.
    println!("==> host control agent (T-289)");
    if !agent_env.install {
        println!("[SKIP] agent install — TBD_INSTALL_AGENT=1 to enable.");
        println!("       Preview the exact bytes with: --render-agent <dir>");
        println!("       Prove the behaviour with:     --agent-selftest <dir>");
    } else if cli.dry_run {
        println!("[dry-run] render agent, scp -> {}", agent_env.remote_path);
        println!("[dry-run]   unit under control: {}", agent_env.unit);
        println!(
            "[dry-run]   socket: $XDG_RUNTIME_DIR/{} (SocketMode=0600)",
            agent_env.socket
        );
        println!("[dry-run] systemctl --user enable --now tbd-reforger-agent.socket");
    } else if let Some(code) = install_agent(&runner, &base, &host, &agent_env)? {
        return Ok(code);
    }

    // ── V6 ──────────────────────────────────────────────────────────────────────────────────
    println!("==> V6 remote log grep");
    if cli.dry_run {
        println!("[dry-run] cargo run -q -p xtask -- mod remote-logs");
        return Ok(0);
    }
    let out = Run::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("mod")
        .arg("remote-logs")
        .cwd(&paths.mono_root)
        .timeout(Duration::from_secs(3600))
        .merged_output();
    let code = match out {
        Ok(o) => {
            let _ = io::stdout().write_all(o.text.as_bytes());
            o.code
        }
        // A `NotRun` here is the ENVIRONMENT outcome by any honest reading: no log was examined.
        // Bash could only see a status, so it fell into the `*)` arm; naming it is strictly better
        // and lands on the same exit code.
        Err(e) => {
            eprintln!("V6 could not run: {e:?}");
            return Ok(1);
        }
    };
    if v6_verdict(code) != 0 {
        return Ok(1);
    }
    println!("==> deploy complete");
    Ok(0)
}

/// Render, validate and push the agent, then enable it and re-read the socket's state.
fn install_agent(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    agent_env: &AgentEnv,
) -> Result<Option<u8>> {
    // Render LOCALLY and validate BEFORE anything is pushed — the T-288 posture: a broken artefact
    // fails here, on the dev machine, not after it has landed on the server.
    let local = std::env::temp_dir().join(format!("tbd-agent.{}", std::process::id()));
    let _ = fs::remove_dir_all(&local);
    if let Err(code) = agent::render_agent_files(agent_env, &local) {
        return Ok(Some(code));
    }
    if let Err(code) = agent::validate_agent_files(agent_env, &local) {
        return Ok(Some(code));
    }
    let files: [(&str, String); 3] = [
        (
            "tbd-reforger-agent.sh",
            format!(
                "mkdir -p \"$HOME/.config/systemd/user\" && cat > '{p}' && chmod 0700 '{p}'",
                p = agent_env.remote_path
            ),
        ),
        (
            "tbd-reforger-agent.socket",
            "cat > \"$HOME/.config/systemd/user/tbd-reforger-agent.socket\"".to_string(),
        ),
        (
            "tbd-reforger-agent@.service",
            "cat > \"$HOME/.config/systemd/user/tbd-reforger-agent@.service\"".to_string(),
        ),
    ];
    for (name, remote) in files {
        let body = fs::read_to_string(local.join(name)).unwrap_or_default();
        if let Err(code) = runner.ssh_ok(base, host, &[remote], Some(body)) {
            let _ = fs::remove_dir_all(&local);
            return Ok(Some(code));
        }
    }
    let _ = fs::remove_dir_all(&local);
    if let Err(code) = runner.ssh_ok(
        base,
        host,
        &["bash".to_string(), "-s".to_string()],
        Some(AGENT_INSTALL_PAYLOAD.to_string()),
    ) {
        return Ok(Some(code));
    }
    Ok(None)
}

/// T-607: assert the boot, do not assume it.
///
/// `systemctl restart` exits 0 over a unit that is already dead — the same defect T-289's agent
/// selftest exists for. And even a genuinely-running server proves nothing about WHICH mod it
/// loaded. Until this block existed the deploy's last word was `sleep 8`, after which it printed a
/// success banner regardless of what the engine did.
///
/// This waits for the engine to get far enough to have decided (addon resolution and room
/// registration both land inside ~20 s of start — measured: config load at +4 s, addons resolved at
/// +7 s, room registered at +14 s on a 2026-08-01 boot), then pulls the log back and runs the same
/// verdict `--verify-boot` runs locally. One implementation, two callers.
fn verify_boot_remote(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
) -> Result<Option<u8>> {
    println!("==> waiting for the engine to reach a verdict");
    let timeout: u64 = env.boot_verify_timeout.trim().parse().unwrap_or(180);
    let mut remote_log = String::new();
    let mut waited: u64 = 0;
    while waited < timeout {
        // `|| true` in the bash: a failed probe leaves the previous value. Reproduced by ignoring
        // the Err and keeping `remote_log` as-is.
        if let Ok((_, out)) = runner.ssh_capture(
            base,
            host,
            &[format!(
                "ls -1d '{}'/logs/logs_* 2>/dev/null | tail -1",
                env.profile_dir
            )],
        ) {
            remote_log = out.trim().to_string();
        }
        if !remote_log.is_empty() {
            let probe = runner.ssh_capture(
                base,
                host,
                &[format!(
                    "grep -qF 'Server registered with address:' '{remote_log}/console.log' 2>/dev/null"
                )],
            );
            if matches!(probe, Ok((0, _))) {
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(10));
        waited += 10;
        println!(
            "    {waited}s — no room registration yet (log: {})",
            if remote_log.is_empty() {
                "none"
            } else {
                &remote_log
            }
        );
    }

    if remote_log.is_empty() {
        eprintln!(
            "FAIL: the server produced no log directory under {}/logs after {waited}s.",
            env.profile_dir
        );
        eprintln!("      The unit may not have started at all. Check:");
        eprintln!("        ssh {host} systemctl --user status tbd-reforger.service");
        return Ok(Some(1));
    }

    let local_log =
        std::env::temp_dir().join(format!("tbd-staging-console.{}.log", std::process::id()));
    // FAIL-OPEN CLOSED (3 of 3). The bash was
    //   ssh_cmd "cat '$log/console.log'" > "$_local_log" 2>/dev/null || true
    // which discarded ssh's status entirely; only the follow-up `[ ! -s ]` caught it, so a
    // TRUNCATED-but-non-empty pull (a dropped connection mid-transfer) read as a complete log and
    // the verdict then ran over a partial file. The status is now checked, and a non-zero ssh is
    // the same refusal as an empty file.
    let pulled = runner.ssh_capture(base, host, &[format!("cat '{remote_log}/console.log'")]);
    let text = match pulled {
        Ok((0, t)) => t,
        _ => String::new(),
    };
    if text.is_empty() {
        eprintln!("FAIL: could not read {remote_log}/console.log off {host}.");
        eprintln!("      Refusing to report the deploy OK over a log this script never examined.");
        return Ok(Some(1));
    }
    if fs::write(&local_log, &text).is_err() {
        eprintln!(
            "FAIL: could not stage the pulled log at {}",
            local_log.display()
        );
        return Ok(Some(1));
    }

    let admin_count = env.admin_count();
    println!("    pulled {remote_log}/console.log ({} bytes)", text.len());

    // Measure the rival ON THE HOST. `TBD_PROFILE_DIR` is a remote path, so the local `[ -f ]`
    // fallback inside the verdict would answer "absent" for a pak that is really sitting there, and
    // downgrade a genuine contest to WEAK EVIDENCE on every deploy.
    let rival = runner
        .ssh_capture(
            base,
            host,
            &[format!(
                "wc -c < '{}/addons/TBDFramework_{}/data.pak' 2>/dev/null || echo 0",
                env.profile_dir, env.addon_guid
            )],
        )
        .map(|(_, s)| s.trim().to_string())
        .unwrap_or_default();
    let rival = if rival.is_empty() {
        "0".to_string()
    } else {
        rival
    };

    let mut out = Out::streams();
    if env.server_mode == "config" {
        let rc = boot::verify_boot_log(
            &mut out,
            &local_log,
            &env.addon_guid,
            &env.addons_staging,
            &admin_count.to_string(),
            &env.profile_dir,
            Some(&rival),
        );
        if rc != 0 {
            eprintln!();
            eprintln!(
                "DEPLOY FAILED ITS OWN ACCEPTANCE CHECK. The files are on the host and the unit may"
            );
            eprintln!(
                "be running, but it is NOT serving what you deployed, or it is not joinable."
            );
            eprintln!("Full log kept at: {}", local_log.display());
            return Ok(Some(1));
        }
    } else {
        // addons mode cannot register a room or hold admins by construction, so running the full
        // verdict here would manufacture two guaranteed failures. Assert the half that IS
        // meaningful and say plainly that the rest was not checked, rather than printing green.
        println!(
            "==> boot verdict: {} (mode=addons — addon check only)",
            local_log.display()
        );
        if boot::assert_local_addon_won(&mut out, &local_log, &env.addon_guid, &env.addons_staging)
            != 0
        {
            eprintln!("Full log kept at: {}", local_log.display());
            return Ok(Some(1));
        }
        println!("  SKIP  room + admin checks: addons mode registers no room and loads no server");
        println!("        config. This server is NOT joinable and has NO admins. Use config mode.");
    }
    let _ = fs::remove_file(&local_log);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_staging::config::tests::base;

    // ── ARGV: the stand-in for live coverage ────────────────────────────────────────────────

    #[test]
    fn ssh_argv_plain_identity_and_sshpass() {
        let remote = vec!["bash".to_string(), "-s".to_string()];
        assert_eq!(
            ssh_argv(&SshBase::Plain, "sam@h", &remote),
            vec![
                "ssh",
                "-o",
                "StrictHostKeyChecking=no",
                "sam@h",
                "bash",
                "-s"
            ]
        );
        assert_eq!(
            ssh_argv(&SshBase::Identity("/k/id".into()), "sam@h", &remote),
            vec![
                "ssh",
                "-i",
                "/k/id",
                "-o",
                "StrictHostKeyChecking=no",
                "sam@h",
                "bash",
                "-s"
            ]
        );
        assert_eq!(
            ssh_argv(&SshBase::Pass("pw".into()), "sam@h", &remote),
            vec![
                "sshpass",
                "-p",
                "pw",
                "ssh",
                "-o",
                "StrictHostKeyChecking=no",
                "sam@h",
                "bash",
                "-s"
            ]
        );
    }

    #[test]
    fn sshpass_wins_over_identity_file() {
        // ODDITY PINNED: a deploy.env with both silently ignores the key.
        let mut e = base();
        e.ssh_pass = Some("pw".into());
        e.ssh_identity_file = Some("/k/id".into());
        assert_eq!(SshBase::from_env(&e), SshBase::Pass("pw".into()));
        e.ssh_pass = None;
        assert_eq!(SshBase::from_env(&e), SshBase::Identity("/k/id".into()));
        e.ssh_identity_file = None;
        assert_eq!(SshBase::from_env(&e), SshBase::Plain);
    }

    #[test]
    fn rsync_argv_keeps_every_exclude_in_order() {
        let argv = rsync_argv(
            &SshBase::Plain,
            Path::new("/repo"),
            "sam@h",
            "/home/sam/tbd/repo",
        );
        assert_eq!(argv[0], "rsync");
        assert_eq!(argv[1], "-e");
        assert_eq!(argv[2], "ssh -o StrictHostKeyChecking=no");
        assert_eq!(argv[3], "-avz");
        assert_eq!(argv[4], "--delete");
        // The licence boundary. All three oracle lanes, plus the credential itself.
        for needed in [
            "--exclude=apps/mod/crf_framework/",
            "--exclude=apps/mod/vanilla_reference/",
            "--exclude=apps/mod/playable_selector/",
            "--exclude=scripts/deploy/deploy.env",
            "--exclude=apps/website/api/.env",
            "--exclude=apps/mod/tbd-framework/Scripts/WorkbenchGame/",
        ] {
            assert!(argv.iter().any(|a| a == needed), "missing {needed}");
        }
        // Source has a trailing slash (rsync copies CONTENTS) and so does the destination.
        assert_eq!(argv[argv.len() - 2], "/repo/");
        assert_eq!(argv[argv.len() - 1], "sam@h:/home/sam/tbd/repo/");
    }

    #[test]
    fn exec_start_config_mode_carries_both_flags() {
        // The whole of T-604/T-607: without -addonsDir the engine loads the Workshop copy.
        let s = exec_start(&base());
        assert!(s.contains(" -addonsDir /home/sam/tbd/addons "), "{s}");
        assert!(
            s.contains(" -config /home/sam/tbd/server.config.json "),
            "{s}"
        );
        assert!(
            !s.contains(" -addons "),
            "config mode must NOT pass -addons: {s}"
        );
        assert!(s.ends_with("-maxFPS 60 -logStats 30000 -nothrow"));
    }

    #[test]
    fn exec_start_addons_mode_quotes_the_scenario() {
        let mut e = base();
        e.server_mode = "addons".into();
        let s = exec_start(&e);
        // -addons <GUID> is the flag that is fatal beside -config, and addons mode is the only
        // place it appears.
        assert!(s.contains(" -addons B2C3D4E5F6A78901 "), "{s}");
        assert!(!s.contains("-config"), "{s}");
        assert!(
            s.contains(" -server \"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf\" "),
            "{s}"
        );
        assert!(s.contains(" -bindPort 2001 -a2sPort 17777 "), "{s}");
    }

    #[test]
    fn v6_maps_all_four_outcomes_and_refuses_to_guess() {
        // The four-outcome contract, which is the whole reason this step is not `set -e`'d.
        assert_eq!(v6_verdict(0), 0, "HEALTHY");
        assert_eq!(
            v6_verdict(2),
            0,
            "PARTIAL is the NORMAL post-deploy state, not a failure"
        );
        assert_eq!(v6_verdict(1), 1, "FAIL");
        assert_eq!(
            v6_verdict(3),
            1,
            "ENVIRONMENT examined nothing and is not a pass"
        );
        assert_eq!(
            v6_verdict(127),
            1,
            "unknown status is a failure, not a guess"
        );
        assert_eq!(v6_verdict(-1), 1);
    }

    #[test]
    fn not_run_never_reads_as_success() {
        // The type-level version of the fail-open this whole program is written against.
        assert_eq!(not_run_exit(&NotRun::ToolAbsent("ssh".into())), 127);
        assert_eq!(
            not_run_exit(&NotRun::Signalled {
                tool: "ssh".into(),
                signal: 9
            }),
            1
        );
        assert_eq!(
            not_run_exit(&NotRun::Timeout {
                tool: "rsync".into(),
                secs: 7200
            }),
            1
        );
    }

    #[test]
    fn dry_run_never_spawns() {
        // The `Runner` guard, asserted directly: with dry_run set, an `sshpass` that may not exist
        // on this machine still yields 0 and prints the plan instead of reaching a socket.
        let r = Runner { dry_run: true };
        let code = r
            .ssh(
                &SshBase::Pass("pw".into()),
                "sam@h",
                &["bash".to_string(), "-s".to_string()],
                Some("payload".into()),
            )
            .expect("dry run cannot fail");
        assert_eq!(code, 0);
    }
}
