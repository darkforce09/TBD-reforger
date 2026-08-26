//! T-892 — port of `scripts/mod/world-boot.sh` → `cargo xtask mod world-boot`.
//!
//! Exit: **0** PASS · **1** CODE · **2** usage · **3** ENVIRONMENT.
//! Verdict / `--selftest` → [`crate::mod_world_boot_verdict`] (SIZE split).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Value, json};

use crate::mod_world_boot_verdict::{self, MissionCtx};
use crate::root::find_repo_root;

const FIXTURE_TITLE: &str = "T-186 compiled-boot fixture";
const SERVER_REL: &str = ".local/share/Steam/steamapps/common/Arma Reforger Server";

struct Opts {
    keep_logs: bool,
    selftest: bool,
    mission: Option<String>,
    compiled: bool,
    compiled_uuid: Option<String>,
}

/// CLI entry: trailing args mirror bash's `for arg in "$@"` parser.
pub fn run(args: &[String]) -> Result<u8> {
    let opts = match parse_args(args) {
        Ok(o) => o,
        Err(code) => return Ok(code),
    };
    if opts.selftest {
        return Ok(mod_world_boot_verdict::cmd_selftest());
    }
    boot(&find_repo_root()?, opts)
}

fn parse_args(args: &[String]) -> std::result::Result<Opts, u8> {
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

fn boot(root: &Path, mut opts: Opts) -> Result<u8> {
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
                "See xtask/src/hostrun.rs: the container has no C toolchain and an older glibc, so the game binary cannot run in here at all.",
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

    if mod_world_boot_verdict::assess_log(&log_path, &scenario, mission_ctx) {
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

struct GateExit(u8);

struct RunState {
    run_dir: PathBuf,
    keep_logs: bool,
    cleaned: AtomicBool,
    svc_token: Option<String>,
    dev_access_token: Option<String>,
    api_base: String,
    child: Option<Child>,
}

impl RunState {
    fn cleanup(&self) {
        if self
            .cleaned
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        kill_run(&self.run_dir.join("server.pid"));
        sweep_fixture_missions(
            &self.run_dir,
            &self.api_base,
            self.svc_token.as_deref(),
            self.dev_access_token.as_deref(),
        );
        if self.keep_logs {
            println!("run dir kept: {}", self.run_dir.display());
        } else {
            let _ = fs::remove_dir_all(&self.run_dir);
        }
    }
}

impl Drop for RunState {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn env_fail(msg: &str, hint: Option<&str>) -> u8 {
    println!();
    println!("WORLD BOOT: ENV FAIL — {msg}");
    println!(
        "  This is the HARNESS's environment. The world was never booted, so this says NOTHING"
    );
    println!("  about the mod — do not read it as a code failure.");
    if let Some(h) = hint {
        println!("  {h}");
    }
    3
}

fn api_env_fail(api_base: &str, msg: &str, hint: Option<&str>) -> GateExit {
    let default = format!(
        "Bring the stack up and re-run:  cargo xtask db up && cargo xtask mk rust-api   (API expected at {api_base})"
    );
    println!();
    println!("COMPILED BOOT: ENV FAIL — {msg}");
    println!(
        "  This is the HARNESS's environment. The mod was never started, so this says NOTHING"
    );
    println!("  about the mod or the compiler — do not read it as a code failure.");
    println!("  {}", hint.unwrap_or(&default));
    GateExit(3)
}

fn api_doc_fail(msg: &str) -> GateExit {
    println!();
    println!("COMPILED BOOT: FAIL — {msg}");
    println!(
        "  The API would not produce a compiled document. That is a COMPILER/CONTRACT defect,"
    );
    println!(
        "  not an environment one — re-running will not fix it. Check the API log: a 500 here is"
    );
    println!("  'compiled mission failed schema validation' (validated_compiled_body in");
    println!("  apps/website/api/src/handlers/missions/missions.rs), a 409 is 'no placed slots'.");
    GateExit(1)
}

fn api_http_fail(api_base: &str, code: u16, what: &str, doc_msg: &str) -> GateExit {
    if code == 404 {
        return api_env_fail(
            api_base,
            &format!("{what} -> HTTP 404 — nothing at that id/route on {api_base}"),
            Some("Check the mission id you passed and that the API is the one you think it is."),
        );
    }
    if (500..600).contains(&code) {
        let hint = format!(
            "Check the API log first, then:  cargo xtask db up && cargo xtask mk rust-api   (API expected at {api_base})"
        );
        return api_env_fail(
            api_base,
            &format!(
                "{what} -> HTTP {code} — the API could not serve the request. A stopped or unmigrated Postgres surfaces here as a 500; the API log says which."
            ),
            Some(&hint),
        );
    }
    api_doc_fail(doc_msg)
}

fn compiled_lane(
    state: &mut RunState,
    root: &Path,
    api_base: &str,
    compiled_uuid: &mut Option<String>,
    mission_path: &mut Option<String>,
    warn_key: &mut String,
) -> std::result::Result<(), GateExit> {
    let svc = resolve_service_token(root).ok_or_else(|| {
        api_env_fail(
            api_base,
            "no SERVICE_TOKEN — set TBD_SERVICE_TOKEN, or add it to apps/website/api/.env",
            None,
        )
    })?;
    state.svc_token = Some(svc.clone());
    let err_path = state.run_dir.join("curl.err");

    let (probe_rc, probe_code) = curl_http(
        &[
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-m",
            "10",
            "-H",
            &format!("X-Service-Token: {svc}"),
            &format!("{api_base}/api/v1/ingest/missions"),
        ],
        &err_path,
    );
    if probe_rc != 0 {
        let err = fs::read_to_string(&err_path)
            .unwrap_or_default()
            .replace('\n', " ");
        return Err(api_env_fail(
            api_base,
            &format!("API unreachable at {api_base} (curl exit {probe_rc}: {err})"),
            None,
        ));
    }
    if probe_code == 401 {
        return Err(api_env_fail(
            api_base,
            "service token rejected (GET /api/v1/ingest/missions -> 401) — SERVICE_TOKEN does not match the running API",
            None,
        ));
    }
    if probe_code != 200 {
        return Err(api_env_fail(
            api_base,
            &format!(
                "service-token probe GET /api/v1/ingest/missions -> HTTP {probe_code} (expected 200)"
            ),
            None,
        ));
    }

    if compiled_uuid.as_ref().is_some_and(|s| !s.is_empty()) {
        println!(
            "    using existing mission {}",
            compiled_uuid.as_deref().unwrap_or("")
        );
    } else {
        let token = dev_login_token(api_base).ok_or_else(|| {
            api_env_fail(
                api_base,
                "dev-login returned no access_token — is the API running with APP_ENV=development?",
                None,
            )
        })?;
        state.dev_access_token = Some(token.clone());
        let seed_path = state.run_dir.join("seed.json");
        fs::write(&seed_path, seed_fixture_body())
            .map_err(|_| api_env_fail(api_base, "could not write seed.json", None))?;
        let resp_path = state.run_dir.join("seed-resp.json");
        let resp_s = resp_path.to_string_lossy().into_owned();
        let seed_s = format!("@{}", seed_path.display());
        let (seed_rc, seed_code) = curl_http(
            &[
                "-sS",
                "-o",
                &resp_s,
                "-w",
                "%{http_code}",
                "-m",
                "30",
                "-X",
                "POST",
                &format!("{api_base}/api/v1/missions"),
                "-H",
                &format!("Authorization: Bearer {token}"),
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                &seed_s,
            ],
            &err_path,
        );
        if seed_rc != 0 {
            let err = fs::read_to_string(&err_path)
                .unwrap_or_default()
                .replace('\n', " ");
            return Err(api_env_fail(
                api_base,
                &format!("POST /api/v1/missions transport failure (curl exit {seed_rc}: {err})"),
                None,
            ));
        }
        if seed_code != 201 {
            println!("  POST /api/v1/missions -> HTTP {seed_code}");
            let body = fs::read_to_string(&resp_path).unwrap_or_default();
            println!("{}", body.chars().take(600).collect::<String>());
            return Err(api_http_fail(
                api_base,
                seed_code,
                "POST /api/v1/missions",
                &format!(
                    "the API rejected the editor payload this harness seeds (HTTP {seed_code})"
                ),
            ));
        }
        let id = serde_json::from_str::<Value>(&fs::read_to_string(&resp_path).unwrap_or_default())
            .ok()
            .and_then(|v| v.get("id")?.as_str().map(str::to_string))
            .unwrap_or_default();
        if id.is_empty() {
            return Err(api_doc_fail(
                "POST /api/v1/missions 201 but returned no mission id",
            ));
        }
        println!("    seeded mission {id}");
        *compiled_uuid = Some(id);
    }

    let uuid = compiled_uuid.clone().unwrap_or_default();
    let compiled_path = state.run_dir.join("compiled.json");
    let out_s = compiled_path.to_string_lossy().into_owned();
    let (comp_rc, comp_code) = curl_http(
        &[
            "-sS",
            "-o",
            &out_s,
            "-w",
            "%{http_code}",
            "-m",
            "60",
            "-H",
            &format!("X-Service-Token: {svc}"),
            &format!("{api_base}/api/v1/missions/{uuid}/compiled"),
        ],
        &err_path,
    );
    if comp_rc != 0 {
        let err = fs::read_to_string(&err_path)
            .unwrap_or_default()
            .replace('\n', " ");
        return Err(api_env_fail(
            api_base,
            &format!("GET /compiled transport failure (curl exit {comp_rc}: {err})"),
            None,
        ));
    }
    if comp_code != 200 {
        println!("  GET /api/v1/missions/{uuid}/compiled -> HTTP {comp_code}");
        let body = fs::read_to_string(&compiled_path).unwrap_or_default();
        println!("{}", body.chars().take(1200).collect::<String>());
        return Err(api_http_fail(
            api_base,
            comp_code,
            &format!("GET /api/v1/missions/{uuid}/compiled"),
            &format!("GET /compiled -> HTTP {comp_code} (expected 200)"),
        ));
    }
    let bytes = fs::metadata(&compiled_path).map(|m| m.len()).unwrap_or(0);
    println!("    fetched {bytes} bytes of compiled document");
    *mission_path = Some(compiled_path.to_string_lossy().into_owned());
    *warn_key = "compiled".into();
    Ok(())
}

fn seed_fixture_body() -> String {
    let v = json!({
        "title": FIXTURE_TITLE,
        "terrain": "everon",
        "game_mode": "pvp",
        "weather": "clear",
        "time_of_day": "05:30",
        "max_players": 8,
        "briefing": "Generated by cargo xtask mod world-boot --compiled. Safe to delete.",
        "payload": {
            "schemaVersion": 1,
            "editor": {
                "factions": [{ "key": "blufor", "name": "US Army", "squadIds": ["sq_alpha"] }],
                "squads": [{ "id": "sq_alpha", "callsign": "Alpha", "name": "Alpha", "slotIds": ["sl_sl", "sl_ar", "sl_rfl"] }],
                "slots": [
                    {
                        "id": "sl_sl", "index": 0, "role": "SL",
                        "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",
                        "position": { "x": 4870.0, "y": 7760.0, "z": 0.0, "rotation": 45.0 }
                    },
                    {
                        "id": "sl_ar", "index": 1, "role": "AR",
                        "assetId": "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et",
                        "position": { "x": 4880.0, "y": 7770.0, "z": 0.0, "rotation": 90.0 },
                        "loadout": {
                            "wear": {
                                "jacket": "{293F577C298061E3}Prefabs/Characters/Uniforms/Jacket_US_BDU_02.et",
                                "armoredVest": "{477A190AF2A17B8A}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_MG.et",
                                "headCover": "{B74A4FF0DD8BB116}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01.et",
                                "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
                                "boots": "{DAAFD15478BDE1C3}Prefabs/Characters/Footwear/CombatBoots_US_01.et"
                            },
                            "weapons": [{
                                "slotIndex": 0, "slotType": "primary",
                                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et"
                            }],
                            "cargo": [{
                                "container": "vest",
                                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                                "qty": 6
                            }]
                        }
                    },
                    {
                        "id": "sl_rfl", "index": 2, "role": "RFL",
                        "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
                        "position": { "x": 4890.0, "y": 7780.0, "z": 136.0, "rotation": 315.0 }
                    }
                ]
            }
        }
    });
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}

fn write_server_json(
    src: &Path,
    dst: &Path,
    guid: &str,
    bind_port: u32,
    a2s_port: u32,
) -> Result<()> {
    let mut cfg: Value = serde_json::from_str(&fs::read_to_string(src)?)?;
    cfg["bindPort"] = json!(bind_port);
    cfg["publicPort"] = json!(bind_port);
    if cfg.get("a2s").map(|v| v.is_object()).unwrap_or(false) {
        cfg["a2s"]["port"] = json!(a2s_port);
    }
    if cfg.get("game").is_none() {
        cfg["game"] = json!({});
    }
    cfg["game"]["mods"] = json!([{ "modId": guid, "name": "TBD_Framework" }]);
    let mut f = fs::File::create(dst)?;
    serde_json::to_writer_pretty(&mut f, &cfg)?;
    Ok(())
}

fn spawn_server(server_dir: &Path, run_dir: &Path, max_wait: u64) -> Result<Child> {
    let run = run_dir.display().to_string();
    let script = format!(
        "echo $$ > \"{run}/server.pid\"\n\
         exec timeout {max_wait} ./ArmaReforgerServer \\\n\
           -addonsDir \"{run}/addons\" -config \"{run}/server.json\" -profile \"{run}/profile\" -maxFPS 15"
    );
    Ok(host_command("env")
        .arg("-C")
        .arg(server_dir)
        .arg("setsid")
        .arg("sh")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

fn poll_for_log(run_dir: &Path, max_wait: u64, mission: Option<&str>) -> Option<PathBuf> {
    let mission_pat = Regex::new(r"mission result=").ok()?;
    let roll_pat = Regex::new(r"\[TBD\] roll-call").ok()?;
    let fatal_pat = Regex::new(r"\(F\):|Unable to initialize the game").ok()?;
    for _ in 0..(max_wait * 2) {
        if let Some(log) = latest_console_log(run_dir)
            && let Ok(text) = fs::read_to_string(&log)
        {
            let done = if mission.is_some() {
                mission_pat.is_match(&text)
            } else {
                roll_pat.is_match(&text)
            };
            if done || fatal_pat.is_match(&text) {
                return Some(log);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    latest_console_log(run_dir)
}

fn latest_console_log(run_dir: &Path) -> Option<PathBuf> {
    let logs = run_dir.join("profile/logs");
    let mut dirs: Vec<_> = fs::read_dir(&logs)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("logs_"))
        })
        .collect();
    dirs.sort();
    let console = dirs.pop()?.join("console.log");
    console.is_file().then_some(console)
}

fn kill_run(pidfile: &Path) {
    let Ok(s) = fs::read_to_string(pidfile) else {
        return;
    };
    let pgid = s.trim();
    if pgid.is_empty() {
        return;
    }
    let neg = format!("-{pgid}");
    let _ = host_command("kill")
        .args(["-TERM", "--", &neg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    for _ in 0..20 {
        let alive = host_command("kill")
            .args(["-0", "--", &neg])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !alive {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    let _ = host_command("kill")
        .args(["-9", "--", &neg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn sweep_fixture_missions(run_dir: &Path, api_base: &str, svc: Option<&str>, dev: Option<&str>) {
    let (Some(svc), Some(dev)) = (svc, dev) else {
        return;
    };
    let listing = run_dir.join("sweep.json");
    let out = listing.to_string_lossy().into_owned();
    let _ = Command::new("curl")
        .args([
            "-sS",
            "-o",
            &out,
            "-m",
            "10",
            "-H",
            &format!("X-Service-Token: {svc}"),
            &format!("{api_base}/api/v1/ingest/missions"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let Ok(v) = serde_json::from_str::<Value>(&fs::read_to_string(&listing).unwrap_or_default())
    else {
        return;
    };
    for m in v
        .get("missions")
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
    {
        if m.get("name").and_then(|n| n.as_str()) != Some(FIXTURE_TITLE) {
            continue;
        }
        let Some(id) = m.get("id").and_then(|i| i.as_str()) else {
            continue;
        };
        let _ = Command::new("curl")
            .args([
                "-sS",
                "-o",
                "/dev/null",
                "-m",
                "10",
                "-X",
                "DELETE",
                &format!("{api_base}/api/v1/missions/{id}"),
                "-H",
                &format!("Authorization: Bearer {dev}"),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn resolve_service_token(root: &Path) -> Option<String> {
    if let Ok(t) = std::env::var("TBD_SERVICE_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    let main_root = git_main_root(root).unwrap_or_else(|| root.to_path_buf());
    for f in [
        root.join("apps/website/api/.env"),
        main_root.join("apps/website/api/.env"),
    ] {
        if let Some(tok) = token_from_env_file(&f) {
            return Some(tok);
        }
    }
    None
}

fn token_from_env_file(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("SERVICE_TOKEN=") else {
            continue;
        };
        let mut tok = rest.trim_end_matches('\r').to_string();
        if (tok.starts_with('"') && tok.ends_with('"'))
            || (tok.starts_with('\'') && tok.ends_with('\''))
        {
            tok = tok[1..tok.len() - 1].to_string();
        }
        if !tok.is_empty() {
            return Some(tok);
        }
    }
    None
}

fn git_main_root(root: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args([
            "-C",
            root.to_str()?,
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let common = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&common).parent().map(|p| p.to_path_buf())
}

fn dev_login_token(api_base: &str) -> Option<String> {
    let out = Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-D",
            "-",
            "-m",
            "10",
            &format!("{api_base}/api/v1/auth/dev-login?role=mission_maker"),
        ])
        .output()
        .ok()?;
    let headers = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    Regex::new(r"[#&]access_token=([^&]*)")
        .ok()?
        .captures(&headers)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn curl_http(args: &[&str], err_path: &Path) -> (i32, u16) {
    let mut cmd = Command::new("curl");
    cmd.args(args);
    if let Ok(f) = fs::File::create(err_path) {
        cmd.stderr(Stdio::from(f));
    }
    match cmd.output() {
        Ok(o) => {
            let code = String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse()
                .unwrap_or(0);
            (o.status.code().unwrap_or(1), code)
        }
        Err(_) => (127, 0),
    }
}

fn read_addon_guid(gproj: &Path) -> Option<String> {
    let text = fs::read_to_string(gproj).ok()?;
    Regex::new(r#"(?m)^\s*GUID\s+"([0-9A-Fa-f]+)""#)
        .ok()?
        .captures(&text)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn read_scenario_id(config: &Path) -> Option<String> {
    let text = fs::read_to_string(config).ok()?;
    let chunk = Regex::new(r#""scenarioId"[^,]*"#)
        .ok()?
        .find(&text)?
        .as_str();
    Regex::new(r#"\{[^}]+\}[^"]*"#)
        .ok()?
        .find(chunk)
        .map(|m| m.as_str().to_string())
}

fn in_container() -> bool {
    Path::new("/run/.containerenv").is_file() || Path::new("/.dockerenv").is_file()
}

fn host_bridge() -> Option<&'static str> {
    for b in ["distrobox-host-exec", "host-spawn"] {
        if Command::new("sh")
            .args(["-c", &format!("command -v {b} >/dev/null 2>&1")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(b);
        }
    }
    None
}

fn require_host() -> bool {
    if !in_container() {
        return true;
    }
    if host_bridge().is_none() {
        eprintln!(
            "require_host: no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine."
        );
        return false;
    }
    true
}

fn host_command(program: &str) -> Command {
    if in_container() {
        if let Some(b) = host_bridge() {
            let mut c = Command::new(b);
            c.arg(program);
            return c;
        }
    }
    Command::new(program)
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn tempfile_dir(prefix: &str) -> Result<PathBuf> {
    let base = std::env::var_os("TMPDIR").unwrap_or_else(|| "/tmp".into());
    let path = PathBuf::from(base).join(format!(
        "{prefix}.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
