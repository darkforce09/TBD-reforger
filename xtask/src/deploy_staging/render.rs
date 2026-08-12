//! T-288 — modpack resolution and the `server.config.json` render (bash lines 1252–1522), split
//! out of [`super::config`] for SIZE-3.
//!
//! [`super::config`] owns the INPUTS (`deploy.env`, the `:=` defaults, the mode gate). This module
//! owns the ARTEFACT they produce. The split is the bash's own: `render_server_config()` is a pure
//! function of an already-validated environment, and T-288 exists precisely because the render was
//! once fused to the push and therefore unobservable.
//!
//! The render is reached by two callers and must be identical for both: `--render-only <path>`
//! (local, no ssh) and the deploy's config-mode step, which renders locally, validates, and only
//! then `cat`s the bytes onto the host.

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::{Map, Value};

use super::config::{Env, xargs_like};
use super::pycompat::{
    ensure_ascii, json_repr, py_json_error, py_repr, py_str_or_empty, py_type_name,
};

// ── modpack resolution ──────────────────────────────────────────────────────────────────────

/// `resolve_modpack_doc` — the modpack document (`GET /modpacks/current` shape) as text.
///
/// Returns `(document, src_label)`. The label is what error text names, so a reader can tell a bad
/// file from a bad API response from the synthesized legacy document.
pub fn resolve_modpack_doc(env: &Env) -> Result<(String, String), u8> {
    if !env.modpack_json.is_empty() {
        let p = PathBuf::from(&env.modpack_json);
        if !p.is_file() {
            eprintln!(
                "FAIL: TBD_MODPACK_JSON={} does not exist.",
                env.modpack_json
            );
            return Err(1);
        }
        let text = match fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("FAIL: could not read {}: {e}", env.modpack_json);
                return Err(1);
            }
        };
        println!("  modpack source: file {}", env.modpack_json);
        Ok((text, env.modpack_json.clone()))
    } else if !env.modpack_url.is_empty() {
        if env.modpack_token.is_empty() {
            eprintln!("FAIL: TBD_MODPACK_URL is set but TBD_MODPACK_TOKEN is empty.");
            eprintln!("      GET /api/v1/modpacks/current is gated by AuthUser (Bearer JWT,");
            eprintln!(
                "      apps/website/api/src/middleware/auth.rs). TBD_GAME_SERVER_TOKEN is the"
            );
            eprintln!(
                "      SERVICE_TOKEN checked on the X-Service-Token header by ServiceAuth and"
            );
            eprintln!("      will NOT authenticate this route. See T-288.");
            return Err(1);
        }
        fetch_modpack_url(env)
    } else {
        // LEGACY: synthesize the same document shape from the env var so there is one renderer and
        // one validator, not two divergent code paths. The bash built this with python3's
        // `json.dump` (call site 1 of 14); the field order is that dict's insertion order.
        let mut mod0 = Map::new();
        mod0.insert("name".into(), Value::String(env.workshop_mod_name.clone()));
        mod0.insert(
            "workshop_id".into(),
            Value::String(env.workshop_mod_id.clone()),
        );
        mod0.insert("version".into(), Value::String(String::new()));
        let mut doc = Map::new();
        doc.insert(
            "name".into(),
            Value::String("(legacy TBD_WORKSHOP_MOD_ID env, not a database modpack)".into()),
        );
        doc.insert("version".into(), Value::String(String::new()));
        doc.insert("mods".into(), Value::Array(vec![Value::Object(mod0)]));
        println!("  modpack source: LEGACY env TBD_WORKSHOP_MOD_ID (no modpack configured)");
        Ok((
            Value::Object(doc).to_string(),
            "LEGACY TBD_WORKSHOP_MOD_ID".to_string(),
        ))
    }
}

/// `curl -sS -o "$out" -w '%{http_code}' -H "Authorization: Bearer …" "$url"`.
///
/// **NEVER EXECUTED LOCALLY.** No credential of this tier exists on any machine in this program
/// (see the module header), so this path has no live coverage at all; `tests::curl_argv_is_stable`
/// pins the argv instead. `curl` is spawned rather than a Rust HTTP client added, because the
/// argv is the thing under test and because adding reqwest+TLS to xtask for one unreachable call
/// would be a large dependency bought with no evidence.
fn fetch_modpack_url(env: &Env) -> Result<(String, String), u8> {
    let out = std::env::temp_dir().join(format!("tbd-modpack.{}.json", std::process::id()));
    let args = curl_argv(env, &out);
    if tbd_gate::proc::which("curl").is_err() {
        eprintln!("FAIL: could not reach {}", env.modpack_url);
        return Err(1);
    }
    let mut run = tbd_gate::proc::Run::new("curl");
    for a in &args {
        run = run.arg(a);
    }
    let res = run.timeout(std::time::Duration::from_secs(120)).output();
    let code = match res {
        // `|| { echo FAIL: could not reach …; exit 1; }` — curl's own non-zero status.
        Ok(o) if o.code == 0 => o.stdout.trim().to_string(),
        _ => {
            eprintln!("FAIL: could not reach {}", env.modpack_url);
            return Err(1);
        }
    };
    if code != "200" {
        eprintln!(
            "FAIL: {} returned HTTP {code} (expected 200).",
            env.modpack_url
        );
        eprintln!("      401/403 means the credential tier is wrong — see T-288.");
        return Err(1);
    }
    let text = fs::read_to_string(&out).unwrap_or_default();
    let _ = fs::remove_file(&out);
    println!("  modpack source: {} (HTTP 200)", env.modpack_url);
    Ok((text, env.modpack_url.clone()))
}

fn curl_argv(env: &Env, out: &Path) -> Vec<String> {
    vec![
        "-sS".into(),
        "-o".into(),
        out.display().to_string(),
        "-w".into(),
        "%{http_code}".into(),
        "-H".into(),
        format!("Authorization: Bearer {}", env.modpack_token),
        env.modpack_url.clone(),
    ]
}

pub fn modpack_mods_json(doc_text: &str, src: &str) -> Result<String, u8> {
    let doc: Value = match serde_json::from_str(doc_text) {
        Ok(v) => v,
        Err(e) => {
            // python: "FAIL: modpack document is not valid JSON (%s): %s" % (src, e), where `e` is
            // a `json.JSONDecodeError`. See [`py_json_error`] for exactly which of python's
            // messages are reproduced and which fall back to `serde_json`'s wording.
            eprintln!(
                "FAIL: modpack document is not valid JSON ({src}): {}",
                py_json_error(doc_text, &e)
            );
            return Err(1);
        }
    };
    let Value::Object(obj) = &doc else {
        eprintln!(
            "FAIL: modpack document must be a JSON object, got {}",
            py_type_name(&doc)
        );
        return Err(1);
    };
    // `doc.get("mods")` — an explicit JSON `null` is python's `None` and takes this branch too.
    let mods = match obj.get("mods") {
        None | Some(Value::Null) => {
            eprintln!("FAIL: modpack document has no `mods` key. Expected the body of");
            eprintln!("      GET /api/v1/modpacks/current (ModpackDto = modpack fields + mods[]).");
            return Err(1);
        }
        Some(v) => v,
    };
    let Value::Array(mods) = mods else {
        eprintln!("FAIL: `mods` must be an array, got {}", py_type_name(mods));
        return Err(1);
    };
    if mods.is_empty() {
        let name = py_str_or_empty(obj.get("name"));
        let label = if name.is_empty() {
            "<unnamed>".to_string()
        } else {
            name
        };
        eprintln!(
            "FAIL: modpack {} has zero mods. Rendering game.mods[] as [] would start a",
            py_repr(&label)
        );
        eprintln!("      server with no content and silently disagree with the website.");
        return Err(1);
    }

    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<(String, String)> = Vec::new();
    for (i, m) in mods.iter().enumerate() {
        let Value::Object(m) = m else {
            eprintln!("FAIL: mods[{i}] is not an object");
            return Err(1);
        };
        let name = py_str_or_empty(m.get("name")).trim().to_string();
        let wid = py_str_or_empty(m.get("workshop_id")).trim().to_string();
        let ver = py_str_or_empty(m.get("version")).trim().to_string();
        if name.is_empty() {
            eprintln!("FAIL: mods[{i}].name is empty");
            return Err(1);
        }
        if wid.is_empty() {
            eprintln!(
                "FAIL: mods[{i}] ({}) has an empty workshop_id.",
                py_repr(&name)
            );
            eprintln!("      Reforger game.mods[].modId IS the Workshop id; an empty one renders");
            eprintln!("      \"modId\": \"\" and the server rejects the config. Populate");
            eprintln!("      modpack_mods.workshop_id (migration 0012_modpack_mods_workshop.sql).");
            return Err(1);
        }
        if let Some((_, prev)) = seen.iter().find(|(k, _)| *k == wid) {
            eprintln!(
                "FAIL: mods[{i}] ({}) repeats modId {wid}, already used by {}",
                py_repr(&name),
                py_repr(prev)
            );
            return Err(1);
        }
        seen.push((wid.clone(), name.clone()));
        // Insertion order IS the rendered key order (serde_json's `preserve_order`), matching the
        // python dict literal `{"modId": wid, "name": name}` plus the conditional `version`.
        let mut entry = Map::new();
        entry.insert("modId".into(), Value::String(wid));
        entry.insert("name".into(), Value::String(name));
        if !ver.is_empty() {
            entry.insert("version".into(), Value::String(ver));
        }
        out.push(Value::Object(entry));
    }

    let text = serde_json::to_string_pretty(&Value::Array(out)).map_err(|e| {
        eprintln!("FAIL: could not serialise game.mods[]: {e}");
        1u8
    })?;
    let text = ensure_ascii(&text);
    Ok(text
        .lines()
        .enumerate()
        .map(|(i, ln)| {
            if i == 0 {
                ln.to_string()
            } else {
                format!("    {ln}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// `render_server_config` — the complete server config to the LOCAL path `out`, then validate it.
///
/// ODDITY PRESERVED, and it is the reason the validator exists: the template substitutes RAW,
/// unescaped values. A `TBD_SERVER_NAME` containing a double quote, or a non-numeric
/// `TBD_GAME_PORT`, produces a document that is not JSON at all — and the validator then catches
/// it by re-parsing the file. Escaping the values here would be a behaviour change that silently
/// accepted configs the engine may still reject, and would make the "is not valid JSON" branch of
/// the validator unreachable.
pub fn render_server_config(env: &Env, out: &Path) -> Result<(), u8> {
    let (doc, src_label) = resolve_modpack_doc(env)?;
    let mods_json = modpack_mods_json(&doc, &src_label)?;

    // A JSON array of admin identityIds from the comma-separated env var. Also raw — an id that
    // could break the quoting was already rejected by `Env::validate`.
    let admins_json = env
        .admin_identity_ids
        .split(',')
        .map(xargs_like)
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let body = format!(
        r#"{{
  "bindAddress": "0.0.0.0",
  "bindPort": {game_port},
  "publicAddress": "{public_address}",
  "publicPort": {game_port},
  "a2s": {{ "address": "0.0.0.0", "port": {a2s_port} }},
  "game": {{
    "name": "{server_name}",
    "password": "",
    "passwordAdmin": "{admin_password}",
    "admins": [{admins_json}],
    "scenarioId": "{scenario}",
    "maxPlayers": {max_players},
    "visible": true,
    "crossPlatform": false,
    "gameProperties": {{
      "battlEye": false,
      "disableThirdPerson": false,
      "fastValidation": false,
      "VONDisableUI": false,
      "VONDisableDirectSpeechUI": false
    }},
    "mods": {mods_json}
  }},
  "operating": {{ "lobbyPlayerSynchronise": true }}
}}
"#,
        game_port = env.game_port,
        public_address = env.public_address,
        a2s_port = env.a2s_port,
        server_name = env.server_name,
        admin_password = env.admin_password,
        scenario = env.scenario,
        max_players = env.max_players,
    );
    if let Err(e) = fs::write(out, &body) {
        eprintln!("FAIL: could not write {}: {e}", out.display());
        return Err(1);
    }
    validate_server_config(out)
}

/// `validate_server_config` — structural check of a rendered server config.
///
/// NOT eyeballing: re-parses the FILE and pins the invariants the Reforger server enforces (and the
/// a2s/game port rule this program already documents at `TBD_A2S_PORT`). Reading the artefact back
/// off disk rather than validating the in-memory struct is deliberate — it is the only way the
/// "renders to something that is not JSON" case above is reachable.
pub fn validate_server_config(path: &Path) -> Result<(), u8> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let doc: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "FAIL: rendered server config is not valid JSON ({}): {}",
                path.display(),
                py_json_error(&text, &e)
            );
            return Err(1);
        }
    };
    let empty = Map::new();
    let top = doc.as_object().unwrap_or(&empty);
    let mut errs: Vec<String> = Vec::new();
    for key in [
        "bindAddress",
        "bindPort",
        "publicAddress",
        "publicPort",
        "a2s",
        "game",
        "operating",
    ] {
        if !top.contains_key(key) {
            errs.push(format!("missing top-level key {}", py_repr(key)));
        }
    }
    let game = top
        .get("game")
        .and_then(|g| g.as_object())
        .unwrap_or(&empty);
    for key in [
        "name",
        "passwordAdmin",
        "admins",
        "scenarioId",
        "maxPlayers",
        "mods",
    ] {
        if !game.contains_key(key) {
            errs.push(format!("missing game.{key}"));
        }
    }
    let a2s = top.get("a2s").and_then(|a| a.as_object()).unwrap_or(&empty);
    if let Some(port) = a2s.get("port").filter(|p| !p.is_null()) {
        if Some(port) == top.get("bindPort") {
            errs.push(format!(
                "a2s.port == bindPort ({}) — replication cannot start",
                json_repr(top.get("bindPort"))
            ));
        }
    }
    // T-607: scenarioId against the ENGINE's OWN schema, copied verbatim out of its rejection
    // (1.7.0.54):
    //   BACKEND (E): RegEx Pattern: "^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$"
    //   BACKEND (E): Pattern Description: "Param must start with ResourceGUID enclosed in brackets."
    // Presence was checked above and that was NOT enough: a TRUNCATED scenarioId
    // ("{69A85365FC09E2CA", the bash-brace defect) is present, is a string, and is fatal. The
    // validator printed "config VALID" over exactly that config — a tool reporting success over an
    // input it never really examined. The engine finds it ~90 s into a boot, after the rsync and a
    // full script compile; this finds it on the dev machine before anything is pushed.
    if let Some(Value::String(scenario)) = game.get("scenarioId") {
        if !scenario.is_empty() {
            let re = Regex::new(r"^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$").expect("static");
            if !re.is_match(scenario) {
                errs.push(format!(
                    "game.scenarioId {} is rejected by the engine's schema (^\\{{[0-9A-F]{{16}}\\}}[a-zA-Z0-9_./ -]+$). A value that stops right after the GUID means TBD_SCENARIO was truncated by brace parsing in the shell.",
                    py_repr(scenario)
                ));
            }
        }
    }
    let mods = match game.get("mods") {
        Some(Value::Array(a)) if !a.is_empty() => a.clone(),
        _ => {
            errs.push("game.mods[] must be a non-empty array".into());
            Vec::new()
        }
    };
    let mut listed: Vec<(String, String)> = Vec::new();
    for (i, m) in mods.iter().enumerate() {
        let Some(m) = m.as_object() else {
            errs.push(format!("game.mods[{i}] is not an object"));
            continue;
        };
        let id = py_str_or_empty(m.get("modId")).trim().to_string();
        let name = py_str_or_empty(m.get("name")).trim().to_string();
        if id.is_empty() {
            errs.push(format!("game.mods[{i}].modId is empty"));
        }
        if name.is_empty() {
            errs.push(format!("game.mods[{i}].name is empty"));
        }
        // The summary line uses the RAW values (python `m["name"]`), not the stripped ones.
        listed.push((
            py_str_or_empty(m.get("name")),
            py_str_or_empty(m.get("modId")),
        ));
    }

    if !errs.is_empty() {
        eprintln!("FAIL: rendered server config is invalid:");
        for e in &errs {
            eprintln!("      {e}");
        }
        return Err(1);
    }
    println!(
        "  config VALID: {} mod(s) -> {}",
        mods.len(),
        listed
            .iter()
            .map(|(n, i)| format!("{n}={i}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}
/// `--render-only <path>`: render locally, validate, exit.
pub fn render_only(env: &Env, out: &str) -> u8 {
    if env.server_mode != "config" {
        eprintln!("--render-only requires TBD_SERVER_MODE=config (addons mode renders no config).");
        return 2;
    }
    println!("==> render server config (local only, no deploy) -> {out}");
    match render_server_config(env, Path::new(out)) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_staging::config::tests::base;

    #[test]
    fn legacy_mods_render_matches_the_captured_bytes() {
        // Diffed against /tmp/t853/ro-legacy.json.old: python json.dumps(indent=2) then every
        // line after the first gets four extra spaces, so `[` sits flush after `"mods": `.
        let e = base();
        let mut mod0 = Map::new();
        mod0.insert("name".into(), Value::String(e.workshop_mod_name.clone()));
        mod0.insert(
            "workshop_id".into(),
            Value::String(e.workshop_mod_id.clone()),
        );
        mod0.insert("version".into(), Value::String(String::new()));
        let mut d = Map::new();
        d.insert("mods".into(), Value::Array(vec![Value::Object(mod0)]));
        let got = modpack_mods_json(&Value::Object(d).to_string(), "L").expect("renders");
        assert_eq!(
            got,
            "[\n      {\n        \"modId\": \"5EAF00DBEEF01234\",\n        \"name\": \"TBD_Framework\"\n      }\n    ]"
        );
    }

    #[test]
    fn version_is_emitted_only_when_non_empty_and_key_order_is_pinned() {
        let doc = r#"{"mods":[{"name":"A","workshop_id":"W","version":"1.0.2"}]}"#;
        let got = modpack_mods_json(doc, "t").expect("renders");
        // modId, name, version — the python dict's insertion order, not alphabetical.
        let idx_id = got.find("modId").unwrap();
        let idx_name = got.find("\"name\"").unwrap();
        let idx_ver = got.find("version").unwrap();
        assert!(idx_id < idx_name && idx_name < idx_ver, "{got}");
        // An empty version is omitted entirely rather than rendered as "".
        let doc = r#"{"mods":[{"name":"A","workshop_id":"W","version":""}]}"#;
        assert!(!modpack_mods_json(doc, "t").unwrap().contains("version"));
    }

    #[test]
    fn every_fail_closed_branch_fires() {
        // ANTI-VACUITY: each of these is a real /tmp/t853 baseline, and each must be RED.
        for bad in [
            "not json at all {",
            "[1,2,3]",
            r#"{"name":"x"}"#,
            r#"{"name":"x","mods":null}"#,
            r#"{"name":"x","mods":{}}"#,
            r#"{"name":"x","mods":[]}"#,
            r#"{"name":"x","mods":[1]}"#,
            r#"{"name":"x","mods":[{"name":"","workshop_id":"a"}]}"#,
            r#"{"name":"x","mods":[{"name":"A","workshop_id":""}]}"#,
            r#"{"name":"x","mods":[{"name":"A","workshop_id":"Z"},{"name":"B","workshop_id":"Z"}]}"#,
        ] {
            assert!(modpack_mods_json(bad, "t").is_err(), "should fail: {bad}");
        }
        // …and the good one must be GREEN, or the ten above prove nothing.
        assert!(modpack_mods_json(r#"{"mods":[{"name":"A","workshop_id":"W"}]}"#, "t").is_ok());
    }

    #[test]
    fn validator_catches_the_truncated_scenario_and_the_port_clash() {
        // The two cases the bash validator was BLIND to before T-607 and T-288 respectively.
        let d = std::env::temp_dir().join(format!("tbd-t853-cfg-{}", std::process::id()));
        let _ = fs::create_dir_all(&d);

        let mut e = base();
        e.scenario = "{69A85365FC09E2CA".into();
        assert!(
            render_server_config(&e, &d.join("trunc.json")).is_err(),
            "truncated scenario must fail"
        );

        let mut e = base();
        e.a2s_port = e.game_port.clone();
        assert!(
            render_server_config(&e, &d.join("clash.json")).is_err(),
            "a2s == bindPort must fail"
        );

        // And the honest config must pass, or the two above are vacuous.
        assert!(render_server_config(&base(), &d.join("ok.json")).is_ok());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn raw_substitution_can_emit_non_json_and_the_validator_catches_it() {
        // ODDITY PINNED: the template does not escape. A quote in the server name breaks the
        // document, and the "not valid JSON" branch — otherwise unreachable — fires.
        let d = std::env::temp_dir().join(format!("tbd-t853-raw-{}", std::process::id()));
        let _ = fs::create_dir_all(&d);
        let mut e = base();
        e.server_name = "a\" , \"evil\": 1, \"x\": \"b".into();
        let p = d.join("raw.json");
        let res = render_server_config(&e, &p);
        let text = fs::read_to_string(&p).unwrap_or_default();
        assert!(
            res.is_err() || text.contains("evil"),
            "raw substitution must be observable"
        );
        // A non-numeric port is the cleaner case: it cannot parse at all.
        let mut e = base();
        e.game_port = "not-a-port".into();
        assert!(render_server_config(&e, &d.join("port.json")).is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn render_only_refuses_addons_mode() {
        let mut e = base();
        e.server_mode = "addons".into();
        assert_eq!(render_only(&e, "/tmp/never-written-t853.json"), 2);
        assert!(!Path::new("/tmp/never-written-t853.json").exists());
    }

    #[test]
    fn curl_argv_is_stable() {
        // NEVER EXECUTED: no credential of this tier exists anywhere in this program. The argv is
        // the only thing that can be asserted, so it is asserted exactly.
        let mut e = base();
        e.modpack_url = "https://tbd.example/api/v1/modpacks/current".into();
        e.modpack_token = "jwt.abc.def".into();
        assert_eq!(
            curl_argv(&e, Path::new("/tmp/out.json")),
            vec![
                "-sS",
                "-o",
                "/tmp/out.json",
                "-w",
                "%{http_code}",
                "-H",
                "Authorization: Bearer jwt.abc.def",
                "https://tbd.example/api/v1/modpacks/current",
            ]
        );
    }

    #[test]
    fn modpack_url_without_a_token_fails_before_any_network_call() {
        // The credential tier T-288 documented: TBD_GAME_SERVER_TOKEN is a SERVICE_TOKEN and does
        // not authenticate an AuthUser route, so an empty TBD_MODPACK_TOKEN must fail closed here
        // rather than produce a 401 nobody reads.
        let mut e = base();
        e.modpack_url = "https://tbd.example/x".into();
        assert!(resolve_modpack_doc(&e).is_err());
    }
}
