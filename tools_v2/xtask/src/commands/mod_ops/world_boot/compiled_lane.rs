use super::*;

/// Seed (or use) a mission through the platform and fetch the artifact its submission compiled:
/// the exact document a deployment of it would run. A live mission's approved artifact is used
/// as it is; a mission under review answers its pending artifact; any other is submitted.
pub(super) fn compiled_lane(
    state: &mut RunState,
    api_base: &str,
    compiled_uuid: &mut Option<String>,
    compiled_artifact: &mut Option<String>,
    mission_path: &mut Option<String>,
    warn_key: &mut String,
) -> std::result::Result<(), GateExit> {
    let transport = CurlTransport::new(state.run_dir.join("api"), 60);
    let token = development_login(&transport, api_base, "mission_maker")
        .map_err(|error| api_env_fail(api_base, &format!("{error:#}"), None))?;
    state.api_token = Some(token.clone());
    let client = ApiClient::new(&transport, api_base, &token);

    let mission_id = match compiled_uuid.as_deref().filter(|id| !id.is_empty()) {
        Some(id) => {
            println!("    using existing mission {id}");
            id.to_string()
        }
        None => {
            let id = create_mission(&client, &seed_fixture_body())
                .map_err(|error| api_doc_fail(&format!("{error:#}")))?;
            println!("    seeded mission {id}");
            *compiled_uuid = Some(id.clone());
            id
        }
    };
    let current = mission(&client, &mission_id).map_err(|error| {
        api_env_fail(
            api_base,
            &format!("{error:#}"),
            Some("Check the mission id you passed."),
        )
    })?;
    let artifact = match (
        current["status"].as_str().unwrap_or_default(),
        current["approved_artifact_id"].as_str(),
    ) {
        ("live", Some(approved)) => approved.to_string(),
        ("pending_approval", _) => pending_review_artifact(&client, &mission_id)
            .map_err(|error| api_doc_fail(&format!("{error:#}")))?,
        _ => submit_mission(&client, &mission_id)
            .map_err(|error| api_doc_fail(&format!("{error:#}")))?,
    };
    let document = artifact_document(&client, &mission_id, &artifact)
        .map_err(|error| api_doc_fail(&format!("{error:#}")))?;
    let compiled_path = state.run_dir.join("compiled.json");
    fs::write(&compiled_path, &document.bytes)
        .map_err(|_| api_env_fail(api_base, "could not write compiled.json", None))?;
    println!(
        "    fetched artifact {artifact}: {} bytes, sha256 {}",
        document.bytes.len(),
        document.sha256
    );
    *compiled_artifact = Some(artifact);
    *mission_path = Some(compiled_path.to_string_lossy().into_owned());
    *warn_key = "compiled".into();
    Ok(())
}

#[rustfmt::skip]
pub(super) fn assert_four_weapon_equip(text: &str, print: bool) -> bool {
    let mut ok = 0usize; let mut bad = 0usize; let mut seen = [false; 4];
    for line in text.lines() {
        let Some(rest) = line.split("[TBD][Equip] slot=").nth(1) else { continue };
        let slot = rest.as_bytes().first().copied().unwrap_or(0).saturating_sub(b'0') as usize;
        if slot < 4 && rest.contains(" weapon=") && rest.contains("result=ok") {
            ok += 1; seen[slot] = true;
        } else if rest.contains("result=") { bad += 1; }
    }
    let pass = ok == EXPECTED_EQUIP_OK && bad == 0 && seen == [true, true, true, true];
    if print {
        if pass { println!("  ok    four-weapon equip ({ok} ok, slots 0-3)"); }
        else { println!("  FAIL  four-weapon equip: ok={ok} other={bad} slots={seen:?} (want ok={EXPECTED_EQUIP_OK} other=0 slots 0-3)"); }
    }
    pass
}

#[rustfmt::skip]
pub(super) fn four_weapon_equip_selftest() -> u8 {
    println!("==> four-weapon equip assertion");
    let g = "[TBD][Equip] slot=0 weapon={3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et result=ok\n[TBD][Equip] slot=1 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=ok\n[TBD][Equip] slot=2 weapon={1353C6EAD1DCFE43}Prefabs/Weapons/Handguns/M9/Handgun_M9.et result=ok\n[TBD][Equip] slot=3 weapon={E8F00BF730225B00}Prefabs/Weapons/Grenades/Grenade_M67.et result=ok\n";
    let mut rc = 0u8;
    if !assert_four_weapon_equip(g, true) { rc = 1; }
    let three: String = g.lines().take(3).collect::<Vec<_>>().join("\n");
    if assert_four_weapon_equip(&three, false) { println!("  FAIL  four-weapon equip selftest accepted 3 weapons"); rc = 1; }
    else { println!("  ok    four-weapon equip selftest rejects 3 weapons"); }
    let r = g.replace("slot=1 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=ok", "slot=0 weapon={9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et result=replaced");
    if assert_four_weapon_equip(&r, false) { println!("  FAIL  four-weapon equip selftest accepted a replaced line"); rc = 1; }
    else { println!("  ok    four-weapon equip selftest rejects replaced"); }
    rc
}

#[rustfmt::skip]
pub(super) fn seed_fixture_body() -> Value {
    // The four-weapon proof lives on sl_ar: the unarmed base kit, so each weapon row inserts
    // (result=ok) instead of replacing a kit weapon.
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
    v
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

/// Delete the fixture missions the compiled lane created: the caller's own missions with the
/// fixture's title.
pub(super) fn sweep_fixture_missions(run_dir: &Path, api_base: &str, token: Option<&str>) {
    let Some(token) = token else {
        return;
    };
    let transport = CurlTransport::new(run_dir.join("api"), 10);
    let client = ApiClient::new(&transport, api_base, token);
    for id in own_missions_titled(&client, FIXTURE_TITLE).unwrap_or_default() {
        let _ = delete_mission(&client, &id);
    }
}
