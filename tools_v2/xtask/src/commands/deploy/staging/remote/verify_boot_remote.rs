use super::*;

/// T-607: assert the boot, do not assume it.
///
/// `systemctl restart` exits 0 over a unit that is already dead — the same defect T-289's agent
/// selftest exists for. And even a genuinely-running server proves nothing about WHICH mod it
/// loaded. Until this block existed the deploy's last word was `sleep 8`, after which it printed a
/// success banner regardless of what the engine did.
///
/// This waits for the engine to get far enough to have decided (addon resolution and room
/// registration both land inside ~20 s of start — measured: config load at +4 s, addons resolved at
/// +7 s, room registered at +14 s on a 2026-08-01 boot), then pulls the log back and runs the same
/// verdict `--verify-boot` runs locally. One implementation, two callers.
pub(super) fn verify_boot_remote(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
) -> Result<Option<u8>> {
    println!("==> waiting for the engine to reach a verdict");
    let timeout: u64 = env.boot_verify_timeout.trim().parse().unwrap_or(180);
    let mut remote_log = String::new();
    let mut waited: u64 = 0;
    while waited < timeout {
        // `|| true` in the bash: a failed probe leaves the previous value. Reproduced by ignoring
        // the Err and keeping `remote_log` as-is.
        if let Ok((_, out)) = runner.ssh_capture(
            base,
            host,
            &[format!(
                "ls -1d '{}'/logs/logs_* 2>/dev/null | tail -1",
                env.profile_dir
            )],
        ) {
            remote_log = out.trim().to_string();
        }
        if !remote_log.is_empty() {
            let probe = runner.ssh_capture(
                base,
                host,
                &[format!(
                    "grep -qF 'Server registered with address:' '{remote_log}/console.log' 2>/dev/null"
                )],
            );
            if matches!(probe, Ok((0, _))) {
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(10));
        waited += 10;
        println!(
            "    {waited}s — no room registration yet (log: {})",
            if remote_log.is_empty() {
                "none"
            } else {
                &remote_log
            }
        );
    }

    if remote_log.is_empty() {
        eprintln!(
            "FAIL: the server produced no log directory under {}/logs after {waited}s.",
            env.profile_dir
        );
        eprintln!("      The unit may not have started at all. Check:");
        eprintln!("        ssh {host} systemctl --user status tbd-reforger.service");
        return Ok(Some(1));
    }

    let local_log =
        std::env::temp_dir().join(format!("tbd-staging-console.{}.log", std::process::id()));
    // FAIL-OPEN CLOSED (3 of 3). The bash was
    //   ssh_cmd "cat '$log/console.log'" > "$_local_log" 2>/dev/null || true
    // which discarded ssh's status entirely; only the follow-up `[ ! -s ]` caught it, so a
    // TRUNCATED-but-non-empty pull (a dropped connection mid-transfer) read as a complete log and
    // the verdict then ran over a partial file. The status is now checked, and a non-zero ssh is
    // the same refusal as an empty file.
    let pulled = runner.ssh_capture(base, host, &[format!("cat '{remote_log}/console.log'")]);
    let text = match pulled {
        Ok((0, t)) => t,
        _ => String::new(),
    };
    if text.is_empty() {
        eprintln!("FAIL: could not read {remote_log}/console.log off {host}.");
        eprintln!("      Refusing to report the deploy OK over a log this script never examined.");
        return Ok(Some(1));
    }
    if fs::write(&local_log, &text).is_err() {
        eprintln!(
            "FAIL: could not stage the pulled log at {}",
            local_log.display()
        );
        return Ok(Some(1));
    }

    let admin_count = env.admin_count();
    println!("    pulled {remote_log}/console.log ({} bytes)", text.len());

    // Measure the rival ON THE HOST. `TBD_PROFILE_DIR` is a remote path, so the local `[ -f ]`
    // fallback inside the verdict would answer "absent" for a pak that is really sitting there, and
    // downgrade a genuine contest to WEAK EVIDENCE on every deploy.
    let rival = runner
        .ssh_capture(
            base,
            host,
            &[format!(
                "wc -c < '{}/addons/TBDFramework_{}/data.pak' 2>/dev/null || echo 0",
                env.profile_dir, env.addon_guid
            )],
        )
        .map(|(_, s)| s.trim().to_string())
        .unwrap_or_default();
    let rival = if rival.is_empty() {
        "0".to_string()
    } else {
        rival
    };

    let mut out = Out::streams();
    if env.server_mode == "config" {
        let rc = boot::verify_boot_log(
            &mut out,
            &local_log,
            &env.addon_guid,
            &env.addons_staging,
            &admin_count.to_string(),
            &env.profile_dir,
            Some(&rival),
        );
        if rc != 0 {
            eprintln!();
            eprintln!(
                "DEPLOY FAILED ITS OWN ACCEPTANCE CHECK. The files are on the host and the unit may"
            );
            eprintln!(
                "be running, but it is NOT serving what you deployed, or it is not joinable."
            );
            eprintln!("Full log kept at: {}", local_log.display());
            return Ok(Some(1));
        }
    } else {
        // addons mode cannot register a room or hold admins by construction, so running the full
        // verdict here would manufacture two guaranteed failures. Assert the half that IS
        // meaningful and say plainly that the rest was not checked, rather than printing green.
        println!(
            "==> boot verdict: {} (mode=addons — addon check only)",
            local_log.display()
        );
        if boot::assert_local_addon_won(&mut out, &local_log, &env.addon_guid, &env.addons_staging)
            != 0
        {
            eprintln!("Full log kept at: {}", local_log.display());
            return Ok(Some(1));
        }
        println!("  SKIP  room + admin checks: addons mode registers no room and loads no server");
        println!("        config. This server is NOT joinable and has NO admins. Use config mode.");
    }
    let _ = fs::remove_file(&local_log);
    Ok(None)
}
