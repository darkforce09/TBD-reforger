//! `Env`: every setting `cargo xtask deploy staging` reads, and the settings check it runs before
//! anything reaches the host.
//!
//! **Role:** builds [`Env`] from `deploy.env` and the process environment, fills in the defaults,
//! and refuses what the host or the engine would refuse ([`Env::validate`]).
//!
//! **Position:** fed by [`crate::core::deploy_environment`], which owns the file's grammar and the
//! precedence rule (the file decides every key it assigns; the process environment fills only the
//! keys it never assigns); consumed by the render, the payloads and the deploy pipeline.
//!
//! **Signals & state:** none; [`Env`] is built once per run. The public address default asks the
//! resolver for the host's IPv4 address.
//!
//! **Invariants:** the remote folders default under the deploy user's home
//! ([`DeployHostFolder`]); `publicAddress` is `TBD_PUBLIC_ADDRESS` when set, which must be IPv4,
//! else the first IPv4 address `TBD_SSH_HOST` resolves to at deploy time, else the deploy stops;
//! the server config is rendered from these values and nothing else.
//!
//! ── WHERE `game.mods[]` COMES FROM ───────────────────────────────────────────────────────────
//!
//! Hardcoding ONE mod — `{"modId": "$TBD_WORKSHOP_MOD_ID", "name":
//! "TBD_Framework"}` — and never read the `modpacks` / `modpack_mods` tables. A modpack authored
//! on the website therefore had no path to a running server.
//!
//! THE SOURCE IS THE API, and specifically the bytes of `GET /api/v1/modpacks/current`
//! (`apps/website/api_v2/src/core/http_router.rs` → `handlers/modpacks.rs::get_current_modpack`) whose `mods[]`
//! rows carry exactly the fields a Reforger `game.mods[]` entry needs — `workshop_id`, `mod_guid`,
//! `version` — added in `migrations/0012_modpack_mods_workshop.sql`, whose header says
//! verbatim: "keep both so a future renderer can choose".
//!
//! REJECTED — reading Postgres directly: this is not a DB client (no `DATABASE_URL` in
//! `deploy.env.example`), the database lives inside docker compose on the remote host, and
//! hand-rolling the projection would duplicate the null-tolerant COALESCE read in
//! `handlers/modpacks.rs mod_cols!()`. The next migration would break the renderer silently.
//! REJECTED — inventing a modpack file format of our own: that IS the defect this render avoids.
//!
//! ⚠ THE DEPLOY HOST HOLDS NO CREDENTIAL FOR THIS READ. `/modpacks/current` is gated by
//! `AuthUser`, a **Bearer JWT** minted from a Discord login (`middleware/auth.rs`). The deploy's
//! secrets are machine credentials (`mod_runtime`, `host_agent`), which that route does not accept.
//! So `TBD_MODPACK_URL` is satisfied only by a user JWT in `TBD_MODPACK_TOKEN`; without one it fails
//! closed.
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

use std::net::Ipv4Addr;
use std::path::Path;

use regex::Regex;

use crate::core::deploy_environment::{
    DeployEnvironment, DeployHost, DeployHostFolder, SettingError,
};

/// Every setting the deploy reads, after `deploy.env` and the `:=` defaults have been applied.
#[derive(Debug, Clone)]
pub struct Env {
    /// `TBD_SSH_HOST`: the ssh destination, and the name the public address resolves from.
    pub deploy_host: DeployHost,
    pub remote_dir: String,
    pub profile_dir: String,
    pub addons_staging: String,
    /// `TBD_MOD_RUNTIME_CREDENTIAL`: the game runtime's `mod_runtime` machine credential, written
    /// into the profile's `TBD_BackendConfig.json` as `machineCredential`.
    pub mod_runtime_credential: String,
    pub backend_url: String,
    pub addon_guid: String,
    pub scenario: String,
    pub server_dir: String,
    pub server_mode: String,
    pub workshop_mod_id: String,
    /// `publicAddress` of the server config: the address the backend room advertises.
    pub public_address: Ipv4Addr,
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
    /// The host agent install, when `TBD_INSTALL_HOST_AGENT=1`.
    pub host_agent: Option<super::host_agent::HostAgentSettings>,
    pub ssh_pass: Option<String>,
    pub ssh_identity_file: Option<String>,
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

/// `TBD_PUBLIC_ADDRESS` when set, which must be an IPv4 address; else the first IPv4 address
/// `host` resolves to from this machine.
fn public_address(
    environment: &DeployEnvironment,
    host: &DeployHost,
) -> Result<Ipv4Addr, SettingError> {
    const KEY: &str = "TBD_PUBLIC_ADDRESS";
    if let Some(explicit) = environment.value(KEY) {
        return explicit
            .parse::<Ipv4Addr>()
            .map_err(|_| environment.invalid(KEY, format!("`{explicit}` is not an IPv4 address")));
    }
    host.resolve_ipv4().map_err(|cause| {
        environment.not_derivable(
            KEY,
            format!("{} has no IPv4 address from here ({cause})", host.host()),
            "run avahi-daemon on the host",
        )
    })
}

impl Env {
    /// Loads the deploy file, which must exist, and builds [`Env`] from it; a refusal is
    /// printed and answered with exit 1.
    pub fn load(env_file: &Path) -> Result<Env, u8> {
        match DeployEnvironment::load_required(env_file) {
            Ok(environment) => Env::from_environment(&environment),
            Err(error) => {
                eprintln!("{error}");
                Err(1)
            }
        }
    }

    /// Requires what must be set and fills in the rest. Deploy-file values override the process
    /// environment, so `TBD_A2S_PORT=1 cargo xtask deploy staging` is ignored when the deploy
    /// file sets `TBD_A2S_PORT`; keys the file never assigns still come from the environment,
    /// which is how `TBD_MODPACK_JSON=… --render-only` works.
    pub fn from_environment(environment: &DeployEnvironment) -> Result<Env, u8> {
        let refuse = |error: SettingError, hint: &str| -> u8 {
            eprintln!("{error}");
            if !hint.is_empty() {
                eprintln!("  {hint}");
            }
            1
        };
        let get = |k: &str| -> String { environment.value(k).unwrap_or_default().to_string() };
        let def = |k: &str, d: &str| -> String { environment.value_or(k, d).to_string() };
        let req = |k: &str, hint: &str| -> Result<String, u8> {
            environment
                .required(k)
                .map(str::to_string)
                .map_err(|error| refuse(error, hint))
        };
        let deploy_host = environment
            .deploy_host()
            .map_err(|error| refuse(error, ""))?;
        let folder = |folder: DeployHostFolder| {
            folder
                .resolve(environment, &deploy_host)
                .map_err(|error| refuse(error, "TBD_SSH_HOST names no user to default it under"))
        };
        let remote_dir = folder(DeployHostFolder::Checkout)?;
        let profile_dir = folder(DeployHostFolder::Profile)?;
        let addons_staging = folder(DeployHostFolder::AddonsStaging)?;
        let server_dir = folder(DeployHostFolder::ServerInstall)?;
        let public_address =
            public_address(environment, &deploy_host).map_err(|error| refuse(error, ""))?;
        let mod_runtime_credential = req(
            "TBD_MOD_RUNTIME_CREDENTIAL",
            "issue a mod_runtime credential for this server in Server Control",
        )?;
        let backend_url = def("TBD_BACKEND_URL", "http://127.0.0.1:8080");
        let host_agent = if def("TBD_INSTALL_HOST_AGENT", "0") == "1" {
            Some(super::host_agent::HostAgentSettings {
                credential: req(
                    "TBD_HOST_AGENT_CREDENTIAL",
                    "required with TBD_INSTALL_HOST_AGENT=1: issue a host_agent credential for this server",
                )?,
                rcon_password: req(
                    "TBD_RCON_PASSWORD",
                    "required with TBD_INSTALL_HOST_AGENT=1",
                )?,
                rcon_port: def("TBD_RCON_PORT", "19999"),
                api_base_url: def("TBD_HOST_AGENT_API_URL", &backend_url),
            })
        } else {
            None
        };

        Ok(Env {
            deploy_host,
            remote_dir,
            profile_dir: profile_dir.clone(),
            addons_staging,
            mod_runtime_credential,
            backend_url,
            addon_guid: def("TBD_ADDON_GUID", "B2C3D4E5F6A78901"),
            // NOT `: "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/...}"`. That idiom — which
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
            server_dir,
            // Server launch mode. `config` is THE DEFAULT and the only mode that is both correct
            // and joinable; see `boot.rs` for why the default is not `addons` and why that is
            // the wrong half to default to.
            server_mode: def("TBD_SERVER_MODE", "config"),
            workshop_mod_id: get("TBD_WORKSHOP_MOD_ID"),
            public_address,
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
            // How long to wait for the engine to reach a verdict before failing the deploy.
            // Room registration landed 14 s after start on a measured 2026-08-01 boot, but that
            // number is not reliable — the playtest runner records the same binary and config
            // registering in 13 s on one boot and never across 300 s on another. This is a bound
            // on patience, not an estimate.
            boot_verify_timeout: def("TBD_BOOT_VERIFY_TIMEOUT", "180"),
            modpack_json: get("TBD_MODPACK_JSON"),
            modpack_url: get("TBD_MODPACK_URL"),
            modpack_token: get("TBD_MODPACK_TOKEN"),
            workshop_mod_name: def("TBD_WORKSHOP_MOD_NAME", "TBD_Framework"),
            host_agent,
            ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
            ssh_identity_file: environment
                .value("TBD_SSH_IDENTITY_FILE")
                .map(str::to_string),
        })
    }

    /// The gproj cross-check, the prairielearn refusal and the `TBD_SERVER_MODE` case, in the
    /// bash's order (guid at 1143, prairielearn at 1197, mode at 1202). The order is observable:
    /// a deploy.env with both a stale guid and a prairielearn path reports the guid.
    pub fn validate(&self, mono_root: &Path) -> Result<(), u8> {
        // The GUID is the join between the deployed checkout and game.mods[], and if
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
        super::host_agent::validate_machine_credential(
            "TBD_MOD_RUNTIME_CREDENTIAL",
            &self.mod_runtime_credential,
        )?;
        if let Some(host_agent) = &self.host_agent {
            host_agent.validate(&self.server_mode)?;
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
                    eprintln!("source: TBD_MODPACK_JSON=<file> / TBD_MODPACK_URL=<url>.");
                    return Err(1);
                }
                if self.a2s_port == self.game_port {
                    eprintln!(
                        "TBD_A2S_PORT must differ from TBD_GAME_PORT (a2s/game can't share a UDP port)."
                    );
                    return Err(1);
                }
                // Validate admin ids against the ENGINE's own schema, here, before anything
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
