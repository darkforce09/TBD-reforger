//! `Env`: every setting `cargo xtask deploy staging` reads, and the settings check it runs before
//! anything reaches the host.
//!
//! **Role:** builds [`Env`] from `deploy.env` and the process environment, fills in the defaults,
//! refuses the settings the fleet does not read ([`RETIRED_SETTINGS`]), and refuses what the host
//! or the engine would refuse ([`Env::validate`]).
//!
//! **Position:** fed by [`deploy_settings`], which owns the file's grammar and the
//! precedence rule (the file decides every key it assigns; the process environment fills only the
//! keys it never assigns); consumed by the render, the payloads and the deploy pipeline. The fleet
//! settings are [`super::fleet_instances::FleetSettings`]'s.
//!
//! **Signals & state:** none; [`Env`] is built once per run. The public address default asks the
//! resolver for the host's IPv4 address.
//!
//! **Invariants:** no secret is a setting: machine credentials, RCON passwords and the join password
//! live only in files on the host; the remote folders default under the deploy user's home
//! ([`DeployHostFolder`]); `backend_url` is [`fleet_instances::backend_url`]'s reading, the one
//! `cargo xtask staging` shares; `publicAddress` is `TBD_PUBLIC_ADDRESS` when set, which must be
//! IPv4, else the first IPv4 address `TBD_SSH_HOST` resolves to at deploy time, else the deploy
//! stops; the server configs are rendered from these values and nothing else.
//!
//! ── WHERE `game.mods[]` COMES FROM ───────────────────────────────────────────────────────────
//!
//! Hardcoding ONE mod — `{"modId": "$TBD_WORKSHOP_MOD_ID", "name":
//! "TBD_Framework"}` — and never read the `modpacks` / `modpack_mods` tables. A modpack authored
//! on the website therefore had no path to a running server.
//!
//! THE SOURCE IS THE API, and specifically the bytes of `GET /api/v1/modpacks/current`
//! (`crates/api/api_server/src/router.rs` → `handlers/modpacks.rs::get_current_modpack`) whose `mods[]`
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

use super::fleet_instances::{self, FleetSettings, MAXIMUM_FLEET_INSTANCES};
use deploy_settings::{DeployEnvironment, DeployHost, DeployHostFolder, SettingError};

newtype_ids::string_id! {
    /// `TBD_WORKSHOP_MOD_ID`: the Workshop id of the published framework mod, the single-mod
    /// fallback of `game.mods[]` when no modpack is configured; empty when unset.
    pub struct WorkshopModId;
}

/// Every setting the deploy reads, after `deploy.env` and the `:=` defaults have been applied.
#[derive(Debug, Clone)]
pub struct Env {
    /// `TBD_SSH_HOST`: the ssh destination, and the name the public address resolves from.
    pub deploy_host: DeployHost,
    pub remote_dir: String,
    /// `TBD_PROFILE_DIR`: the single-instance server's profile, which
    /// `--migrate-single-instance` archives. No fleet instance uses it.
    pub profile_dir: String,
    /// `TBD_ADDONS_STAGING`: the `-addonsDir` every instance shares.
    pub addons_staging: String,
    /// `TBD_BACKEND_URL` without a trailing `/` ([`fleet_instances::backend_url`]): every
    /// instance profile's `backendUrl`, and the origin the health check probes.
    pub backend_url: String,
    pub addon_guid: String,
    /// `TBD_SCENARIO`: the scenario of an instance whose config does not exist yet.
    pub scenario: String,
    /// `TBD_SERVER_DIR`: the install of Steam app 1890870, the experimental dedicated server.
    pub server_dir: String,
    /// `TBD_WORKSHOP_MOD_ID`, empty when unset.
    pub workshop_mod_id: WorkshopModId,
    /// `publicAddress` of every server config: the address the backend rooms advertise.
    pub public_address: Ipv4Addr,
    pub admin_password: String,
    pub max_players: String,
    pub admin_identity_ids: String,
    pub boot_verify_timeout: String,
    pub modpack_json: String,
    pub modpack_url: String,
    pub modpack_token: String,
    pub workshop_mod_name: String,
    /// The instances, their ports and the relay.
    pub fleet: FleetSettings,
    pub ssh_pass: Option<String>,
    pub ssh_identity_file: Option<String>,
}

/// Settings the fleet deploy does not read, each with what the fleet reads instead. A deploy file
/// that assigns one is refused, so no value is silently ignored and no secret stays on the
/// development machine.
pub(super) const RETIRED_SETTINGS: &[(&str, &str)] = &[
    (
        "TBD_MOD_RUNTIME_CREDENTIAL",
        "each instance's mod_runtime credential lives on the host in \
         ~/tbd/fleet/instance-N/secrets/mod-runtime-credential",
    ),
    (
        "TBD_HOST_AGENT_CREDENTIAL",
        "each instance's host_agent credential lives on the host in \
         ~/tbd/fleet/instance-N/secrets/host-agent-credential",
    ),
    (
        "TBD_RCON_PASSWORD",
        "each instance's RCON password is generated on the host into \
         ~/tbd/fleet/instance-N/secrets/rcon-password",
    ),
    (
        "TBD_RCON_PORT",
        "instance N's RCON port is TBD_FLEET_RCON_PORT_BASE + N",
    ),
    (
        "TBD_GAME_PORT",
        "instance N's game port is TBD_FLEET_GAME_PORT_BASE + N",
    ),
    (
        "TBD_A2S_PORT",
        "instance N's A2S port is TBD_FLEET_A2S_PORT_BASE + N",
    ),
    (
        "TBD_INSTALL_HOST_AGENT",
        "every fleet instance runs its host agent",
    ),
    (
        "TBD_SERVER_MODE",
        "every fleet instance starts with -config",
    ),
    ("TBD_SERVER_NAME", "instance N is named \"TBD Staging N\""),
    (
        "TBD_SERVER_CONFIG_REMOTE",
        "instance N's config is ~/tbd/fleet/instance-N/server.config.json",
    ),
];

/// `dirname`, POSIX. Strip trailing slashes, drop the last component, and answer `.` for a bare
/// name. Used for the single-instance server config beside `TBD_PROFILE_DIR`; measured against
/// coreutils: `/p/q`→`/p`, `/p`→`/`, `p`→`.`, `/p/q/`→`/p`.
pub(super) fn dirname(p: &str) -> String {
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

/// Every [`RETIRED_SETTINGS`] key the settings still assign, as refusals naming the replacement.
pub(super) fn retired_setting_refusals(environment: &DeployEnvironment) -> Vec<SettingError> {
    RETIRED_SETTINGS
        .iter()
        .filter(|(key, _)| environment.value(key).is_some())
        .map(|(key, replacement)| {
            environment.invalid(
                key,
                format!("is no longer read ({replacement}); delete the assignment"),
            )
        })
        .collect()
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

    /// Refuses every retired setting at once, then fills in the rest. Deploy-file values override
    /// the process environment, so `TBD_MAX_PLAYERS=1 cargo xtask deploy staging` is ignored when
    /// the deploy file sets `TBD_MAX_PLAYERS`; keys the file never assigns still come from the
    /// environment, which is how `TBD_MODPACK_JSON=… --render-only` works.
    pub fn from_environment(environment: &DeployEnvironment) -> Result<Env, u8> {
        let retired = retired_setting_refusals(environment);
        if !retired.is_empty() {
            for refusal in &retired {
                eprintln!("{refusal}");
            }
            return Err(1);
        }
        let refuse = |error: SettingError, hint: &str| -> u8 {
            eprintln!("{error}");
            if !hint.is_empty() {
                eprintln!("  {hint}");
            }
            1
        };
        let get = |k: &str| -> String { environment.value(k).unwrap_or_default().to_string() };
        let def = |k: &str, d: &str| -> String { environment.value_or(k, d).to_string() };
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
        let backend_url = fleet_instances::backend_url(environment);
        let fleet = FleetSettings::from_environment(environment).map_err(|error| {
            refuse(
                error,
                &format!("the fleet runs 1 to {MAXIMUM_FLEET_INSTANCES} instances"),
            )
        })?;

        Ok(Env {
            deploy_host,
            remote_dir,
            profile_dir,
            addons_staging,
            backend_url,
            addon_guid: def("TBD_ADDON_GUID", "B2C3D4E5F6A78901"),
            // NOT `: "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/...}"`. That idiom is silently
            // truncated by bash: the `}` of the ResourceGUID closes the parameter expansion, so the
            // default became `{69A85365FC09E2CA` and the rest was discarded. Measured:
            //   $ : "${X:={69A85365FC09E2CA}Missions/TBD_Dev_POC.conf}"; echo "[$X]"
            //   [{69A85365FC09E2CA]
            // Rust has no such parse, but `validate_server_config` still checks for the truncated
            // shape, which the engine hard-rejects ~90 s into a boot.
            scenario: def(
                "TBD_SCENARIO",
                "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf",
            ),
            server_dir,
            workshop_mod_id: WorkshopModId::new(get("TBD_WORKSHOP_MOD_ID")),
            public_address,
            admin_password: def("TBD_ADMIN_PASSWORD", "tbd-admin"),
            max_players: def("TBD_MAX_PLAYERS", "64"),
            // comma-separated identityIds → in-game admins (#tbd commands)
            admin_identity_ids: get("TBD_ADMIN_IDENTITY_IDS"),
            // How long to wait for the engines to reach a verdict before failing the deploy.
            // Room registration landed 14 s after start on a measured 2026-08-01 boot, but that
            // number is not reliable — the playtest runner records the same binary and config
            // registering in 13 s on one boot and never across 300 s on another. This is a bound
            // on patience, not an estimate.
            boot_verify_timeout: def("TBD_BOOT_VERIFY_TIMEOUT", "180"),
            modpack_json: get("TBD_MODPACK_JSON"),
            modpack_url: get("TBD_MODPACK_URL"),
            modpack_token: get("TBD_MODPACK_TOKEN"),
            workshop_mod_name: def("TBD_WORKSHOP_MOD_NAME", "TBD_Framework"),
            fleet,
            ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
            ssh_identity_file: environment
                .value("TBD_SSH_IDENTITY_FILE")
                .map(str::to_string),
        })
    }

    /// The single-instance server config the migration archives: `server.config.json` beside
    /// `TBD_PROFILE_DIR`.
    pub fn single_instance_server_config(&self) -> String {
        format!("{}/server.config.json", dirname(&self.profile_dir))
    }

    /// The gproj cross-check, the prairielearn refusal, the fleet's port rules, the mod source and
    /// the admin ids, in that order. The order is observable: a deploy.env with both a stale guid
    /// and a prairielearn path reports the guid.
    pub fn validate(&self, mono_root: &Path) -> Result<(), u8> {
        // The GUID is the join between the deployed checkout and game.mods[], and if
        // deploy.env drifts from the gproj the addon assertion starts checking the wrong id — it
        // would then pass only when the mod did NOT load. Cross-check rather than trust.
        if let Some(g) = super::boot::read_addon_guid(mono_root)
            && !g.is_empty()
            && g != self.addon_guid
        {
            eprintln!(
                "TBD_ADDON_GUID='{}' does not match {}/addon.gproj",
                self.addon_guid,
                repository_layout::enfusion_mod_folders::FRAMEWORK_ADDON_DIR
            );
            eprintln!("  ('{g}'). The gproj is the source of truth — fix deploy.env, or the boot");
            eprintln!("  assertion will be checking an addon id this checkout does not publish.");
            return Err(1);
        }
        if self.remote_dir.contains("prairielearn") {
            eprintln!("Refusing to deploy: TBD_REMOTE_DIR must not be under prairielearn/");
            return Err(1);
        }
        if let Err(problem) = self.fleet.check_port_rules() {
            eprintln!("Refusing to deploy: {problem}.");
            return Err(1);
        }
        // TBD_WORKSHOP_MOD_ID is the single-mod env fallback and is only required when no modpack
        // document is configured — a modpack carries its own workshop ids.
        if self.workshop_mod_id.as_str().is_empty()
            && self.modpack_json.is_empty()
            && self.modpack_url.is_empty()
        {
            eprintln!(
                "The fleet's -config servers need TBD_WORKSHOP_MOD_ID (publish tbd-framework"
            );
            eprintln!("to the Workshop first, then set its modId in deploy.env), or a modpack");
            eprintln!("source: TBD_MODPACK_JSON=<file> / TBD_MODPACK_URL=<url>.");
            return Err(1);
        }
        // Validate admin ids against the ENGINE's own schema, here, before anything is rsynced.
        // Both patterns copied verbatim out of the engine's rejection of a bad value (1.7.0.54):
        //   BACKEND (E): RegEx Pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        //   BACKEND (E): RegEx Pattern: "^[0-9]{17}$"
        // A bad entry is a HARD FATAL at boot ("There are errors in server config!" -> "Unable to
        // initialize the game") reported ~90 s in, AFTER a full deploy and script compile.
        if self.admin_identity_ids.is_empty() {
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
            return Ok(());
        }
        let uuid = Regex::new("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .expect("static");
        let steam = Regex::new("^[0-9]{17}$").expect("static");
        for raw in self.admin_identity_ids.split(',') {
            let aid = xargs_like(raw);
            if aid.is_empty() || uuid.is_match(&aid) || steam.is_match(&aid) {
                continue;
            }
            eprintln!(
                "TBD_ADMIN_IDENTITY_IDS contains '{aid}', which is neither an identityId nor a SteamID."
            );
            eprintln!("  identityId: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (lowercase hex)");
            eprintln!("  SteamID:    17 digits");
            eprintln!(
                "  The engine rejects anything else and refuses to start; this is its schema, not ours."
            );
            return Err(1);
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
