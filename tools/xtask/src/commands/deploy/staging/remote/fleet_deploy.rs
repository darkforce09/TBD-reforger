//! The fleet deploy pipeline, and the plan `--dry-run` prints instead of running it.
//!
//! **Role:** [`deploy`] runs the whole fleet deploy in order: the website API check, the host's
//! secret files and single-instance check, the rsync, the migration when asked, each instance's
//! files and V2–V4 smoke, the units and the game server restart, a boot verdict per instance, the
//! relay and the host agents, and the log check per instance; [`dry_run_plan`] is the same walk as
//! text.
//!
//! **Position:** called by `run` in `tools/xtask/src/commands/deploy/staging.rs` through
//! [`super::deploy`]; it sends through [`super::Runner`] and takes every remote script from the
//! payload modules of `super::super`.
//!
//! **Signals & state:** none held; one run spawns `rsync`, `ssh` and one `mod remote-logs` child
//! per instance.
//!
//! **Invariants:** nothing on the host changes before its secret files are proven present; the
//! deploy stops at the first step that fails, with that step's code, except that every instance
//! gets its boot verdict before a failing one stops the deploy; it runs no compose command, because
//! the website stack on the host belongs to `cargo xtask deploy website`; no secret appears in an
//! argument vector, a payload or a printed line.

use super::super::acknowledgement_relay::relay_install_payload;
use super::super::fleet_server_config::{fleet_mods_json, render_instance_server_config};
use super::super::fleet_units::{game_servers_restart_payload, units_install_payload};
use super::super::host_agent::host_agents_install_payload;
use super::super::legacy_single_instance_migration::{
    migration_payload, migration_plan_line, single_instance_units_absent_payload,
    units_absent_plan_line,
};
use super::super::payloads::{
    fleet_secret_files_check_payload, instance_files_payload, smoke_payload,
};
use super::*;

/// What `--dry-run` prints after the settings load: every step, every instance, no socket.
pub(crate) fn dry_run_plan(env: &Env, instances: &[FleetInstance], migrate: bool) -> Vec<String> {
    let mut plan = vec![
        format!(
            "[dry-run] curl -sSf {} on the host; the deploy stops unless it answers",
            website_api_health_url(&env.backend_url)
        ),
        "[dry-run] check on the host: ~/tbd/fleet/join-password and each instance's two credential files (mode 600, expected shape, never printed)".to_string(),
    ];
    if !migrate {
        plan.push(units_absent_plan_line());
    }
    plan.push(format!(
        "[dry-run] rsync -avz --delete ... {}/",
        env.remote_dir
    ));
    if migrate {
        plan.push(migration_plan_line(
            &env.profile_dir,
            &env.single_instance_server_config(),
        ));
    }
    plan.push(format!(
        "[dry-run] game.mods[] from: {}",
        env.mod_source_label()
    ));
    for instance in instances {
        plan.push(format!(
            "[dry-run] instance {n}: \"{name}\" game {game} A2S {a2s} RCON 127.0.0.1:{rcon} (admin), visible {visible}, folder ~/{folder}, agent polls {api}",
            n = instance.number,
            name = instance.server_name(),
            game = instance.game_port,
            a2s = instance.a2s_port,
            rcon = instance.rcon_port,
            visible = instance.listed_in_server_browser(),
            folder = instance.home_relative_folder(),
            api = instance.agent_api_url,
        ));
        plan.push(format!(
            "[dry-run] instance {}: RCON password generated on the host once; profile, server config (passwords filled in on the host) and V2–V4 smoke",
            instance.number
        ));
    }
    plan.push(format!(
        "[dry-run] install tbd-reforger@.service (server {}, -addonsDir {}), fleet_host_agent@.service, acknowledgement-dropping-relay@.service; restart {}",
        env.server_dir,
        env.addons_staging,
        super::super::fleet_units::unit_list(instances, FleetInstance::game_server_unit)
    ));
    plan.push("[dry-run] boot verdict per instance over the console.log of its new boot".into());
    if let Some(relay) = &env.fleet.relay {
        plan.push(format!(
            "[dry-run] relay: acknowledgement-dropping-relay@{} on 127.0.0.1:{} forwarding to {}",
            relay.instance, relay.port, relay.upstream
        ));
    }
    plan.push(format!(
        "[dry-run] build fleet_host_agent; write ~/.config/fleet_host_agent/instance-N/agent.toml; restart {}",
        super::super::fleet_units::unit_list(instances, FleetInstance::host_agent_unit)
    ));
    plan.push("[dry-run] mod remote-logs --file over each instance's console.log".into());
    plan
}

/// One `ssh … bash -s` step that must succeed.
fn remote_step(runner: &Runner, base: &SshBase, host: &str, payload: String) -> Result<(), u8> {
    runner.ssh_ok(base, host, &bash_stdin(), Some(payload))
}

/// The whole deploy. `Ok(0)` only when every step held for every instance.
pub fn deploy(paths: &Paths, cli: &Cli) -> Result<u8> {
    let env = match Env::load(&paths.deploy_env) {
        Ok(e) => e,
        Err(code) => return Ok(code),
    };
    if let Err(code) = env.validate(&paths.mono_root) {
        return Ok(code);
    }
    // --render-only sits HERE, after the settings load and pass their check. It never reaches a
    // socket.
    if let Some(out) = cli.render_only_out.as_deref() {
        return Ok(super::super::fleet_server_config::render_only(&env, out));
    }
    let instances = env.fleet.instances();
    println!("==> publicAddress {}", env.public_address);
    if cli.dry_run {
        for line in dry_run_plan(&env, &instances, cli.migrate_single_instance) {
            println!("{line}");
        }
        return Ok(0);
    }
    let base = SshBase::from_settings(env.ssh_pass.as_deref(), env.ssh_identity_file.as_deref());
    let runner = Runner { dry_run: false };
    let host = env.deploy_host.ssh_destination();
    let step = |payload: String| remote_step(&runner, &base, &host, payload);

    println!(
        "==> website API ({} on the host)",
        website_api_health_url(&env.backend_url)
    );
    if let Err(code) = require_website_api(&runner, &base, &host, &env) {
        return Ok(code);
    }
    println!("==> secret files on the host");
    if let Err(code) = step(fleet_secret_files_check_payload(&instances)) {
        eprintln!(
            "  `cargo xtask staging provision-fleet` writes each instance's credentials; write"
        );
        eprintln!(
            "  the join password on the host with: umask 077; cat > ~/tbd/fleet/join-password"
        );
        return Ok(code);
    }
    if !cli.migrate_single_instance {
        println!("==> single-instance units");
        if let Err(code) = step(single_instance_units_absent_payload()) {
            return Ok(code);
        }
    }

    println!("==> rsync to {}", env.remote_dir);
    if let Err(e) = proc::which("rsync") {
        return Ok(not_run_exit(&e));
    }
    let mut rsync = base.with_password(Run::new("rsync"));
    for a in ssh_argv::rsync_argv(&base, &paths.mono_root, &host, &env.remote_dir)
        .iter()
        .skip(1)
    {
        rsync = rsync.arg(a);
    }
    match rsync.timeout(Duration::from_secs(7200)).merged_output() {
        Ok(o) => {
            let _ = io::stdout().write_all(o.text.as_bytes());
            if o.code != 0 {
                return Ok(o.code as u8);
            }
        }
        Err(e) => return Ok(not_run_exit(&e)),
    }

    if cli.migrate_single_instance {
        println!("==> migrate the single-instance server");
        let payload = migration_payload(&env.profile_dir, &env.single_instance_server_config());
        if let Err(code) = step(payload) {
            return Ok(code);
        }
    }

    let mods_json = match fleet_mods_json(&env) {
        Ok(json) => json,
        Err(code) => return Ok(code),
    };
    for instance in &instances {
        let n = instance.number;
        println!("==> instance {n}: files, profile and server config");
        let scenario = match deployed_scenario(&runner, &base, &host, instance) {
            Ok(Some(live)) => {
                if live != env.scenario {
                    println!(
                        "  keeping the deployed scenario {live} (TBD_SCENARIO seeds only a new instance)"
                    );
                }
                live
            }
            Ok(None) => env.scenario.clone(),
            Err(code) => return Ok(code),
        };
        let local = std::env::temp_dir().join(format!(
            "tbd-server.config.{}.instance-{n}.json",
            std::process::id()
        ));
        if let Err(code) =
            render_instance_server_config(&env, &mods_json, instance, &scenario, &local)
        {
            return Ok(code);
        }
        let rendered = fs::read_to_string(&local).unwrap_or_default();
        let _ = fs::remove_file(&local);
        if let Err(code) = step(instance_files_payload(&env, instance, &rendered)) {
            return Ok(code);
        }
        println!("==> instance {n}: game-runtime smoke (V2–V4)");
        if let Err(code) = step(smoke_payload(instance)) {
            return Ok(code);
        }
    }

    println!("==> units, then restart {} game server(s)", instances.len());
    if let Err(code) = step(units_install_payload(&env, &instances)) {
        return Ok(code);
    }
    let before = instance_boot_verdict::probe(&runner, &base, &host, &instances);
    if let Err(code) = step(game_servers_restart_payload(&instances)) {
        return Ok(code);
    }
    let verdicts = instance_boot_verdicts(&runner, &base, &host, &env, &instances, &before);
    println!("==> boot verdicts");
    for verdict in &verdicts {
        println!(
            "  instance {}: {}",
            verdict.instance,
            if verdict.passed { "PASS" } else { "FAIL" }
        );
    }
    if verdicts.iter().any(|verdict| !verdict.passed) {
        return Ok(1);
    }

    if let Some(relay) = &env.fleet.relay
        && let Some(instance) = instances.iter().find(|i| i.number == relay.instance)
    {
        println!("==> relay for instance {}", relay.instance);
        if let Err(code) = step(relay_install_payload(&env.remote_dir, instance, relay)) {
            return Ok(code);
        }
    }
    println!("==> host agents");
    if let Err(code) = step(host_agents_install_payload(&env.remote_dir, &instances)) {
        return Ok(code);
    }

    let mut status = 0;
    for verdict in &verdicts {
        if instance_boot_verdict::instance_log_check(&runner, &base, &host, paths, verdict) != 0 {
            status = 1;
        }
    }
    if status == 0 {
        println!("==> deploy complete: {} instance(s)", instances.len());
    }
    Ok(status)
}
