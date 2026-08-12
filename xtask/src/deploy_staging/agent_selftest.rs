//! T-289 — the agent selftest (bash lines 413–595), split out of [`super::agent`] for SIZE-3.
//!
//! [`super::agent`] owns the ARTEFACT — what gets rendered and what invariants it must satisfy.
//! This module owns the EVIDENCE — running that artefact against a controllable systemd and
//! asserting it reports the unit's real state. The split follows the bash's own seam: everything
//! here hangs off `--agent-selftest` and nothing here runs during a deploy.
//!
//! WHAT IT PROVES. Case 4 is the ticket: `systemctl restart` EXITS 0 while the unit is `failed`.
//! An agent that trusted that exit status would answer `accepted` over a dead server.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::Result;
use regex::Regex;
use tbd_gate::proc::{self, Run};

use super::agent::{AgentEnv, render_agent_files, validate_agent_files, write_or_die};

// ── the selftest ────────────────────────────────────────────────────────────────────────────

/// One stdin-driven case: name, request, stub LoadState, stub ActiveState, stub verb rc, and the
/// two fields the reply must carry.
struct StdinCase {
    name: &'static str,
    request: &'static str,
    load: &'static str,
    active: &'static str,
    verb_rc: &'static str,
    want_result: &'static str,
    want_state: &'static str,
}

/// Stub `systemctl`. `STUB_LOAD`/`STUB_ACTIVE` are the unit's state; `STUB_VERB_RC` is what the
/// verb returns — deliberately independent, so "verb says OK, unit is dead" is expressible.
///
/// It stays a shell stub because it must be reachable through `PATH` by the *rendered bash agent*,
/// which spawns `systemctl` by name. A Rust stub would have to be built and placed on PATH, which
/// is more moving parts for the same four lines.
const STUB_SYSTEMCTL: &str = r##"#!/usr/bin/env bash
for a in "$@"; do
  if [ "$a" = "show" ]; then
    printf 'LoadState=%s\nActiveState=%s\n' "${STUB_LOAD:-loaded}" "${STUB_ACTIVE:-inactive}"
    exit 0
  fi
done
exit "${STUB_VERB_RC:-0}"
"##;

/// Drive the rendered agent against a STUB systemctl whose answers this function controls.
///
/// WHY A STUB AND NOT THE REAL ONE. The real `systemctl --user` on a dev box has no
/// `tbd-reforger.service`, so every case would collapse to "unreachable" and prove nothing; and
/// the one host that does have it is the live staging server, which this must not touch. The stub
/// lets the interesting states — `active`, `failed`, not-installed — all be produced on demand,
/// locally.
///
/// WHAT IT PROVES. Case 4 is the ticket: `systemctl restart` EXITS 0 while the unit is `failed`.
/// An agent that trusted that exit status would answer `accepted` over a dead server. The
/// assertion demands `rejected` + `state=failed`, so the honest answer passes and the signature
/// defect fails.
pub fn run(d: &Path) -> Result<u8> {
    let env = AgentEnv::from_env();
    let mut pass = 0u32;
    let mut fail = 0u32;

    let _ = fs::remove_dir_all(d);
    if let Err(e) = fs::create_dir_all(d.join("bin")) {
        eprintln!("FAIL: could not create {}: {e}", d.join("bin").display());
        return Ok(1);
    }
    // Render with dwell 0 so the selftest does not sleep 8s per start/restart case.
    //
    // ODDITY PRESERVED: the bash used a one-shot `TBD_AGENT_DWELL_S=0 render_agent_files "$d"`,
    // which scoped the 0 to the render only. The trailing `validate_agent_files` therefore
    // reports the AMBIENT dwell (8), not the 0 that was baked into the rendered @.service. Both
    // numbers are correct about different things and the baseline prints `dwell=8s`, so the split
    // is reproduced rather than tidied.
    let render_env = AgentEnv {
        dwell_s: "0".to_string(),
        ..env.clone()
    };
    if let Err(code) = render_agent_files(&render_env, d) {
        return Ok(code);
    }

    let stub = d.join("bin/systemctl");
    if let Err(code) = write_or_die(&stub, STUB_SYSTEMCTL) {
        return Ok(code);
    }
    if let Ok(meta) = fs::metadata(&stub) {
        let mut p = meta.permissions();
        p.set_mode(0o755);
        let _ = fs::set_permissions(&stub, p);
    }

    // One row per bash `cases=()` entry — kept as a TABLE so the two can be diffed by eye.
    #[rustfmt::skip]
    let cases = [
        StdinCase { name: "status of a running unit", request: "status", load: "loaded", active: "active", verb_rc: "0", want_result: "accepted", want_state: "active" },
        StdinCase { name: "status of a stopped unit", request: "status", load: "loaded", active: "inactive", verb_rc: "0", want_result: "accepted", want_state: "inactive" },
        StdinCase { name: "restart that really came up", request: "restart", load: "loaded", active: "active", verb_rc: "0", want_result: "accepted", want_state: "active" },
        StdinCase { name: "restart that exits 0 over a DEAD unit", request: "restart", load: "loaded", active: "failed", verb_rc: "0", want_result: "rejected", want_state: "failed" },
        StdinCase { name: "start that never came up", request: "start", load: "loaded", active: "inactive", verb_rc: "0", want_result: "rejected", want_state: "inactive" },
        StdinCase { name: "stop that really stopped", request: "stop", load: "loaded", active: "inactive", verb_rc: "0", want_result: "accepted", want_state: "inactive" },
        StdinCase { name: "unit not installed", request: "status", load: "not-found", active: "inactive", verb_rc: "0", want_result: "unreachable", want_state: "unknown" },
        StdinCase { name: "unit masked", request: "restart", load: "masked", active: "inactive", verb_rc: "0", want_result: "unreachable", want_state: "unknown" },
        StdinCase { name: "garbage verb is refused", request: "rm -rf /", load: "loaded", active: "active", verb_rc: "0", want_result: "rejected", want_state: "unknown" },
        StdinCase { name: "empty request is refused", request: "", load: "loaded", active: "active", verb_rc: "0", want_result: "rejected", want_state: "unknown" },
    ];

    let bin_path = format!(
        "{}:{}",
        d.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let agent = d.join("tbd-reforger-agent.sh");

    for c in &cases {
        let out = Run::new("bash")
            .arg(&agent)
            .stdin(format!("{}\n", c.request))
            .env("PATH", bin_path.clone())
            .env("TBD_AGENT_UNIT", env.unit.clone())
            .env("TBD_AGENT_DWELL_S", "0")
            .env("STUB_LOAD", c.load)
            .env("STUB_ACTIVE", c.active)
            .env("STUB_VERB_RC", c.verb_rc)
            .timeout(Duration::from_secs(60))
            .output();
        // `2>/dev/null` in the bash: stderr was discarded, only stdout parsed. A NotRun (bash
        // absent, signal, timeout) yields an empty reply here, which fails the comparison — the
        // bash would have compared against an empty `$out` for the same reason.
        let reply = out
            .map(|o| o.stdout.trim_end().to_string())
            .unwrap_or_default();
        let got_result = capture(r#""result":"([a-z]*)""#, &reply);
        let got_state = capture(r#""state":"([a-z-]*)""#, &reply);
        if got_result == c.want_result && got_state == c.want_state {
            println!("  PASS  {} -> {got_result}/{got_state}", c.name);
            pass += 1;
        } else {
            println!("  FAIL  {}", c.name);
            println!(
                "        want result={} state={}",
                c.want_result, c.want_state
            );
            println!("        got  result={got_result} state={got_state}");
            println!("        raw  {reply}");
            fail += 1;
        }
    }

    // FAIL-OPEN CLOSED (1 of 2 in this module). The bash wrapped this case in
    // `if command -v python3`, so on a box without python3 the check vanished AND the summary
    // still read "N passed, 0 failed" — a smaller denominator reported as a clean sweep. The
    // parse is `serde_json`, compiled in, so the case can no longer be skipped.
    {
        let out = Run::new("bash")
            .arg(&agent)
            .stdin("status\n")
            .env("PATH", bin_path.clone())
            .env("TBD_AGENT_UNIT", env.unit.clone())
            .env("STUB_LOAD", "loaded")
            .env("STUB_ACTIVE", "active")
            .timeout(Duration::from_secs(60))
            .output();
        let reply = out
            .map(|o| o.stdout.trim_end().to_string())
            .unwrap_or_default();
        if json_has_exactly_contract_keys(&reply) {
            println!("  PASS  response parses as JSON with exactly the contract keys");
            pass += 1;
        } else {
            println!("  FAIL  response is not valid contract JSON: {reply}");
            fail += 1;
        }
    }

    // ── The socket half ─────────────────────────────────────────────────────────────────────
    //
    // Everything above drives the agent as a stdin/stdout filter, which proves the LOGIC and
    // nothing about the CHANNEL. This block opens a real AF_UNIX socket, activates the agent
    // through it exactly as `Accept=yes` + `StandardInput=socket` will on the host, and reads the
    // reply back off the wire. Without it, "the agent works" would be a claim about a program
    // nobody ever connected to.
    //
    // FAIL CLOSED on a missing tool rather than skipping. `systemd-socket-activate` ships in the
    // base systemd package, and this program installs systemd units for a living — an environment
    // without it cannot validate this artefact, and should say so rather than print a green line
    // about a check that did not execute.
    //
    // The bash's third guard, `command -v python3` (it was the test client), is GONE: the client
    // is now `std::os::unix::net::UnixStream`. That removes a branch that could only ever have
    // failed the run for a reason unrelated to the artefact.
    if proc::which("systemd-socket-activate").is_err() {
        println!("  FAIL  socket round-trip — systemd-socket-activate not found.");
        println!("        Refusing to report the channel OK without ever opening it.");
        fail += 1;
    } else if proc::which("setsid").is_err() {
        println!(
            "  FAIL  socket round-trip — setsid not found (see the note below on why it is required)."
        );
        fail += 1;
    } else {
        // name | STUB_ACTIVE | request | expected result | expected state
        let sock_cases = [
            (
                "socket round-trip, healthy unit",
                "active",
                "restart",
                "accepted",
                "active",
            ),
            (
                "socket round-trip, systemctl exits 0 over a DEAD unit",
                "failed",
                "restart",
                "rejected",
                "failed",
            ),
        ];
        for (i, (name, active, request, want_r, want_s)) in sock_cases.iter().enumerate() {
            let sock_path = d.join(format!("agent-{}.sock", i + 1));
            let _ = fs::remove_file(&sock_path);
            let log = d.join(format!("activate-{}.log", i + 1));

            // setsid is LOAD-BEARING, not tidiness. `systemd-socket-activate` re-broadcasts a
            // received SIGTERM to its whole process GROUP, so a plain SIGTERM from here would
            // reach this process too — measured in the bash: the selftest exited 143 with the
            // socket cases never reported. Its own session means teardown can only reach the
            // listener. (This port additionally kills with SIGKILL via `Child::kill`, which
            // cannot be re-broadcast at all; setsid is kept because it also contains anything
            // systemd-socket-activate itself spawns.)
            let logf = fs::File::create(&log).ok();
            let mut cmd = Command::new("setsid");
            cmd.arg("systemd-socket-activate")
                .arg(format!("--listen={}", sock_path.display()))
                .arg("--accept")
                .arg("--inetd")
                .arg("-E")
                .arg(format!("PATH={bin_path}"))
                .arg("-E")
                .arg(format!("TBD_AGENT_UNIT={}", env.unit))
                .arg("-E")
                .arg("STUB_LOAD=loaded")
                .arg("-E")
                .arg(format!("STUB_ACTIVE={active}"))
                .arg("-E")
                .arg("STUB_VERB_RC=0")
                .arg("-E")
                .arg("TBD_AGENT_DWELL_S=0")
                .arg("--")
                .arg("bash")
                .arg(&agent)
                .stdin(Stdio::null());
            match logf {
                Some(f) => {
                    let f2 = f.try_clone().ok();
                    cmd.stdout(Stdio::from(f));
                    if let Some(f2) = f2 {
                        cmd.stderr(Stdio::from(f2));
                    }
                }
                None => {
                    cmd.stdout(Stdio::null()).stderr(Stdio::null());
                }
            }
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    println!("  FAIL  {name}");
                    println!("        could not spawn the listener: {e}");
                    fail += 1;
                    continue;
                }
            };

            // Wait for the listener rather than sleeping a guess (bash: 50 × 0.1s).
            let deadline = Instant::now() + Duration::from_secs(5);
            while !sock_path.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(100));
            }

            let reply = socket_round_trip(&sock_path, request)
                .unwrap_or_else(|e| format!("CLIENT-ERROR {e}"));
            let _ = child.kill();
            let _ = child.wait();

            let got_result = capture(r#""result":"([a-z]*)""#, &reply);
            let got_state = capture(r#""state":"([a-z-]*)""#, &reply);
            if got_result == *want_r && got_state == *want_s {
                println!("  PASS  {name} -> {got_result}/{got_state}");
                pass += 1;
            } else {
                println!("  FAIL  {name}");
                println!("        want result={want_r} state={want_s}");
                println!("        got  result={got_result} state={got_state}");
                println!("        raw  {reply}");
                fail += 1;
            }
        }
    }

    // The rendered units must be units systemd itself accepts, not just files that grep right.
    //
    // This one KEEPS its `command -v` guard, unlike the JSON case above, because it tests the
    // HOST's systemd rather than our artefact — a container without systemd-analyze is not
    // evidence about the unit file. But the skip is now SAID OUT LOUD instead of silently
    // shrinking the denominator, which was the honest half of the bash's fail-open.
    if proc::which("systemd-analyze").is_ok() {
        let unit = d.join("tbd-reforger-agent.socket");
        let out = Run::new("systemd-analyze")
            .arg("verify")
            .arg("--user")
            .arg(&unit)
            .timeout(Duration::from_secs(120))
            .merged_output();
        match out {
            Ok(o) if o.code == 0 => {
                println!("  PASS  systemd-analyze verify accepts the rendered socket unit");
                pass += 1;
            }
            Ok(o) => {
                println!("  FAIL  systemd-analyze rejected the rendered socket unit:");
                for line in o.text.lines() {
                    println!("        {line}");
                }
                fail += 1;
            }
            Err(e) => {
                println!("  FAIL  systemd-analyze could not run: {e:?}");
                fail += 1;
            }
        }
    } else {
        println!("  SKIP  systemd-analyze not present — the rendered unit was NOT offered to");
        println!("        systemd itself. This is a check that did not run, not a pass.");
    }

    println!();
    if fail != 0 {
        println!("AGENT SELFTEST: {pass} passed, {fail} FAILED");
        return Ok(1);
    }
    println!("AGENT SELFTEST: {pass} passed, 0 failed");
    match validate_agent_files(&env, d) {
        Ok(()) => Ok(0),
        Err(code) => Ok(code),
    }
}

/// The python3 socket client, replaced by the standard library.
///
/// Timeouts match the bash client's `s.settimeout(30)`. `shutdown(SHUT_WR)` is load-bearing: the
/// agent's `read -r request` only returns on a newline or EOF, and `--inetd` wires the same fd to
/// both directions, so a client that never half-closes would deadlock against an agent waiting
/// for more input.
fn socket_round_trip(sock: &Path, request: &str) -> std::io::Result<String> {
    let mut s = UnixStream::connect(sock)?;
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    s.set_write_timeout(Some(Duration::from_secs(30)))?;
    s.write_all(request.as_bytes())?;
    s.write_all(b"\n")?;
    s.shutdown(std::net::Shutdown::Write)?;
    let mut buf = vec![0u8; 4096];
    let n = s.read(&mut buf)?;
    buf.truncate(n);
    Ok(String::from_utf8_lossy(&buf).trim().to_string())
}

/// `sed -n 's/.*"result":"\([a-z]*\)".*/\1/p'`, faithfully.
///
/// The bash sed had a GREEDY leading `.*`, so it captured the LAST match on the line; `.last()`
/// reproduces that. No match yields the empty string, exactly as `sed -n` printing nothing did.
fn capture(pat: &str, subject: &str) -> String {
    let re = Regex::new(pat).expect("static pattern");
    re.captures_iter(subject)
        .last()
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default()
}

/// The python3 one-liner
/// `d=json.load(sys.stdin); assert set(d)=={"ok","action","result","state","detail"}`.
fn json_has_exactly_contract_keys(reply: &str) -> bool {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(reply)
    else {
        return false;
    };
    let want = ["ok", "action", "result", "state", "detail"];
    map.len() == want.len() && want.iter().all(|k| map.contains_key(*k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_reproduces_the_greedy_sed() {
        let reply = r#"{"ok":true,"action":"status","result":"accepted","state":"active","detail":"observed"}"#;
        assert_eq!(capture(r#""result":"([a-z]*)""#, reply), "accepted");
        assert_eq!(capture(r#""state":"([a-z-]*)""#, reply), "active");
        // No match prints nothing under `sed -n`, i.e. the empty string.
        assert_eq!(capture(r#""result":"([a-z]*)""#, "CLIENT-ERROR nope"), "");
        // Greedy leading `.*` takes the LAST occurrence; pinned so a future rewrite to `find()`
        // does not silently change which one wins.
        assert_eq!(
            capture(
                r#""result":"([a-z]*)""#,
                r#""result":"first" "result":"second""#
            ),
            "second"
        );
        // `state` accepts a hyphen (`not-found` never reaches the wire, but `[a-z-]` is the
        // bash's class and a narrower one would silently drop a future state name).
        assert_eq!(
            capture(r#""state":"([a-z-]*)""#, r#""state":"not-found""#),
            "not-found"
        );
    }

    #[test]
    fn contract_key_check_is_exact() {
        assert!(json_has_exactly_contract_keys(
            r#"{"ok":true,"action":"status","result":"accepted","state":"active","detail":"x"}"#
        ));
        // Extra key -> not the contract.
        assert!(!json_has_exactly_contract_keys(
            r#"{"ok":true,"action":"s","result":"a","state":"a","detail":"x","extra":1}"#
        ));
        // Missing key -> not the contract.
        assert!(!json_has_exactly_contract_keys(r#"{"ok":true}"#));
        // Not JSON at all — the case the bash could only detect when python3 happened to exist.
        assert!(!json_has_exactly_contract_keys("CLIENT-ERROR nope"));
        assert!(!json_has_exactly_contract_keys(""));
    }

    #[test]
    fn stub_systemctl_can_express_a_zero_exit_over_a_dead_unit() {
        // The stub is the whole reason case 4 is expressible: STUB_VERB_RC and STUB_ACTIVE are
        // independent, so "the verb says OK and the unit is failed" is a state it can produce.
        assert!(STUB_SYSTEMCTL.contains("${STUB_VERB_RC:-0}"));
        assert!(STUB_SYSTEMCTL.contains("${STUB_ACTIVE:-inactive}"));
        assert!(STUB_SYSTEMCTL.contains("${STUB_LOAD:-loaded}"));
    }
}
