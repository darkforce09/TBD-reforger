use super::*;

/// CLI entry: trailing args mirror bash's `for arg in "$@"` parser.
pub fn run(args: &[String]) -> Result<u8> {
    let opts = match parse_args(args) {
        Ok(o) => o,
        Err(code) => return Ok(code),
    };
    if opts.selftest {
        let v = crate::commands::mod_ops::world_boot_verdict::cmd_selftest();
        let t = t302_selftest();
        return Ok(if v == 0 && t == 0 { 0 } else { 1 });
    }
    boot(&find_repo_root()?, opts)
}

pub(super) fn parse_args(args: &[String]) -> std::result::Result<Opts, u8> {
    let mut opts = Opts {
        keep_logs: false,
        selftest: false,
        mission: None,
        compiled: false,
        compiled_uuid: None,
    };
    for arg in args {
        if arg == "--keep-logs" {
            opts.keep_logs = true;
        } else if arg == "--selftest" {
            opts.selftest = true;
        } else if let Some(rest) = arg.strip_prefix("--mission=") {
            opts.mission = Some(rest.to_string());
        } else if arg == "--mission" {
            eprintln!("use --mission=<file|name>");
            return Err(2);
        } else if arg == "--compiled" {
            opts.compiled = true;
        } else if let Some(rest) = arg.strip_prefix("--compiled=") {
            opts.compiled = true;
            opts.compiled_uuid = Some(rest.to_string());
        } else {
            eprintln!("unknown argument: {arg}");
            return Err(2);
        }
    }
    if opts.compiled && opts.mission.is_some() {
        eprintln!("ERROR: --compiled and --mission are mutually exclusive");
        return Err(2);
    }
    Ok(opts)
}

pub(super) fn boot(root: &Path, mut opts: Opts) -> Result<u8> {
    let goldens = root.join("packages/tbd-schema/golden-missions");
    if let Some(ref m) = opts.mission.clone() {
        if !Path::new(&m).is_file() {
            let as_json = goldens.join(format!("{m}.json"));
            let as_raw = goldens.join(m);
            if as_json.is_file() {
                opts.mission = Some(as_json.to_string_lossy().into_owned());
            } else if as_raw.is_file() {
                opts.mission = Some(as_raw.to_string_lossy().into_owned());
            } else {
                eprintln!(
                    "ERROR: no such mission '{m}' (looked in {})",
                    goldens.display()
                );
                return Ok(2);
            }
        }
    }

    if !require_host() {
        return Ok(env_fail(
            "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine",
            Some(
                "See tools_v2/xtask/src/core/host_execution.rs: the container has no C toolchain and an older glibc, so the game binary cannot run in here at all.",
            ),
        ));
    }

    let home = std::env::var_os("HOME").unwrap_or_default();
    let server_dir = PathBuf::from(&home).join(SERVER_REL);
    let server_bin = server_dir.join("ArmaReforgerServer");
    if !is_executable(&server_bin) {
        return Ok(env_fail(
            &format!("server binary not found at {}", server_bin.display()),
            Some("Install it from Steam (appid 1890870):  steam steam://install/1890870"),
        ));
    }
    let dev_config = root.join("scripts/mod/tbd-dev-server.config.json");
    if !dev_config.is_file() {
        return Ok(env_fail(
            &format!("dev config not found at {}", dev_config.display()),
            Some(
                "The checkout does not look like this repo — verify the working tree before blaming the mod.",
            ),
        ));
    }

    let mod_src = root.join("apps/mod/tbd-framework");
    let addon_guid = read_addon_guid(&mod_src.join("addon.gproj")).unwrap_or_default();
    if addon_guid.is_empty() {
        eprintln!(
            "ERROR: could not read GUID from {}",
            mod_src.join("addon.gproj").display()
        );
        return Ok(1);
    }
    let scenario = read_scenario_id(&dev_config).unwrap_or_default();
    if scenario.is_empty() {
        eprintln!(
            "ERROR: could not read scenarioId from {}",
            dev_config.display()
        );
        return Ok(1);
    }

    let max_wait: u64 = env_u64("TBD_WORLDBOOT_TIMEOUT", 240);
    let settle: u64 = env_u64("TBD_WORLDBOOT_SETTLE", 4);
    let api_base = std::env::var("TBD_API_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let warn_baseline = root.join(".world-boot-warning-baseline");

    let run_dir = tempfile_dir("tbd-worldboot")?;
    fs::create_dir_all(run_dir.join("addons"))?;
    fs::create_dir_all(run_dir.join("profile"))?;
    let link = run_dir.join("addons/tbd-framework");
    let _ = fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&mod_src, &link)?;

    let mut state = RunState {
        run_dir: run_dir.clone(),
        keep_logs: opts.keep_logs,
        cleaned: AtomicBool::new(false),
        svc_token: None,
        dev_access_token: None,
        api_base: api_base.clone(),
        child: None,
    };

    let pidfile = run_dir.join("server.pid");
    let mut mission_path = opts.mission.clone();
    let mut warn_key = String::new();
    let mut compiled_uuid = opts.compiled_uuid.clone();

    if opts.compiled {
        println!("==> seeding a compiled mission via {api_base}");
        if let Err(GateExit(code)) = compiled_lane(
            &mut state,
            root,
            &api_base,
            &mut compiled_uuid,
            &mut mission_path,
            &mut warn_key,
        ) {
            state.cleanup();
            return Ok(code);
        }
    }

    let mut mission_id = String::new();
    if let Some(ref mission) = mission_path {
        let doc: Value = serde_json::from_str(&fs::read_to_string(mission)?)
            .with_context(|| format!("parse mission {mission}"))?;
        mission_id = doc
            .pointer("/meta/id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if mission_id.is_empty() {
            eprintln!("ERROR: {mission} has no meta.id");
            state.cleanup();
            return Ok(1);
        }
        let dest_dir = run_dir.join("profile/profile/missions");
        fs::create_dir_all(&dest_dir)?;
        fs::copy(mission, dest_dir.join(format!("{mission_id}.json")))?;
        fs::write(
            run_dir.join("profile/profile/TBD_BackendConfig.json"),
            format!(
                "{{\"backendUrl\":\"\",\"serverToken\":\"\",\"missionId\":\"{mission_id}\",\"eventId\":\"\"}}\n"
            ),
        )?;
        fs::copy(
            mod_src.join("Data/registry.json"),
            run_dir.join("profile/profile/TBD_Registry.json"),
        )?;
        if warn_key.is_empty() {
            warn_key = mission_id.clone();
        }
    }

    let bind_port = 21000 + (std::process::id() % 4000);
    let a2s_port = 26000 + (std::process::id() % 4000);
    write_server_json(
        &dev_config,
        &run_dir.join("server.json"),
        &addon_guid,
        bind_port,
        a2s_port,
    )?;

    println!("==> booting world (addon {addon_guid}, scenario {scenario})");
    state.child = Some(spawn_server(&server_dir, &run_dir, max_wait)?);

    let log = poll_for_log(
        &run_dir,
        max_wait,
        (!mission_id.is_empty()).then_some(mission_id.as_str()),
    );
    thread::sleep(Duration::from_secs(settle));
    kill_run(&pidfile);
    if let Some(mut c) = state.child.take() {
        for _ in 0..10 {
            if c.try_wait()?.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
        let _ = c.kill();
        let _ = c.wait();
    }

    let Some(log_path) = log else {
        let code = env_fail(
            &format!(
                "no console.log produced under {}/profile/logs — the engine never started writing",
                run_dir.display()
            ),
            Some(&format!(
                "Check that {} runs at all and that {} is writable.",
                server_bin.display(),
                std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into())
            )),
        );
        state.cleanup();
        return Ok(code);
    };

    let text = fs::read_to_string(&log_path).unwrap_or_default();
    let fatal = Regex::new(r"\(F\):").expect("fatal");
    if text.lines().any(|l| fatal.is_match(l)) {
        println!("  FATAL from engine:");
        for line in text.lines().filter(|l| fatal.is_match(l)).take(4) {
            println!("        {line}");
        }
    }

    let mission_ctx = if mission_id.is_empty() {
        None
    } else {
        Some(MissionCtx {
            mission_id: &mission_id,
            warn_key: if warn_key.is_empty() {
                &mission_id
            } else {
                &warn_key
            },
            warn_baseline: &warn_baseline,
        })
    };

    let mut pass =
        crate::commands::mod_ops::world_boot_verdict::assess_log(&log_path, &scenario, mission_ctx);
    if opts.compiled {
        pass = t302_assert(&text, true) && pass;
    }
    if pass {
        println!("WORLD BOOT: PASS");
        state.cleanup();
        Ok(0)
    } else {
        println!("WORLD BOOT: FAIL");
        if !opts.keep_logs {
            println!("  (re-run with --keep-logs to inspect the full console.log)");
        }
        state.cleanup();
        Ok(1)
    }
}

#[rustfmt::skip]
pub(super) fn env_fail(msg: &str, hint: Option<&str>) -> u8 {
    println!("\nWORLD BOOT: ENV FAIL — {msg}");
    println!("  This is the HARNESS's environment. The world was never booted, so this says NOTHING about the mod — do not read it as a code failure.");
    if let Some(h) = hint { println!("  {h}"); }
    3
}

#[rustfmt::skip]
pub(super) fn api_env_fail(api_base: &str, msg: &str, hint: Option<&str>) -> GateExit {
    let default = format!("Bring the stack up and re-run:  cargo xtask db up && cargo xtask mk rust-api   (API expected at {api_base})");
    println!("\nCOMPILED BOOT: ENV FAIL — {msg}");
    println!("  This is the HARNESS's environment. The mod was never started, so this says NOTHING about the mod or the compiler — do not read it as a code failure.");
    println!("  {}", hint.unwrap_or(&default));
    GateExit(3)
}

#[rustfmt::skip]
pub(super) fn api_doc_fail(msg: &str) -> GateExit {
    println!("\nCOMPILED BOOT: FAIL — {msg}");
    println!("  The API would not produce a compiled document. That is a COMPILER/CONTRACT defect, not an environment one — re-running will not fix it.");
    println!("  Check the API log: a 500 is schema validation (validated_compiled_body in apps/website/api/src/handlers/missions/missions.rs); a 409 is no placed slots.");
    GateExit(1)
}

#[rustfmt::skip]
pub(super) fn api_http_fail(api_base: &str, code: u16, what: &str, doc_msg: &str) -> GateExit {
    if code == 404 {
        return api_env_fail(api_base, &format!("{what} -> HTTP 404 — nothing at that id/route on {api_base}"),
            Some("Check the mission id you passed and that the API is the one you think it is."));
    }
    if (500..600).contains(&code) {
        let hint = format!("Check the API log first, then:  cargo xtask db up && cargo xtask mk rust-api   (API expected at {api_base})");
        return api_env_fail(api_base,
            &format!("{what} -> HTTP {code} — the API could not serve the request. A stopped or unmigrated Postgres surfaces here as a 500; the API log says which."),
            Some(&hint));
    }
    api_doc_fail(doc_msg)
}
