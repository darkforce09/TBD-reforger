//! Config rendering — **all three former `python3` heredocs**, plus the profile seed step.
//!
//! `scripts/python-inventory.txt` listed `run-playtest-server.sh` for "backend cfg + admin list
//! JSON". The three sites were:
//!
//! | bash line | what it did | here |
//! |---|---|---|
//! | `:600` | patch `TBD_BackendConfig.json` (missionId / eventId / backendUrl / serverToken) | [`patch_backend_config`] |
//! | `:630` | `json.dumps` the `--admin` list for the echoed summary line | [`admins_json`] |
//! | `:631` | render `server.json` from the dev config | [`render_server_json`] |
//!
//! ── THE TWO FORMATTING DETAILS THAT HAD TO BE REPRODUCED BY HAND ──────────────────────────────
//!
//! `serde_json` is built here with `preserve_order`, so `Map` is an `IndexMap` and a load/store
//! round-trip keeps the operator's key order exactly as CPython's `dict` did. Indent, `": "` and
//! `,\n` already agree with `json.dump(…, indent=2)`. Two things do NOT:
//!
//! 1. **`ensure_ascii` defaults to `True` in Python.** `json.dump` emits `Café`, serde_json
//!    emits the raw UTF-8 bytes. Both parse to the same string, so this is invisible to the engine —
//!    but it is visible to a byte-for-byte diff of the rendered file, and `--name='Café über'` is
//!    an entirely ordinary thing for an operator to pass. `ensure_ascii` reproduces CPython's
//!    escaping, surrogate pairs included. MEASURED against `json.dumps({"n":"café ü"}, indent=2)`.
//! 2. **`json.dumps(list)` with no `indent` uses `', '` as the item separator**, so the admin
//!    summary line reads `admins=["a", "b"]` — with a space. `serde_json::to_string` would emit
//!    `["a","b"]`. That string is echoed to the operator's terminal and is in the baselines, so
//!    [`admins_json`] joins by hand.
//!
//! Neither difference is guessed; both were measured with `python3 -c` on this machine while porting.

use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value};
use verification_core::proc::Run;

use super::{Opts, env_fail};

/// bash: `(cd "$ROOT" && cargo run -q -p xtask -- setup server-profile "$RUN_DIR/profile") >/dev/null`
///
/// ── ONE DELIBERATE DIVERGENCE, NAMED ─────────────────────────────────────────────────────────
///
/// The bash reached the profile seeder through `cargo run`. That makes the playtest launcher report
///
/// ```text
///   ENVIRONMENT: setup server-profile failed
/// ```
///
/// whenever **any crate in the workspace fails to compile**, including crates that have nothing to
/// do with a playtest. MEASURED 2026-08-12 while porting: a concurrent edit to
/// `tools_v2/xtask/src/commands/deploy/staging.rs` left four `mod` declarations without files, and every `--dry-run`
/// arm of the bash script turned into rc 3 with a wall of `E0583` in the middle of its output. The
/// launcher was reporting an ENVIRONMENT verdict about a machine it had never got as far as
/// examining — this repo's signature defect, in the diagnosis rather than the check.
///
/// So this re-invokes **our own binary** (`current_exe`) instead. It is the same code, in the same
/// build, with no compiler in the path, so the rc it returns is genuinely about the profile tree.
/// `setup server-profile` resolves the repo root from its cwd, which is why the cwd is still `root`.
///
/// The hint text still says `cargo run -q -p xtask -- setup server-profile …` because that is the
/// command an operator should type to see the failure for themselves, and it works either way.
pub fn setup_server_profile(root: &Path, run_dir: &str) -> Result<(), u8> {
    let profile = format!("{run_dir}/profile");
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return Err(env_fail(
                &format!("cannot locate this executable to seed the server profile: {e}"),
                &format!(
                    "Run it directly to see why: cargo run -q -p xtask -- setup server-profile {profile}"
                ),
            ));
        }
    };
    let out = Run::new(exe)
        .args(["setup", "server-profile", &profile])
        .cwd(root)
        .output();
    let failed = match out {
        // bash discarded STDOUT (`>/dev/null`) and let stderr through. Same split here: the
        // seeder's success banner is noise inside a boot, its diagnostics are not.
        Ok(o) => {
            let _ = std::io::stderr().write_all(o.stderr.as_bytes());
            o.code != 0
        }
        // Absent / signalled / timed out. bash could only see "non-zero"; we could say more, but the
        // operator-facing contract is the env_fail below, and adding a second message here would
        // diverge from every captured baseline for no diagnostic gain.
        Err(_) => true,
    };
    if failed {
        return Err(env_fail(
            "setup server-profile failed",
            &format!(
                "Run it directly to see why: cargo run -q -p xtask -- setup server-profile {profile}"
            ),
        ));
    }
    Ok(())
}

/// former python3 site 1 of 3 — patch the mod's backend config in place.
///
/// ```python
/// d = json.load(open(p))
/// d["missionId"] = mid
/// d["eventId"] = eid
/// d["backendUrl"] = url
/// if tok:
///     d["serverToken"] = tok
/// json.dump(d, open(p, "w"), indent=2)
/// ```
///
/// PRESERVED: `eventId` is set even when `--event-id` was not given, so the key lands as `""`.
/// `serverToken` is the only conditional one — `setup server-profile` has already substituted the
/// value from `apps/website/api_v2/.env` and an empty `--token` must leave that work alone.
pub fn patch_backend_config(path: &str, o: &Opts) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut d: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let map = d
        .as_object_mut()
        .ok_or_else(|| "backend config is not a JSON object".to_string())?;
    map.insert("missionId".into(), Value::String(o.mission_id.clone()));
    map.insert("eventId".into(), Value::String(o.event_id.clone()));
    map.insert("backendUrl".into(), Value::String(o.backend_url.clone()));
    if !o.token.is_empty() {
        map.insert("serverToken".into(), Value::String(o.token.clone()));
    }
    write_python_json(Path::new(path), &d)
}

/// former python3 site 2 of 3 — the admin list as `json.dumps` would have printed it.
///
/// bash:
/// ```sh
/// ADMINS_JSON="$(printf '%s\n' ${ADMINS+"${ADMINS[@]}"} \
///   | python3 -c 'import json,sys; print(json.dumps([l for l in sys.stdin.read().split("\n") if l]))')"
/// ```
///
/// PRESERVED ODDITY, AND A LATENT BUG IT CARRIES. Routing the array through `printf '%s\n'` and
/// splitting the result on `\n` means the pipeline is not a list of admins — it is a list of LINES.
/// Consequences, both reproduced here:
///
/// * an empty admin is DROPPED rather than emitted as `""` (unreachable in practice: validation
///   rejects `""` first, baseline `a15`);
/// * **an admin containing a newline is SPLIT INTO TWO ENTRIES.** That is reachable, because the
///   validator anchors per line and accepts `"junk\n<valid-uuid>"` (see
///   `admin_id_is_valid`'s note). So a value that passed schema validation as one id reaches
///   `game.admins[]` as two, one of them junk — and the engine's response to a junk admin id is
///   `There are errors in server config!` roughly 90 seconds in, after a full script compile, which
///   is the exact failure the pre-boot validation exists to prevent.
///
/// Reproduced rather than fixed, and pinned by `newline_in_an_admin_splits_it_in_two`: closing it
/// means tightening the validator (a behaviour change on its own baselines), not quietly changing
/// what this function emits.
pub fn admins_json(admins: &[String]) -> String {
    let lines: Vec<&str> = admins
        .iter()
        .flat_map(|a| a.split('\n'))
        .filter(|l| !l.is_empty())
        .collect();
    let items: Vec<String> = lines
        .iter()
        .map(|l| {
            // `serde_json::to_string` of a String is exactly CPython's string escaping for the
            // ASCII range, and `ensure_ascii` covers the rest.
            ensure_ascii(&serde_json::to_string(l).unwrap_or_else(|_| "\"\"".into()))
        })
        .collect();
    // Python's default separator when `indent` is None is `', '` — with the space. MEASURED.
    format!("[{}]", items.join(", "))
}

/// Everything [`render_server_json`] needs. A struct because ten positional arguments is how the
/// bash's `python3 - "$A" "$B" …` call grew to be unreadable.
pub struct ServerJson<'a> {
    pub src: &'a Path,
    pub dst: &'a Path,
    pub ip: &'a str,
    pub port: &'a str,
    pub a2s: &'a str,
    pub max_players: &'a str,
    pub guid: &'a str,
    pub scenario: &'a str,
    pub name: &'a str,
    pub admins: &'a [String],
}

/// former python3 site 3 of 3 — render `server.json` from `tbd-dev-server.config.json`.
///
/// KEY ORDER IS OBSERVABLE and is reproduced exactly. `mods` already exists in the dev config so it
/// is replaced in place; `admins` does NOT, so `dict.__setitem__` appends it AFTER `mods` inside
/// `game`. MEASURED against the real CPython render of the committed dev config.
///
/// `visible` is left **true**: `visible=false` only hides the room from the public browser, the room
/// is registered either way and Direct Join still works — true so the friend can also just find it
/// in the list.
pub fn render_server_json(a: &ServerJson<'_>) -> Result<(), String> {
    let text = std::fs::read_to_string(a.src).map_err(|e| e.to_string())?;
    let mut c: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let top = c
        .as_object_mut()
        .ok_or_else(|| "dev config is not a JSON object".to_string())?;

    top.insert("bindAddress".into(), Value::String("0.0.0.0".into()));
    top.insert("bindPort".into(), int_like(a.port)?);
    top.insert("publicAddress".into(), Value::String(a.ip.to_string()));
    top.insert("publicPort".into(), int_like(a.port)?);

    // python `c.setdefault("a2s", {})["address"] = …` — inserts an empty object at the END when the
    // key is absent, then mutates it. A present-but-not-an-object value is a TypeError in CPython,
    // i.e. rc 1 and "could not render", so it is an error here too.
    setdefault_object(top, "a2s")?.insert("address".into(), Value::String("0.0.0.0".into()));
    setdefault_object(top, "a2s")?.insert("port".into(), int_like(a.a2s)?);

    let g = setdefault_object(top, "game")?;
    g.insert("name".into(), Value::String(a.name.to_string()));
    g.insert("scenarioId".into(), Value::String(a.scenario.to_string()));
    g.insert("maxPlayers".into(), int_like(a.max_players)?);
    g.insert("visible".into(), Value::Bool(true));
    // Same line-splitting semantics as `admins_json`, because the bash fed the very same
    // `ADMINS_JSON` string into this python site via `json.loads(admins)`.
    g.insert(
        "admins".into(),
        Value::Array(
            a.admins
                .iter()
                .flat_map(|s| s.split('\n'))
                .filter(|l| !l.is_empty())
                .map(|l| Value::String(l.to_string()))
                .collect(),
        ),
    );
    let mut mod_entry = Map::new();
    mod_entry.insert("modId".into(), Value::String(a.guid.to_string()));
    mod_entry.insert("name".into(), Value::String("TBD_Framework".into()));
    g.insert("mods".into(), Value::Array(vec![Value::Object(mod_entry)]));

    write_python_json(a.dst, &c)
}

/// python `d.setdefault(k, {})`, returning the object for mutation.
fn setdefault_object<'m>(
    m: &'m mut Map<String, Value>,
    k: &str,
) -> Result<&'m mut Map<String, Value>, String> {
    if !m.contains_key(k) {
        m.insert(k.to_string(), Value::Object(Map::new()));
    }
    m.get_mut(k)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("'{k}' is present but is not a JSON object"))
}

/// python `int(x)` on a value that came off the command line.
///
/// bash never validated `--port` / `--max-players` — nothing between the flag loop and this python
/// site looked at them — so `--port=abc` travelled the whole way here and died with, verbatim:
///
/// ```text
///   Traceback (most recent call last):
///     File "<stdin>", line 5, in <module>
///   ValueError: invalid literal for int() with base 10: 'abc'
///   ERROR: could not render /run/dir/server.json
/// ```
///
/// The `ValueError:` line and the `ERROR:` line are reproduced exactly, and rc stays 1. The two
/// framing lines are the ONLY thing dropped, deliberately: `File "<stdin>", line 5` points into a
/// heredoc that no longer exists, so carrying it over would be fabricating a citation to deleted
/// code — the same class of stale pointer T-606 spent a wave removing from this file's comments.
/// Pinned by `int_like_reproduces_the_python_valueerror_text`.
fn int_like(s: &str) -> Result<Value, String> {
    s.trim()
        .parse::<i64>()
        .map(|n| Value::Number(n.into()))
        .map_err(|_| format!("ValueError: invalid literal for int() with base 10: '{s}'"))
}

/// Write `v` the way `json.dump(v, f, indent=2)` would: 2-space indent, `": "`, no trailing newline,
/// and non-ASCII escaped.
fn write_python_json(path: &Path, v: &Value) -> Result<(), String> {
    let body = ensure_ascii(&serde_json::to_string_pretty(v).map_err(|e| e.to_string())?);
    // NO trailing newline: `json.dump` does not write one, and the committed baselines have none.
    std::fs::write(path, body).map_err(|e| e.to_string())
}

/// CPython's `json` `ensure_ascii=True`: every non-ASCII scalar becomes `\uXXXX`, lowercase hex,
/// astral planes as a UTF-16 surrogate pair.
///
/// Safe to run over a whole serialised document: in JSON, a non-ASCII byte can only occur inside a
/// string literal, so there is nothing else it could corrupt.
fn ensure_ascii(s: &str) -> String {
    if s.is_ascii() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        if ch.is_ascii() {
            out.push(ch);
            continue;
        }
        let cp = ch as u32;
        if cp <= 0xFFFF {
            out.push_str(&format!("\\u{cp:04x}"));
        } else {
            let v = cp - 0x1_0000;
            out.push_str(&format!("\\u{:04x}", 0xD800 + (v >> 10)));
            out.push_str(&format!("\\u{:04x}", 0xDC00 + (v & 0x3FF)));
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/render/tests.rs"]
mod tests;
