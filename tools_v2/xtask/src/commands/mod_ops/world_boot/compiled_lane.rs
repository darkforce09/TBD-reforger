use super::*;

pub(super) fn compiled_lane(
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
            "no SERVICE_TOKEN — set TBD_SERVICE_TOKEN, or add it to apps/website/api_v2/.env",
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

#[rustfmt::skip]
pub(super) fn t302_assert(text: &str, print: bool) -> bool {
    let mut ok = 0usize; let mut bad = 0usize; let mut seen = [false; 4];
    for line in text.lines() {
        let Some(rest) = line.split("[TBD][Equip] slot=").nth(1) else { continue };
        let slot = rest.as_bytes().first().copied().unwrap_or(0).saturating_sub(b'0') as usize;
        if slot < 4 && rest.contains(" weapon=") && rest.contains("result=ok") {
            ok += 1; seen[slot] = true;
        } else if rest.contains("result=") { bad += 1; }
    }
    let pass = ok == T302_EQUIP_OK && bad == 0 && seen == [true, true, true, true];
    if print {
        if pass { println!("  ok    T-302 four-weapon equip ({ok} ok, slots 0-3)"); }
        else { println!("  FAIL  T-302 four-weapon equip: ok={ok} other={bad} slots={seen:?} (want ok={T302_EQUIP_OK} other=0 slots 0-3)"); }
    }
    pass
}

#[rustfmt::skip]
pub(super) fn t302_selftest() -> u8 {
    println!("==> T-302 four-weapon equip assertion");
    let g = "[TBD][Equip] slot=0 weapon={3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et result=ok\n[TBD][Equip] slot=1 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=ok\n[TBD][Equip] slot=2 weapon={1353C6EAD1DCFE43}Prefabs/Weapons/Handguns/M9/Handgun_M9.et result=ok\n[TBD][Equip] slot=3 weapon={E8F00BF730225B00}Prefabs/Weapons/Grenades/Grenade_M67.et result=ok\n";
    let mut rc = 0u8;
    if !t302_assert(g, true) { rc = 1; }
    let three: String = g.lines().take(3).collect::<Vec<_>>().join("\n");
    if t302_assert(&three, false) { println!("  FAIL  T-302 selftest accepted 3 weapons"); rc = 1; }
    else { println!("  ok    T-302 selftest rejects 3 weapons"); }
    let r = g.replace("slot=1 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=ok", "slot=0 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=replaced");
    if t302_assert(&r, false) { println!("  FAIL  T-302 selftest accepted a replaced line"); rc = 1; }
    else { println!("  ok    T-302 selftest rejects replaced"); }
    rc
}

#[rustfmt::skip]
pub(super) fn seed_fixture_body() -> String {
    // T-302 four-weapon proof lives on sl_ar (Unarmed so each row inserts, result=ok).
    let v = json!({
        "title": FIXTURE_TITLE, "terrain": "everon", "game_mode": "pvp", "weather": "clear",
        "time_of_day": "05:30", "max_players": 8,
        "briefing": "Generated by cargo xtask mod world-boot --compiled. Safe to delete.",
        "payload": { "schemaVersion": 1, "editor": {
            "factions": [{ "key": "blufor", "name": "US Army", "squadIds": ["sq_alpha"] }],
            "squads": [{ "id": "sq_alpha", "callsign": "Alpha", "name": "Alpha", "slotIds": ["sl_sl", "sl_ar", "sl_rfl"] }],
            "slots": [
                { "id": "sl_sl", "index": 0, "role": "SL",
                  "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",
                  "position": { "x": 4870.0, "y": 7760.0, "z": 0.0, "rotation": 45.0 } },
                { "id": "sl_ar", "index": 1, "role": "AR",
                  "assetId": "{2F912ED6E399FF47}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Unarmed.et",
                  "position": { "x": 4880.0, "y": 7770.0, "z": 0.0, "rotation": 90.0 },
                  "loadout": {
                    "wear": {
                      "jacket": "{293F577C298061E3}Prefabs/Characters/Uniforms/Jacket_US_BDU_02.et",
                      "armoredVest": "{477A190AF2A17B8A}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_MG.et",
                      "headCover": "{B74A4FF0DD8BB116}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01.et",
                      "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
                      "boots": "{DAAFD15478BDE1C3}Prefabs/Characters/Footwear/CombatBoots_US_01.et"
                    },
                    "weapons": [
                      { "slotIndex": 0, "slotType": "primary",
                        "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                        "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et" },
                      { "slotIndex": 1, "slotType": "primary",
                        "weapon": "{9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et" },
                      { "slotIndex": 2, "slotType": "secondary",
                        "weapon": "{1353C6EAD1DCFE43}Prefabs/Weapons/Handguns/M9/Handgun_M9.et" },
                      { "slotIndex": 3, "slotType": "grenade",
                        "weapon": "{E8F00BF730225B00}Prefabs/Weapons/Grenades/Grenade_M67.et" }
                    ],
                    "cargo": [{ "container": "vest",
                      "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et", "qty": 6 }]
                  } },
                { "id": "sl_rfl", "index": 2, "role": "RFL",
                  "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
                  "position": { "x": 4890.0, "y": 7780.0, "z": 136.0, "rotation": 315.0 } }
            ]
        } }
    });
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}

pub(super) fn write_server_json(
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

pub(super) fn spawn_server(server_dir: &Path, run_dir: &Path, max_wait: u64) -> Result<Child> {
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

pub(super) fn poll_for_log(
    run_dir: &Path,
    max_wait: u64,
    mission: Option<&str>,
) -> Option<PathBuf> {
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

pub(super) fn latest_console_log(run_dir: &Path) -> Option<PathBuf> {
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

pub(super) fn kill_run(pidfile: &Path) {
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

pub(super) fn sweep_fixture_missions(
    run_dir: &Path,
    api_base: &str,
    svc: Option<&str>,
    dev: Option<&str>,
) {
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
