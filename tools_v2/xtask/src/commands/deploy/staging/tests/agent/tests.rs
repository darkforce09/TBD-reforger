use super::*;
use regex::Regex;

pub(super) fn env() -> AgentEnv {
    AgentEnv {
        unit: "tbd-reforger.service".into(),
        socket: "tbd-reforger-agent.sock".into(),
        dwell_s: "8".into(),
        remote_path: "/home/sam/tbd/tbd-reforger-agent.sh".into(),
        install: false,
    }
}

#[test]
fn agent_script_is_the_quoted_heredoc_verbatim() {
    // Byte-level pins against the bash source. The heredoc delimiter was QUOTED, so any `${}`
    // in here is literal shell for the host to expand, never something this renderer filled.
    assert!(AGENT_SH.starts_with("#!/usr/bin/env bash\n"));
    assert!(AGENT_SH.ends_with("esac\n"));
    assert!(AGENT_SH.contains(r#"UNIT="${TBD_AGENT_UNIT:-}""#));
    assert!(AGENT_SH.contains("read_state"));
    assert!(AGENT_SH.contains("LoadState"));
    assert!(AGENT_SH.contains(r#"status|start|stop|restart) ACTION="$candidate" ;;"#));
    // The security property, asserted here as well as by validate_agent_files, because a
    // regression in the CONSTANT would otherwise only be caught at runtime.
    assert!(!AGENT_SH.contains("custom)"));
    assert!(!Regex::new("eval[[:space:]]").unwrap().is_match(AGENT_SH));
    assert!(!Regex::new("verb_rc.*(-eq|==)").unwrap().is_match(AGENT_SH));
}

#[test]
fn units_render_byte_for_byte() {
    let e = env();
    assert_eq!(
        e.socket_unit(),
        "[Unit]\nDescription=TBD Reforger host control agent socket (T-289)\n\
         Documentation=man:systemd.socket(5)\n\n[Socket]\n\
         ListenStream=%t/tbd-reforger-agent.sock\nSocketMode=0600\nAccept=yes\n\n\
         [Install]\nWantedBy=sockets.target\n"
    );
    assert_eq!(
        e.service_unit(),
        "[Unit]\nDescription=TBD Reforger host control agent connection (T-289)\n\
         Documentation=man:systemd.socket(5)\n\n[Service]\nType=oneshot\n\
         ExecStart=/home/sam/tbd/tbd-reforger-agent.sh\n\
         Environment=TBD_AGENT_UNIT=tbd-reforger.service\n\
         Environment=TBD_AGENT_DWELL_S=8\n\
         StandardInput=socket\nStandardOutput=socket\nStandardError=journal\n"
    );
}

#[test]
fn name_validation_fails_closed_on_injection() {
    let mut e = env();
    e.unit = "a\nExecStart=/bin/sh".into();
    assert!(e.validate_names().is_err());
    e = env();
    e.unit = String::new();
    assert!(e.validate_names().is_err());
    // `@` is legal in a unit name (systemd's template separator) and NOT in the socket file
    // name — the two charsets differ in the bash and the difference is deliberate.
    e = env();
    e.unit = "tbd@.service".into();
    assert!(e.validate_names().is_ok());
    e = env();
    e.socket = "tbd@.sock".into();
    assert!(e.validate_names().is_err());
}

#[test]
fn validate_rejects_a_tampered_agent() {
    // ANTI-VACUITY: the validator must be observed FAILING, or "agent VALID" means nothing.
    let d = std::env::temp_dir().join(format!("tbd-t853-agent-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let e = env();
    render_agent_files(&e, &d).expect("render");
    assert!(
        validate_agent_files(&e, &d).is_ok(),
        "clean render must pass"
    );

    // Add the passthrough arm the ban exists to catch.
    let sh = d.join("tbd-reforger-agent.sh");
    let mut body = fs::read_to_string(&sh).unwrap();
    body.push_str("\ncase $x in\n  custom) do_the_bad_thing ;;\nesac\n");
    fs::write(&sh, &body).unwrap();
    assert!(validate_agent_files(&e, &d).is_err(), "ban must fire");

    // A DELETED artefact must not read as clean — the gate-grep.sh hole this port inherits
    // the fix for. `Verdict::DidNotRun` is a distinct variant, so it cannot fold into Held.
    fs::remove_file(&sh).unwrap();
    assert!(
        validate_agent_files(&e, &d).is_err(),
        "missing target must fail"
    );
    let _ = fs::remove_dir_all(&d);
}
