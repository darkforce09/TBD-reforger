use super::*;

/// `render_agent_files` — the agent + its two systemd units into the LOCAL directory `out`.
pub fn render_agent_files(env: &AgentEnv, out: &Path) -> Result<(), u8> {
    env.validate_names()?;
    if let Err(e) = fs::create_dir_all(out) {
        eprintln!("FAIL: could not create {}: {e}", out.display());
        return Err(1);
    }
    let sh = out.join("tbd-reforger-agent.sh");
    write_or_die(&sh, AGENT_SH)?;
    // chmod +x — the selftest runs it through `bash <path>` so the bit is not load-bearing
    // locally, but the deploy `scp`s it and systemd's ExecStart= demands it.
    if let Ok(meta) = fs::metadata(&sh) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o111);
        let _ = fs::set_permissions(&sh, perms);
    }
    write_or_die(&out.join("tbd-reforger-agent.socket"), &env.socket_unit())?;
    write_or_die(
        &out.join("tbd-reforger-agent@.service"),
        &env.service_unit(),
    )?;
    Ok(())
}

pub(crate) fn write_or_die(path: &Path, body: &str) -> Result<(), u8> {
    match fs::write(path, body) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("FAIL: could not write {}: {e}", path.display());
            Err(1)
        }
    }
}

/// Print a `Verdict` exactly as `gate_require`/`gate_ban` did (`FAIL: <msg>` on stdout, six-space
/// continuations) and fold it into the running fail flag.
///
/// This is the whole of the `gate-grep.sh` dependency, inlined. `Verdict` has no bool conversion,
/// so `DidNotRun` cannot silently read as held — which is the upgrade over the bash, where a
/// missing target file printed two lines and returned the same `1` as a real violation.
pub(super) fn note(fail: &mut bool, v: Verdict) {
    match v {
        Verdict::Held => {}
        Verdict::Failed(ref f) | Verdict::DidNotRun(_, ref f) => {
            println!("{f}");
            *fail = true;
        }
    }
}

/// `validate_agent_files` — structural check of a rendered agent.
///
/// Same posture as the server-config validator: re-read the artefact and pin the invariants,
/// rather than trusting that the write above ran.
pub fn validate_agent_files(env: &AgentEnv, d: &Path) -> Result<(), u8> {
    let sh = d.join("tbd-reforger-agent.sh");
    let sock = d.join("tbd-reforger-agent.socket");
    let svc = d.join("tbd-reforger-agent@.service");
    let mut fail = false;

    note(
        &mut fail,
        verification_core::gate::require(
            "agent script missing its state re-read (read_state)",
            &Pattern::literal("read_state"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::require(
            "agent must gate on LoadState, not ActiveState alone",
            &Pattern::literal("LoadState"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::require(
            "socket must be 0600 — the file mode IS the credential",
            &Pattern::literal("SocketMode=0600"),
            &[&sock],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::require(
            "socket must live in %t ($XDG_RUNTIME_DIR)",
            &Pattern::literal("ListenStream=%t/"),
            &[&sock],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::require(
            "service must name the unit it controls",
            &Pattern::literal(&format!("Environment=TBD_AGENT_UNIT={}", env.unit)),
            &[&svc],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::require(
            "service must take the connection on stdin (Accept=yes contract)",
            &Pattern::literal("StandardInput=socket"),
            &[&svc],
        ),
    );
    // The agent must never grow a passthrough. `custom` is operator-supplied free text and the
    // only reason this channel is safe behind a session cookie is that it cannot carry it. Pin
    // the accepted verb set LITERALLY rather than banning the WORD "custom" — the script's own
    // security comment says the word, and a ban that trips on its own documentation is a gate
    // nobody can keep green honestly.
    note(
        &mut fail,
        verification_core::gate::require(
            "agent must accept exactly the four process verbs",
            &Pattern::literal("status|start|stop|restart) ACTION=\"$candidate\" ;;"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::ban(
            "agent must not grow a custom/passthrough case arm",
            &Pattern::literal("custom)"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        verification_core::gate::ban(
            "agent must never eval a request",
            &Pattern::regex("eval[[:space:]]").expect("static ERE"),
            &[&sh],
        ),
    );
    // NOTE from the bash, still true: the default engine is ERE, so this pattern needs no flag.
    // (In bash, passing `-E` would have been consumed as the PATTERN, turning the file into a
    // second pattern — a trap that no longer exists here but explains the shape of the call.)
    note(
        &mut fail,
        verification_core::gate::ban(
            "agent must not derive its verdict from the systemctl exit status",
            &Pattern::regex("verb_rc.*(-eq|==)").expect("static ERE"),
            &[&sh],
        ),
    );

    if fail {
        return Err(1);
    }
    println!(
        "  agent VALID: unit={} socket=%t/{} dwell={}s",
        env.unit, env.socket, env.dwell_s
    );
    Ok(())
}

/// `--render-agent <dir>`.
pub fn render_and_validate(out: &Path) -> Result<u8> {
    let env = AgentEnv::from_env();
    match render_agent_files(&env, out).and_then(|()| validate_agent_files(&env, out)) {
        Ok(()) => Ok(0),
        Err(code) => Ok(code),
    }
}
