//! One `server.config.json` per fleet instance, rendered on the development machine.
//!
//! **Role:** renders instance N's config from [`Env`] and its [`FleetInstance`]: ports, name,
//! server browser visibility, the loopback `rcon` block with `admin` permission, and the shared mod
//! list, admins and scenario; `--render-only <directory>` writes every instance's config and checks
//! each ([`render_only`]).
//!
//! **Position:** reads the mod list through [`super::render`] once per run and hands each rendered
//! text to `super::payloads::instance_files_payload`, which fills the two passwords in on the host.
//!
//! **Signals & state:** none beyond the files it writes.
//!
//! **Invariants:** a rendered config carries no secret: `rcon.password` is
//! [`RCON_PASSWORD_PLACEHOLDER`] and `game.password` is [`JOIN_PASSWORD_PLACEHOLDER`], and the host
//! replaces both from its own files; RCON listens on `127.0.0.1` only; only instance 1 is
//! `visible`; every rendered file passes `super::render::validate_server_config`.

use std::fs;
use std::path::Path;

use super::config::{Env, xargs_like};
use super::fleet_instances::FleetInstance;
use super::render::{modpack_mods_json, resolve_modpack_doc, validate_server_config};

/// Stands in for the instance's RCON password; the host writes the password from
/// `~/tbd/fleet/instance-N/secrets/rcon-password` in its place.
pub(super) const RCON_PASSWORD_PLACEHOLDER: &str = "TBD_RCON_PASSWORD_FROM_HOST_FILE";
/// Stands in for the join password; the host writes the password from `~/tbd/fleet/join-password`
/// in its place.
pub(super) const JOIN_PASSWORD_PLACEHOLDER: &str = "TBD_JOIN_PASSWORD_FROM_HOST_FILE";

/// The fleet's `game.mods[]`, resolved once per run from the modpack source.
pub(super) fn fleet_mods_json(env: &Env) -> Result<String, u8> {
    let (doc, src_label) = resolve_modpack_doc(env)?;
    modpack_mods_json(&doc, &src_label)
}

/// The `rcon` block: loopback only, `admin` permission (the console command needs it), two clients,
/// and the password placeholder.
pub(super) fn rcon_block(instance: &FleetInstance) -> String {
    format!(
        "\"rcon\": {{ \"address\": \"127.0.0.1\", \"port\": {port}, \"password\": \
         \"{RCON_PASSWORD_PLACEHOLDER}\", \"permission\": \"admin\", \"maxClients\": 2 }},",
        port = instance.rcon_port,
    )
}

/// Instance `instance`'s complete server config to the LOCAL path `out`, then validated.
///
/// ODDITY PRESERVED, and it is the reason the validator exists: the template substitutes RAW,
/// unescaped values. A `TBD_ADMIN_PASSWORD` containing a double quote produces a document that is
/// not JSON at all, and the validator catches it by re-parsing the file.
///
/// `scenario` is the scenario the instance runs: deployments own it once the host agent has
/// switched it, so the deploy passes the live config's value and `TBD_SCENARIO` seeds only an
/// instance that has none.
pub(super) fn render_instance_server_config(
    env: &Env,
    mods_json: &str,
    instance: &FleetInstance,
    scenario: &str,
    out: &Path,
) -> Result<(), u8> {
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
  {rcon}
  "game": {{
    "name": "{server_name}",
    "password": "{JOIN_PASSWORD_PLACEHOLDER}",
    "passwordAdmin": "{admin_password}",
    "admins": [{admins_json}],
    "scenarioId": "{scenario}",
    "maxPlayers": {max_players},
    "visible": {visible},
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
        game_port = instance.game_port,
        public_address = env.public_address,
        a2s_port = instance.a2s_port,
        rcon = rcon_block(instance),
        server_name = instance.server_name(),
        admin_password = env.admin_password,
        max_players = env.max_players,
        visible = instance.listed_in_server_browser(),
    );
    if let Err(e) = fs::write(out, &body) {
        eprintln!("FAIL: could not write {}: {e}", out.display());
        return Err(1);
    }
    validate_server_config(out)
}

/// `--render-only <directory>`: every instance's config to `<directory>/instance-N/
/// server.config.json`, each checked, with `TBD_SCENARIO` as the scenario; exit 0 only when all
/// pass. No socket is opened.
pub(super) fn render_only(env: &Env, directory: &str) -> u8 {
    println!("==> render every instance's server config (local only, no deploy) -> {directory}");
    let mods_json = match fleet_mods_json(env) {
        Ok(json) => json,
        Err(code) => return code,
    };
    let mut status = 0;
    for instance in env.fleet.instances() {
        let folder = Path::new(directory).join(format!("instance-{}", instance.number));
        if let Err(e) = fs::create_dir_all(&folder) {
            eprintln!("FAIL: could not create {}: {e}", folder.display());
            return 1;
        }
        let out = folder.join("server.config.json");
        println!(
            "  instance {}: {} -> {}",
            instance.number,
            instance.server_name(),
            out.display()
        );
        if let Err(code) =
            render_instance_server_config(env, &mods_json, &instance, &env.scenario, &out)
        {
            status = code;
        }
    }
    status
}
