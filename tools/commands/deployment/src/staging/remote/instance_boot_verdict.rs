//! The boot verdict of every fleet instance, and the log check each passing instance gets last.
//!
//! **Role:** probes every instance's newest log folder and room registration in one round trip
//! ([`boot_progress_payload`]), waits for each instance's NEW boot until `TBD_BOOT_VERIFY_TIMEOUT`,
//! then pulls each new `console.log` and runs the boot verdict `--verify-boot` runs locally
//! ([`instance_boot_verdicts`]); [`instance_log_check`] runs `mod remote-logs --file` over a fresh
//! pull after the host agents are up.
//!
//! **Position:** called by `super::fleet_deploy`, which takes the "before" probe just ahead of the
//! restart; one verdict implementation, two callers (`--verify-boot` and this).
//!
//! **Signals & state:** none held; each call spawns ssh through [`Runner`] and writes one temporary
//! copy of a log, removed after a pass.
//!
//! **Invariants:** a log folder that was already the newest before the restart never counts: the
//! previous boot's log also says "Server registered", and reading it would pass a server that did
//! not come back; an instance with no new log folder fails; the verdict never reads a log the pull
//! did not return whole.

use std::time::Instant;

use super::*;

/// What one boot progress probe saw of one instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootProgress {
    pub instance: u16,
    /// The newest log folder's `console.log` holds the room registration line.
    pub registered: bool,
    /// The newest log folder under the instance's profile, empty when there is none.
    pub newest_log_folder: String,
}

/// One instance's boot verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstanceBootVerdict {
    pub instance: u16,
    /// The new boot's log folder, when the instance wrote one.
    pub log_folder: Option<String>,
    pub passed: bool,
}

const BOOT_PROGRESS: &str = r#"set -u
for n in @INSTANCES@; do
  newest="$(ls -1d "$HOME/@FLEET@/instance-$n/profile/logs/logs_"* 2>/dev/null | tail -1 || true)"
  registered=0
  if [ -n "$newest" ] && grep -qF 'Server registered with address:' "$newest/console.log" 2>/dev/null; then
    registered=1
  fi
  printf '%s %s %s\n' "$n" "$registered" "$newest"
done
"#;

/// The probe: one line per instance, `<number> <0|1 registered> <newest log folder>`.
pub(crate) fn boot_progress_payload(instances: &[FleetInstance]) -> String {
    BOOT_PROGRESS
        .replace(
            "@INSTANCES@",
            &super::super::payloads::instance_numbers(instances),
        )
        .replace(
            "@FLEET@",
            super::super::fleet_instances::FLEET_ROOT_UNDER_HOME,
        )
}

/// The probe's lines; a line that does not parse is skipped.
pub(crate) fn parse_boot_progress(text: &str) -> Vec<BootProgress> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, ' ');
            let instance = fields.next()?.parse().ok()?;
            let registered = fields.next()? == "1";
            Some(BootProgress {
                instance,
                registered,
                newest_log_folder: fields.next().unwrap_or("").trim().to_string(),
            })
        })
        .collect()
}

/// The log folder of the boot the deploy started: the newest folder, when there is one and it is
/// not the folder that was newest before the restart.
pub(crate) fn new_log_folder(before: &[BootProgress], now: &BootProgress) -> Option<String> {
    let previous = before
        .iter()
        .find(|seen| seen.instance == now.instance)
        .map(|seen| seen.newest_log_folder.as_str())
        .unwrap_or("");
    (!now.newest_log_folder.is_empty() && now.newest_log_folder != previous)
        .then(|| now.newest_log_folder.clone())
}

/// One probe round; an ssh failure reads as "nothing seen yet".
pub(super) fn probe(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    instances: &[FleetInstance],
) -> Vec<BootProgress> {
    runner
        .ssh_capture(
            base,
            host,
            &bash_stdin(),
            Some(boot_progress_payload(instances)),
        )
        .map(|(_, out)| parse_boot_progress(&out))
        .unwrap_or_default()
}

/// The instance's `-profile` folder as a path on the host, for the verdict's messages.
fn host_profile(env: &Env, instance: &FleetInstance) -> String {
    let home = env
        .deploy_host
        .home_directory()
        .unwrap_or_else(|| "~".to_string());
    format!("{home}/{}/profile", instance.home_relative_folder())
}

/// `cat` of `log_folder`'s `console.log`, or nothing when the pull failed or came back empty.
fn pull_console_log(runner: &Runner, base: &SshBase, host: &str, log_folder: &str) -> String {
    match runner.ssh_capture(
        base,
        host,
        &[format!("cat '{log_folder}/console.log'")],
        None,
    ) {
        Ok((0, text)) => text,
        _ => String::new(),
    }
}

/// Wait for every instance's new boot, then judge each one. `before` is the probe taken just
/// before the restart.
pub(super) fn instance_boot_verdicts(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
    instances: &[FleetInstance],
    before: &[BootProgress],
) -> Vec<InstanceBootVerdict> {
    println!("==> waiting for every instance's engine to reach a verdict");
    let timeout: u64 = env.boot_verify_timeout.trim().parse().unwrap_or(180);
    let started = Instant::now();
    let booted = |seen: &[BootProgress], instance: &FleetInstance| {
        seen.iter().any(|now| {
            now.instance == instance.number
                && now.registered
                && new_log_folder(before, now).is_some()
        })
    };
    let mut latest: Vec<BootProgress> = Vec::new();
    loop {
        let seen = probe(runner, base, host, instances);
        if !seen.is_empty() {
            latest = seen;
        }
        let waiting: Vec<String> = instances
            .iter()
            .filter(|instance| !booted(&latest, instance))
            .map(|instance| instance.number.to_string())
            .collect();
        let elapsed = started.elapsed().as_secs();
        if waiting.is_empty() || elapsed >= timeout {
            break;
        }
        std::thread::sleep(Duration::from_secs(10));
        println!(
            "    {}s — no new room registration yet from instance {}",
            started.elapsed().as_secs(),
            waiting.join(", ")
        );
    }
    instances
        .iter()
        .map(|instance| judge(runner, base, host, env, instance, before, &latest))
        .collect()
}

/// One instance's verdict over its new boot's log.
fn judge(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
    instance: &FleetInstance,
    before: &[BootProgress],
    latest: &[BootProgress],
) -> InstanceBootVerdict {
    let n = instance.number;
    let profile = host_profile(env, instance);
    let log_folder = latest
        .iter()
        .find(|now| now.instance == n)
        .and_then(|now| new_log_folder(before, now));
    let failed = |log_folder: Option<String>| InstanceBootVerdict {
        instance: n,
        log_folder,
        passed: false,
    };
    let Some(folder) = log_folder else {
        eprintln!("FAIL: instance {n} wrote no new log folder under {profile}/logs.");
        eprintln!("      The unit may not have started. Check:");
        eprintln!(
            "        ssh {host} systemctl --user status {}",
            instance.game_server_unit()
        );
        return failed(None);
    };
    // FAIL-OPEN CLOSED: a pull that fails or returns nothing is refused, never judged.
    let text = pull_console_log(runner, base, host, &folder);
    if text.is_empty() {
        eprintln!("FAIL: could not read {folder}/console.log off {host}.");
        return failed(Some(folder));
    }
    let local_log = std::env::temp_dir().join(format!(
        "tbd-staging-console.{}.instance-{n}.log",
        std::process::id()
    ));
    if fs::write(&local_log, &text).is_err() {
        eprintln!(
            "FAIL: could not stage the pulled log at {}",
            local_log.display()
        );
        return failed(Some(folder));
    }
    println!(
        "==> instance {n}: boot verdict over {folder}/console.log ({} bytes)",
        text.len()
    );
    // Measure the rival ON THE HOST: the profile is a remote path, so the verdict's local `[ -f ]`
    // fallback would answer "absent" for a pak that is really there.
    let rival = runner
        .ssh_capture(
            base,
            host,
            &[format!(
                "wc -c < \"$HOME/{}/profile/addons/TBDFramework_{}/data.pak\" 2>/dev/null || echo 0",
                instance.home_relative_folder(),
                env.addon_guid
            )],
            None,
        )
        .map(|(_, s)| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0".to_string());
    let mut out = Out::streams();
    let code = boot::verify_boot_log(
        &mut out,
        &local_log,
        &env.addon_guid,
        &env.addons_staging,
        &env.admin_count().to_string(),
        &profile,
        Some(&rival),
    );
    if code != 0 {
        eprintln!(
            "FAIL: instance {n} is NOT serving what was deployed, or is not joinable; log kept at {}",
            local_log.display()
        );
        return failed(Some(folder));
    }
    let _ = fs::remove_file(&local_log);
    InstanceBootVerdict {
        instance: n,
        log_folder: Some(folder),
        passed: true,
    }
}

/// `mod remote-logs --file` over a fresh pull of the instance's new `console.log`: the four-outcome
/// check, read through [`super::v6_verdict`]. `0` only for HEALTHY or PARTIAL.
pub(super) fn instance_log_check(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    paths: &Paths,
    verdict: &InstanceBootVerdict,
) -> u8 {
    let n = verdict.instance;
    println!("==> V6 instance {n}: mod remote-logs over its console.log");
    let Some(folder) = verdict.log_folder.as_deref() else {
        eprintln!("V6 instance {n}: no log folder to examine, which is not a pass.");
        return 1;
    };
    let text = pull_console_log(runner, base, host, folder);
    let local_log = std::env::temp_dir().join(format!(
        "tbd-staging-v6.{}.instance-{n}.log",
        std::process::id()
    ));
    if text.is_empty() || fs::write(&local_log, &text).is_err() {
        eprintln!("V6 instance {n}: could not pull {folder}/console.log, so nothing was examined.");
        return 1;
    }
    let out = Run::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "remote-logs",
            "--file",
        ])
        .arg(&local_log)
        .cwd(&paths.mono_root)
        .timeout(Duration::from_secs(3600))
        .merged_output();
    let _ = fs::remove_file(&local_log);
    match out {
        Ok(o) => {
            let _ = io::stdout().write_all(o.text.as_bytes());
            ssh_argv::v6_verdict(o.code)
        }
        // A `NotRun` is the ENVIRONMENT outcome: no log was examined.
        Err(e) => {
            eprintln!("V6 instance {n} could not run: {e:?}");
            1
        }
    }
}
