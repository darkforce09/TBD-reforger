//! The payloads of one instance and the secret file check, including a run under a local bash.
use super::*;
use crate::commands::deploy::staging::config::tests::base;
use crate::commands::deploy::staging::fleet_server_config::JOIN_PASSWORD_PLACEHOLDER;

/// A well-formed machine credential; no server accepts it.
const CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn website_api_health_payload_fails_on_an_error_status_and_a_silent_server() {
    let p = website_api_health_payload("http://127.0.0.1:8080/healthz");
    assert_eq!(
        p,
        "set -euo pipefail\n\
         curl -sSf --max-time 10 'http://127.0.0.1:8080/healthz'\n\
         echo\n"
    );
}

#[test]
fn instance_files_are_written_from_host_secrets_and_never_carry_one() {
    let env = base();
    let third = &env.fleet.instances()[2];
    let rendered = format!("{{\"game\": {{\"password\": \"{JOIN_PASSWORD_PLACEHOLDER}\"}}}}\n");
    let p = instance_files_payload(&env, third, &rendered);
    assert!(
        p.starts_with("set -euo pipefail\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\numask 077\n")
    );
    assert!(p.contains("INSTANCE=\"$HOME/tbd/fleet/instance-3\"\n"));
    assert!(p.contains("chmod 700 \"$FLEET\" \"$INSTANCE\" \"$SECRETS\" \"$INSTANCE/profile\"\n"));
    assert!(p.contains(
        "ln -sfn \"/home/deploy/tbd/repo/apps/mod/tbd-framework\" \"/home/deploy/tbd/addons/tbd-framework\"\n"
    ));
    // The RCON password is generated on the host, once, create-exclusive, and read back checked.
    assert!(p.contains("if [ ! -s \"$SECRETS/rcon-password\" ]; then\n"));
    assert!(p.contains(
        "(set -o noclobber; printf '%s\\n' \"$RCON_PASSWORD\" > \"$SECRETS/rcon-password\")\n"
    ));
    assert!(p.contains("RCON_PASSWORD_SHAPE='^[0-9a-f]{32}$'\n"));
    // The credential reaches `setup server-profile` through its environment only.
    assert!(p.contains("TBD_MACHINE_CREDENTIAL=\"$(<\"$SECRETS/mod-runtime-credential\")\"\nexport TBD_MACHINE_CREDENTIAL\n"));
    assert!(p.contains(
        "(cd \"/home/deploy/tbd/repo\" && cargo run -q -p xtask -- setup server-profile \"$INSTANCE/profile\")\nunset TBD_MACHINE_CREDENTIAL\n"
    ));
    assert!(p.contains("\"backendUrl\": \"http://127.0.0.1:8080\""));
    // The profile comes from the one profile writer, which the staging harness's promotion runs.
    let profile = instance_profile_commands(&env.remote_dir, &env.backend_url);
    assert!(p.contains(&format!("\nfi\n{profile}cat > ")), "{p}");
    // The rendered config goes over verbatim and the passwords are filled in by parameter
    // expansion, which puts them in no process's argument vector.
    assert!(p.contains(&format!("<<'CONFIGEOF'\n{rendered}CONFIGEOF\n")));
    assert!(
        p.contains("CONFIG=\"${CONFIG//TBD_RCON_PASSWORD_FROM_HOST_FILE/\"$RCON_PASSWORD\"}\"\n")
    );
    assert!(
        p.contains("CONFIG=\"${CONFIG//TBD_JOIN_PASSWORD_FROM_HOST_FILE/\"$JOIN_PASSWORD\"}\"\n")
    );
    assert!(p.contains("  *_FROM_HOST_FILE*) echo"));
    assert!(p.contains(
        "mv -f \"$INSTANCE/server.config.json.next\" \"$INSTANCE/server.config.json\"\n"
    ));
    assert!(!p.contains("sed -i \"s") && !p.contains("tbdm_"), "{p}");
}

/// The profile writer: the instance's `mod_runtime` credential reaches `setup server-profile`
/// through its environment only, run from the checkout, and the deployment's `backendUrl` follows.
#[test]
fn instance_profile_commands_pass_the_credential_by_environment_then_set_the_backend() {
    assert_eq!(
        instance_profile_commands("/home/deploy/tbd/repo", "https://api.example.org"),
        "TBD_MACHINE_CREDENTIAL=\"$(<\"$SECRETS/mod-runtime-credential\")\"\n\
         export TBD_MACHINE_CREDENTIAL\n\
         (cd \"/home/deploy/tbd/repo\" && cargo run -q -p xtask -- setup server-profile \
         \"$INSTANCE/profile\")\n\
         unset TBD_MACHINE_CREDENTIAL\n\
         sed -i 's|\"backendUrl\": \"[^\"]*\"|\"backendUrl\": \"https://api.example.org\"|' \
         \"$INSTANCE/profile/profile/TBD_BackendConfig.json\"\n"
    );
}

#[test]
fn the_smoke_reads_the_deployment_with_the_host_credential_on_stdin() {
    let p = smoke_payload(&base().fleet.instances()[0]);
    assert!(p.contains(
        "CREDENTIAL=\"$(<\"$HOME/tbd/fleet/instance-1/secrets/mod-runtime-credential\")\"\n"
    ));
    assert!(p.contains("bearer() { printf 'Authorization: Bearer %s\\n' \"$CREDENTIAL\"; }\n"));
    // Every authenticated request takes its header from stdin: `-H @-`, never on the command line.
    assert_eq!(p.matches("bearer | curl").count(), 2);
    assert_eq!(p.matches("-H @-").count(), 2);
    assert!(!p.contains("-H \"Authorization"), "{p}");
    assert!(p.contains("grep -q '\"NO_DEPLOYMENT\"' \"$WORK/deployment.json\" || exit 1"));
    assert!(p.contains("[ \"$ACTUAL\" = \"$EXPECTED\" ]"));
    // V4: an UNAUTHENTICATED read of the same route must answer 401.
    assert!(p.contains("code=$(curl -sS -o /dev/null -w '%{http_code}' \"$API/deployment\")"));
    assert!(p.contains("[ \"$code\" = \"401\" ] || exit 1"));
}

/// Runs the secret file check under a local bash over a scratch HOME: good files pass; a missing,
/// a group-readable and a malformed file each fail; and no secret value ever reaches the output.
#[test]
fn the_secret_file_check_runs_and_never_prints_a_secret() {
    use std::os::unix::fs::PermissionsExt;
    let home = std::env::temp_dir().join(format!("tbd-secret-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let fleet = home.join("tbd/fleet");
    let write = |path: &std::path::Path, text: &str, mode: u32| {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    let instances = base().fleet.instances();
    write(&fleet.join("join-password"), "join-canary-42\n", 0o600);
    for instance in &instances {
        for file in ["mod-runtime-credential", "host-agent-credential"] {
            let path = fleet.join(format!("instance-{}/secrets/{file}", instance.number));
            write(&path, &format!("{CREDENTIAL}\n"), 0o600);
        }
    }
    let run = || {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(fleet_secret_files_check_payload(&instances))
            .env("HOME", &home)
            .output()
            .expect("bash runs")
    };
    let clean = |out: &std::process::Output| {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("join-canary") && !text.contains("tbdm_"),
            "{text}"
        );
        text
    };
    let good = run();
    assert!(good.status.success(), "{}", clean(&good));
    assert_eq!(clean(&good).matches("  ok      ").count(), 11);

    let second_agent = fleet.join("instance-2/secrets/host-agent-credential");
    std::fs::set_permissions(&second_agent, std::fs::Permissions::from_mode(0o640)).unwrap();
    write(
        &fleet.join("instance-4/secrets/mod-runtime-credential"),
        "tbdm_short\n",
        0o600,
    );
    std::fs::remove_file(fleet.join("instance-5/secrets/host-agent-credential")).unwrap();
    write(&fleet.join("join-password"), "has space inside\n", 0o600);
    let bad = run();
    assert_eq!(bad.status.code(), Some(1));
    let text = clean(&bad);
    assert!(
        text.contains("MISSING instance 2 host_agent credential"),
        "{text}"
    );
    assert!(
        text.contains("INVALID instance 4 mod_runtime credential"),
        "{text}"
    );
    assert!(
        text.contains("MISSING instance 5 host_agent credential"),
        "{text}"
    );
    assert!(text.contains("INVALID join password"), "{text}");
    assert!(text.contains("FAIL: 4 secret file(s)"), "{text}");
    assert!(!text.contains("has space"), "{text}");
    std::fs::remove_dir_all(&home).unwrap();
}
