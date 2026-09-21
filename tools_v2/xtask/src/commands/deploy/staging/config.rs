//! T-288 — deploy.env, modpack resolution and the `server.config.json` render half
//! (bash lines 1072–1522).
//!
//! ── THE RENDER IS A PURE FUNCTION THAT WRITES A FILE ─────────────────────────────────────────
//!
//! The push is a separate step that copies that file. Before T-288 the two were fused into one
//! `ssh_cmd "cat > remote" <<EOF` heredoc, which meant the only way to see what this script
//! produces was to deploy it to a live server — so nothing ever checked it.
//!
//! ── WHERE `game.mods[]` COMES FROM ───────────────────────────────────────────────────────────
//!
//! Before T-288 this script hardcoded ONE mod — `{"modId": "$TBD_WORKSHOP_MOD_ID", "name":
//! "TBD_Framework"}` — and never read the `modpacks` / `modpack_mods` tables. A modpack authored
//! on the website therefore had no path to a running server.
//!
//! THE SOURCE IS THE API, and specifically the bytes of `GET /api/v1/modpacks/current`
//! (`apps/website/api_v2/src/core/http_router.rs` → `handlers/modpacks.rs::get_current_modpack`) whose `mods[]`
//! rows carry exactly the fields a Reforger `game.mods[]` entry needs — `workshop_id`, `mod_guid`,
//! `version` — added by T-271 in `migrations/0012_modpack_mods_workshop.sql`, whose header says
//! verbatim: "keep both so a future renderer (T-288) can choose".
//!
//! REJECTED — reading Postgres directly: this is not a DB client (no `DATABASE_URL` in
//! `deploy.env.example`), the database lives inside docker compose on the remote host, and
//! hand-rolling the projection would duplicate the null-tolerant COALESCE read in
//! `handlers/modpacks.rs mod_cols!()`. The next migration would break the renderer silently.
//! REJECTED — inventing a modpack file format of our own: that IS the defect T-288 removed.
//!
//! ⚠ THE CREDENTIAL DOES NOT EXIST YET. `/modpacks/current` is gated by `AuthUser`, a **Bearer
//! JWT** minted from a Discord login (`middleware/auth.rs`). This program's only secret is
//! `TBD_GAME_SERVER_TOKEN`, which is `SERVICE_TOKEN` and is checked by `ServiceAuth` against the
//! **X-Service-Token** header — a different auth tier, and no ServiceAuth-guarded modpack read
//! exists. So `TBD_MODPACK_URL` cannot be satisfied by anything the deploy host holds today; it is
//! wired and fails closed, ready for the day a service-token modpack read (or a deploy JWT) ships.
//!
//! * `TBD_MODPACK_JSON` — path to a file holding a `GET /modpacks/current` response body. Works
//!   TODAY, and is the supported path right now.
//! * `TBD_MODPACK_URL` — fetch that same document over HTTP. Needs `TBD_MODPACK_TOKEN`.
//! * Neither → the single-mod env fallback `TBD_WORKSHOP_MOD_ID`, which goes through the
//!   SAME renderer and the SAME validator, so there is exactly one place that can emit
//!   `game.mods[]`.
//!
//! ── THE FOURTEEN `python3` CALL SITES ────────────────────────────────────────────────────────
//!
//! The bash reached for python3 because "`jq` is NOT installed here (measured) and hand-rolled
//! JSON in bash silently emits invalid documents". `serde_json` is compiled in, so
//! `require_python3()` — a preflight that existed only to name a dependency this port does not
//! have — is DELETED rather than translated. That is a dependency removed, not asserted.
//!
//! Two python behaviours are load-bearing for byte parity and are reproduced deliberately:
//! `json.dumps(..., ensure_ascii=True)` (see `ensure_ascii`) and `%r` string formatting (see
//! `py_repr`). Getting either wrong would change error text a wave log greps for.

use std::fs;
use std::path::Path;

use regex::Regex;

/// Every setting the deploy reads, after `deploy.env` and the `:=` defaults have been applied.
#[derive(Debug, Clone)]
pub struct Env {
    pub ssh_host: String,
    pub remote_dir: String,
    pub profile_dir: String,
    pub addons_staging: String,
    pub game_server_token: String,
    pub mission_id: String,
    pub event_id: String,
    pub backend_url: String,
    pub addon_guid: String,
    pub scenario: String,
    /// `TBD_BIND_IP`. Its ONLY consumer is the `TBD_PUBLIC_ADDRESS` default, which is resolved in
    /// [`Env::load`] — the addons-mode ExecStart hardcodes `-bindIP 0.0.0.0` and never reads this.
    /// Kept as a field anyway: it is a documented deploy.env knob, and dropping it from the struct
    /// would hide from the next reader that the resolved value is observable.
    #[allow(dead_code)]
    pub bind_ip: String,
    pub server_dir: String,
    pub server_mode: String,
    pub workshop_mod_id: String,
    pub public_address: String,
    pub game_port: String,
    pub a2s_port: String,
    pub server_name: String,
    pub admin_password: String,
    pub max_players: String,
    pub admin_identity_ids: String,
    pub server_config_remote: String,
    pub boot_verify_timeout: String,
    pub modpack_json: String,
    pub modpack_url: String,
    pub modpack_token: String,
    pub workshop_mod_name: String,
    pub run_t092_smoke: bool,
    pub ssh_pass: Option<String>,
    pub ssh_identity_file: Option<String>,
}

/// KEY=VALUE parser. **Not** a shell `source`.
///
/// FAIL-OPEN CLOSED (1 of 3). The bash `source`d this file, i.e. EXECUTED it. A syntax error
/// aborted under `set -e`, but a stray command in it ran silently with the deploy's privileges and
/// its network reach. Nothing about the deploy needs shell in a config file. Same call as
/// `gate_deploy_website.rs` made for `deploy-website.sh`.
///
/// The consequence to keep in mind: `export FOO=$(hostname)` used to work and now yields the
/// literal text. No committed `deploy.env.example` line uses substitution, and a value that is
/// really a command belongs in the script.
fn parse_deploy_env(path: &Path) -> Result<Vec<(String, String)>, u8> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("FAIL: could not read {}: {e}", path.display());
            return Err(1);
        }
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            out.push((k.trim().to_string(), v));
        }
    }
    Ok(out)
}

/// `dirname`, POSIX. Strip trailing slashes, drop the last component, and answer `.` for a bare
/// name. Used only for the `TBD_SERVER_CONFIG_REMOTE` default; measured against coreutils:
/// `/p/q`→`/p`, `/p`→`/`, `p`→`.`, `/p/q/`→`/p`.
fn dirname(p: &str) -> String {
    let s = p.trim_end_matches('/');
    if s.is_empty() {
        return if p.starts_with('/') {
            "/".into()
        } else {
            ".".into()
        };
    }
    match s.rfind('/') {
        None => ".".into(),
        Some(0) => "/".into(),
        Some(i) => s[..i].to_string(),
    }
}

/// `echo "$x" | xargs` — trim and collapse runs of whitespace to a single space.
///
/// ODDITY NOTE: real `xargs` also interprets quotes, so `echo "a'b" | xargs` *errors* and the
/// bash's `$(...)` would have yielded the empty string. That input then failed the identityId
/// regex anyway, so the two implementations reach the same verdict by different routes; the
/// difference is only in which error text a reader sees. Not worth reproducing an `xargs` parser
/// for.
pub(super) fn xargs_like(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Env {
    /// Read the deploy file, require what must be set, and fill in the rest.
    ///
    /// Deploy-file values OVERRIDE the process environment, so `TBD_A2S_PORT=1 cargo xtask deploy
    /// staging` is ignored when the deploy file sets `TBD_A2S_PORT`. Keys the file does not
    /// mention still come from the environment, which is how `TBD_MODPACK_JSON=… --render-only`
    /// works.
    pub fn load(env_file: &Path) -> Result<Env, u8> {
        if !env_file.is_file() {
            eprintln!(
                "Missing {} — copy from {}",
                env_file.display(),
                crate::core::repository_layout::DEPLOY_ENV_EXAMPLE
            );
            return Err(1);
        }
        let pairs = parse_deploy_env(env_file)?;
        // Snapshot the process environment, then let the file win.
        let mut map: std::collections::HashMap<String, String> = std::env::vars().collect();
        for (k, v) in pairs {
            map.insert(k, v);
        }
        // Unset and empty are the same thing to every read below.
        let get = |k: &str| -> String { map.get(k).cloned().unwrap_or_default() };
        // `line` is the line of the deploy file the value is expected on, so the message points
        // at the edit to make rather than at the check that refused.
        let req = |k: &str, line: u32, msg: &str| -> Result<String, u8> {
            let v = get(k);
            if v.is_empty() {
                eprintln!("deploy.env: line {line}: {k}: {msg}");
                return Err(1);
            }
            Ok(v)
        };
        let def = |k: &str, d: &str| -> String {
            let v = get(k);
            if v.is_empty() { d.to_string() } else { v }
        };

        let ssh_host = req("TBD_SSH_HOST", 1079, "TBD_SSH_HOST required in deploy.env")?;
        let remote_dir = req("TBD_REMOTE_DIR", 1080, "TBD_REMOTE_DIR required")?;
        let profile_dir = req("TBD_PROFILE_DIR", 1081, "TBD_PROFILE_DIR required")?;
        let addons_staging = req("TBD_ADDONS_STAGING", 1082, "TBD_ADDONS_STAGING required")?;
        let game_server_token = req(
            "TBD_GAME_SERVER_TOKEN",
            1083,
            "TBD_GAME_SERVER_TOKEN required",
        )?;

        let bind_ip = def("TBD_BIND_IP", "192.168.0.140");
        Ok(Env {
            ssh_host,
            remote_dir,
            profile_dir: profile_dir.clone(),
            addons_staging,
            game_server_token,
            mission_id: def("TBD_MISSION_ID", "msn_8f3a2c"),
            event_id: def("TBD_EVENT_ID", "b0000000-0000-4000-8000-000000000001"),
            backend_url: def("TBD_BACKEND_URL", "http://127.0.0.1:8080"),
            addon_guid: def("TBD_ADDON_GUID", "B2C3D4E5F6A78901"),
            // T-607: NOT `: "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/...}"`. That idiom — which
            // is what this line was — is silently truncated by bash: the `}` of the ResourceGUID
            // closes the parameter expansion, so the default became `{69A85365FC09E2CA` and the
            // rest of the line was parsed as literal text and discarded. Measured:
            //   $ : "${X:={69A85365FC09E2CA}Missions/TBD_Dev_POC.conf}"; echo "[$X]"
            //   [{69A85365FC09E2CA]
            // Every deploy that did NOT override TBD_SCENARIO rendered a config the engine
            // hard-rejects, and found out ~90 s into the boot, after a full rsync and script
            // compile. Rust has no such parse, but `validate_server_config` still checks for the
            // truncated shape — that validator is what caught it.
            scenario: def(
                "TBD_SCENARIO",
                "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf",
            ),
            server_dir: def("TBD_SERVER_DIR", "/home/sam/steam/arma-reforger-server"),
            // Server launch mode. `config` is THE DEFAULT and the only mode that is both correct
            // and joinable; see `boot.rs` for why the default used to be `addons` and why that was
            // the wrong half to default to.
            server_mode: def("TBD_SERVER_MODE", "config"),
            workshop_mod_id: get("TBD_WORKSHOP_MOD_ID"),
            public_address: def("TBD_PUBLIC_ADDRESS", &bind_ip),
            game_port: def("TBD_GAME_PORT", "2001"),
            // MUST differ from TBD_GAME_PORT or replication fails.
            a2s_port: def("TBD_A2S_PORT", "17777"),
            server_name: def("TBD_SERVER_NAME", "TBD Staging POC"),
            admin_password: def("TBD_ADMIN_PASSWORD", "tbd-admin"),
            max_players: def("TBD_MAX_PLAYERS", "64"),
            // comma-separated identityIds → in-game admins (#tbd commands)
            admin_identity_ids: get("TBD_ADMIN_IDENTITY_IDS"),
            server_config_remote: def(
                "TBD_SERVER_CONFIG_REMOTE",
                &format!("{}/server.config.json", dirname(&profile_dir)),
            ),
            // T-607: how long to wait for the engine to reach a verdict before failing the deploy.
            // Room registration landed 14 s after start on a measured 2026-08-01 boot, but that
            // number is not reliable — the playtest runner records the same binary and config
            // registering in 13 s on one boot and never across 300 s on another. This is a bound
            // on patience, not an estimate.
            boot_verify_timeout: def("TBD_BOOT_VERIFY_TIMEOUT", "180"),
            modpack_json: get("TBD_MODPACK_JSON"),
            modpack_url: get("TBD_MODPACK_URL"),
            modpack_token: get("TBD_MODPACK_TOKEN"),
            workshop_mod_name: def("TBD_WORKSHOP_MOD_NAME", "TBD_Framework"),
            run_t092_smoke: def("TBD_RUN_T092_SMOKE", "0") == "1",
            ssh_pass: map.get("TBD_SSH_PASS").filter(|v| !v.is_empty()).cloned(),
            ssh_identity_file: map
                .get("TBD_SSH_IDENTITY_FILE")
                .filter(|v| !v.is_empty())
                .cloned(),
            bind_ip,
        })
    }

    /// The gproj cross-check, the prairielearn refusal and the `TBD_SERVER_MODE` case, in the
    /// bash's order (guid at 1143, prairielearn at 1197, mode at 1202). The order is observable:
    /// a deploy.env with both a stale guid and a prairielearn path reports the guid.
    pub fn validate(&self, mono_root: &Path) -> Result<(), u8> {
        // T-607: the GUID is the join between the deployed checkout and game.mods[], and if
        // deploy.env drifts from the gproj the addon assertion starts checking the wrong id — it
        // would then pass only when the mod did NOT load. Cross-check rather than trust.
        if let Some(g) = super::boot::read_addon_guid(mono_root)
            && !g.is_empty()
            && g != self.addon_guid
        {
            eprintln!(
                "TBD_ADDON_GUID='{}' does not match apps/mod/tbd-framework/addon.gproj",
                self.addon_guid
            );
            eprintln!("  ('{g}'). The gproj is the source of truth — fix deploy.env, or the boot");
            eprintln!("  assertion will be checking an addon id this checkout does not publish.");
            return Err(1);
        }
        if self.remote_dir.contains("prairielearn") {
            eprintln!("Refusing to deploy: TBD_REMOTE_DIR must not be under prairielearn/");
            return Err(1);
        }
        match self.server_mode.as_str() {
            "addons" => {}
            "config" => {
                // TBD_WORKSHOP_MOD_ID is the single-mod env fallback and is only required when
                // no modpack document is configured — a modpack carries its own workshop ids.
                if self.workshop_mod_id.is_empty()
                    && self.modpack_json.is_empty()
                    && self.modpack_url.is_empty()
                {
                    eprintln!(
                        "TBD_SERVER_MODE=config requires TBD_WORKSHOP_MOD_ID (publish tbd-framework"
                    );
                    eprintln!(
                        "to the Workshop first, then set its modId in deploy.env), or a modpack"
                    );
                    eprintln!("source: TBD_MODPACK_JSON=<file> / TBD_MODPACK_URL=<url> (T-288).");
                    return Err(1);
                }
                if self.a2s_port == self.game_port {
                    eprintln!(
                        "TBD_A2S_PORT must differ from TBD_GAME_PORT (a2s/game can't share a UDP port)."
                    );
                    return Err(1);
                }
                // T-607: validate admin ids against the ENGINE's own schema, here, before anything
                // is rsynced. Both patterns copied verbatim out of the engine's rejection of a bad
                // value (1.7.0.54):
                //   BACKEND (E): RegEx Pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
                //   BACKEND (E): RegEx Pattern: "^[0-9]{17}$"
                // A bad entry is a HARD FATAL at boot ("There are errors in server config!" ->
                // "Unable to initialize the game") reported ~90 s in, AFTER a full deploy and
                // script compile. Failing here costs a millisecond and names the value instead of
                // burning a deploy cycle.
                if !self.admin_identity_ids.is_empty() {
                    let uuid = Regex::new(
                        "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
                    )
                    .expect("static");
                    let steam = Regex::new("^[0-9]{17}$").expect("static");
                    for raw in self.admin_identity_ids.split(',') {
                        let aid = xargs_like(raw);
                        if aid.is_empty() {
                            continue;
                        }
                        if !uuid.is_match(&aid) && !steam.is_match(&aid) {
                            eprintln!(
                                "TBD_ADMIN_IDENTITY_IDS contains '{aid}', which is neither an identityId nor a SteamID."
                            );
                            eprintln!(
                                "  identityId: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (lowercase hex)"
                            );
                            eprintln!("  SteamID:    17 digits");
                            eprintln!(
                                "  The engine rejects anything else and refuses to start; this is its schema, not ours."
                            );
                            return Err(1);
                        }
                    }
                } else {
                    println!(
                        "NOTE: TBD_ADMIN_IDENTITY_IDS is empty, so game.admins[] will be []. Every '#tbd'"
                    );
                    println!(
                        "      command answers 'TBD: admin only.' — TBD_AdminService.IsAdmin() resolves from"
                    );
                    println!(
                        "      vanilla's SCR_PlayerListedAdminManagerComponent, which is populated ONLY from"
                    );
                    println!(
                        "      game.admins[]. 'passwordAdmin' is a different mechanism and does not feed it."
                    );
                }
            }
            other => {
                eprintln!("Invalid TBD_SERVER_MODE='{other}' (expected: addons | config)");
                return Err(1);
            }
        }
        Ok(())
    }

    /// Count of non-blank admin ids — the bash's
    /// `tr ',' '\n' | grep -c '[^[:space:]]'`, used for the boot verdict's admin assertion.
    pub fn admin_count(&self) -> usize {
        if self.admin_identity_ids.is_empty() {
            return 0;
        }
        self.admin_identity_ids
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count()
    }

    /// The `[dry-run] game.mods[] from:` label. `modId=$TBD_WORKSHOP_MOD_ID` was the only thing
    /// this ever printed, which read as "the mod list is fine" on a run whose mod list came from
    /// nowhere near the modpack the operator had authored.
    pub fn mod_source_label(&self) -> String {
        if !self.modpack_json.is_empty() {
            format!("modpack file {}", self.modpack_json)
        } else if !self.modpack_url.is_empty() {
            format!("modpack API {}", self.modpack_url)
        } else {
            format!(
                "single-mod env fallback TBD_WORKSHOP_MOD_ID={} (no modpack configured)",
                self.workshop_mod_id
            )
        }
    }
}

#[cfg(test)]
#[path = "tests/config/tests.rs"]
pub(super) mod tests;
