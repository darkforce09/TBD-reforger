use super::*;

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
        "--exclude=apps/website/api_v2/.tools/".into(),
        "--exclude=apps/website/api_v2/.env".into(),
        "--exclude=apps/mod/tbd-export/".into(),
        "--exclude=apps/mod/tbd-emcp/".into(),
        format!("--exclude={}", crate::core::repository_layout::DEPLOY_ENV),
        // Build output and the map asset trees: a game-server host needs none of it, and the
        // scratch tree alone is 1.5 GB of gitignored export intermediates. Excluded paths are
        // also protected from `--delete` (there is no `--delete-excluded`).
        "--exclude=target/".into(),
        "--exclude=assets_v2/terrains/".into(),
        "--exclude=assets_v2/scratch/".into(),
        format!("{}/", mono_root.display()),
        format!("{host}:{remote_dir}/"),
    ]
}

/// `ExecStart` per mode.
///
/// `-config` is mutually exclusive with **`-addons`** — NOT with `-addonsDir`. Those are two
/// different flags and the distinction is the whole point. `-addons <GUID>` asks the engine to
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

pub(super) fn not_run_exit(e: &NotRun) -> u8 {
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

/// READ THE EXIT CODE of `mod remote-logs`, do not just inherit it.
///
/// `remote-log-grep` is a FOUR-outcome check and this script is the consumer that pinned `2`:
///
/// ```text
/// 0 HEALTHY  ·  1 FAIL  ·  2 PARTIAL (booted, nobody joined yet)  ·  3 ENVIRONMENT
/// ```
///
/// Were this the last statement in the file, under `set -e` the deploy would simply exit with
/// whatever it returned. `2` is the NORMAL state immediately after a deploy — nobody has had time
/// to join — so every healthy deploy reported failure to any caller reading `!= 0`, and the fix
/// people reach for when a green run keeps "failing" is to stop believing the gate. `3` is the
/// opposite hazard and must never be soft: it means no log was examined at all, so it says nothing
/// about the mod and cannot be allowed to read as success.
///
/// The same contract applies to `cargo xtask mcp wb-logs` and `cargo xtask mod spawn-verify`: an
/// inverted reading of either passes only on a stale build. Do not build a staging check on a
/// `!= 0` reading of any of the three.
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
        return Ok(super::super::render::render_only(&env, out));
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
    // The compose file lives at apps/website/docker-compose.staging.yml, not under
    // apps/website/api_v2/. Match `cargo xtask deploy website`.
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
        println!("[SKIP] V2–V4 API smoke — routes not in the current backend; would 404.");
        println!(
            "       Set TBD_RUN_T092_SMOKE=1 to force once those routes ship. See {}.",
            crate::core::repository_layout::documentation::STAGING_SERVER_RUNBOOK
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
        // it. Render is split from push: an invalid or empty mod list fails here, on the dev
        // machine, instead of landing on the server and failing at boot.
        if env.server_mode == "config" {
            let local =
                std::env::temp_dir().join(format!("tbd-server.config.{}.json", std::process::id()));
            if let Err(code) = super::super::render::render_server_config(&env, &local) {
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

    // ── Install the host control agent ──────────────────────────────────────────────────────
    //
    // OFF BY DEFAULT. The render above is proven by --agent-selftest; THIS step is not, because
    // exercising it means mutating the live staging host, which no test may touch.
    // It also buys nothing until the API side lands — nothing would connect to the socket.
    //
    // The agent is enabled via its SOCKET, never its service: socket activation means the agent
    // process only exists for the lifetime of one connection, so there is no long-lived listener
    // to leak, wedge, or restart.
    println!("==> host control agent");
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
pub(super) fn install_agent(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    agent_env: &AgentEnv,
) -> Result<Option<u8>> {
    // Render LOCALLY and validate BEFORE anything is pushed — the posture: a broken artefact
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
