//! T-289 — the host control agent (bash lines 103–679).
//!
//! WHAT THIS IS. `POST /api/v1/admin/servers/:id/rcon` answers 503 `RCON_NO_TRANSPORT`
//! (`handlers/admin.rs:551`) because the API has no channel to the game host. This module renders
//! the host half of that channel. The API half is a separate slice — the bash carried a full
//! specification for it in a 70-line comment block; it is kept verbatim in
//! [`API_SLICE_SPEC`] rather than summarised away, because it is the only written record of a
//! design decision (same-uid UNIX socket beats a credential column) that a future reader will
//! otherwise re-litigate.
//!
//! ── THE FACT THAT DECIDES THE DESIGN ─────────────────────────────────────────────────────────
//!
//! T-269 recorded "The game server is a **separate host**" (`admin.rs:517`). That is true of the
//! DEVELOPER'S PC and false of the API. Re-measured on main 2026-07-31: `docs/mod/
//! STAGING-SERVER.md:3` puts "API + Postgres and Arma Reforger dedicated server" on
//! `sam@192.168.0.140`; `scripts/deploy/deploy.env.example:17` gives ONE ssh host for BOTH deploy
//! scripts; `docs/website/HOME_SERVER.md:282-304` makes the API
//! `~/.config/systemd/user/tbd-website-api.service`; this port restarts
//! `tbd-reforger.service` through the same `systemctl --user`; and `TBD_BACKEND_URL` defaults to
//! `http://127.0.0.1:8080` — the mod reaches the API on LOOPBACK.
//!
//! So the API process and the game server are SIBLING `systemctl --user` units, same uid (`sam`),
//! same user systemd manager, same `$XDG_RUNTIME_DIR`. Only Postgres is in Docker.
//!
//! THAT COLLAPSES THE CREDENTIAL PROBLEM. Across a same-uid UNIX socket the OPERATING SYSTEM is
//! the credential: a socket at `$XDG_RUNTIME_DIR` with `SocketMode=0600` can be opened by exactly
//! one uid, and that uid is the API's. There is no shared secret to store, rotate, or leak.
//!
//! ── WHAT WAS REJECTED ────────────────────────────────────────────────────────────────────────
//!
//! * SSH from an axum handler. `send_rcon` is gated by `AdminUser` and `RconCommand::Custom`
//!   (`admin.rs:493-510`) carries operator-supplied free text — that is remote code execution
//!   with an admin checkbox in front of it. It is also not possible on the box: `deploy.env` is
//!   gitignored AND rsync-excluded, so the credential exists only on a developer's PC.
//! * BattlEye / Reforger RCON over UDP. Re-measured: `ss -lntu` binds only :8080 / :3000 / :5434
//!   (+ :5432) — 19999 is never bound; the renderer emits NO `rcon` key and `"battlEye": false`.
//!   DECISIVE: RCON only reaches a server that is ALREADY RUNNING. It structurally cannot do
//!   `start`, which is half this ticket's title.
//! * A queued-command table the mod polls. Needs a migration plus mod-side polling that does not
//!   exist — and a dead server polls nothing, so again it cannot `start`.
//!
//! ── WHY THE AGENT RE-READS THE UNIT, WHICH IS THE ENTIRE POINT ───────────────────────────────
//!
//! `systemctl --user restart tbd-reforger.service` EXITS 0 OVER A SERVER THAT IS DEAD. Not
//! hypothetical on this host — `docs/mod/STAGING-SERVER.md:246-250` documents it: with `-a2sPort`
//! equal to `-bindPort` the engine logs "Unable to start replication" → "Game destroyed" and
//! "exits status 0, so `Restart=on-failure` does NOT restart it". The deploy path has always run
//! that restart and then `sleep 8` without ever checking — a tool reporting success over a server
//! it never examined, which is this program's signature defect, already live in this file.
//!
//! So the agent NEVER derives its answer from the exit status of the verb. It runs the verb,
//! waits out the dwell, and RE-READS the unit's LoadState/ActiveState from systemd.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Result;
use tbd_gate::{Pattern, Verdict};

/// The rendered agent, byte-for-byte the bash `<<'AGENT_EOF'` heredoc.
///
/// The heredoc delimiter was QUOTED, so bash performed no substitution inside it: the agent
/// script is byte-identical on every host and the per-server addressing lives in the systemd
/// unit, which is systemd's own place for it. This constant therefore has no `{}` formatting and
/// must not grow any.
///
/// The agent is a pure stdin→stdout filter and holds NO socket code, because bash cannot bind a
/// UNIX socket without pulling in socat/nc. systemd's `Accept=yes` socket activation supplies the
/// connection on stdin, which removes the dependency instead of asserting it.
///
/// It stays SHELL rather than becoming Rust: it is an artefact *deployed to the game host* and
/// executed by systemd there, not part of this toolchain. Porting it would mean shipping a
/// compiled binary through rsync and rebuilding it per host arch, to replace 100 lines that need
/// nothing but `systemctl show`.
const AGENT_SH: &str = r##"#!/usr/bin/env bash
# TBD Reforger host control agent (T-289) — RENDERED by scripts/mod/deploy-staging.sh.
# Do not edit on the host; edit the renderer and redeploy.
#
# Contract: read ONE line from stdin, write ONE line of JSON to stdout.
#
#   in : status | start | stop | restart
#   out: {"ok":<bool>,"action":"<verb>","result":"<r>","state":"<s>","detail":"<text>"}
#
#   result  accepted    the verb ran AND the unit was observed in the intended state
#           rejected    the verb is unknown, or it ran and the unit did NOT get there
#           unreachable systemd could not be reached, or the unit is not installed
#   state   systemd ActiveState as observed AFTER the action: active | inactive |
#           failed | activating | deactivating | reloading | unknown
#
# SECURITY. There is deliberately NO passthrough verb. The request is filtered to [a-z]
# and then matched against a fixed four-element set, so no operator-supplied text — and no
# shell metacharacter — can reach a command. `custom` and `change_map` from RconInput have
# no representation here BY DESIGN; see the scope note in deploy-staging.sh.
set -uo pipefail

UNIT="${TBD_AGENT_UNIT:-}"
SYSTEMCTL="${TBD_AGENT_SYSTEMCTL:-systemctl}"
DWELL="${TBD_AGENT_DWELL_S:-8}"
ACTION="unknown"

# The only variable content in the output is $detail. Restrict it to a charset containing
# no JSON metacharacter, so this hand-rolled JSON cannot emit an invalid document — the
# failure deploy-staging.sh's own header warns about. Every other field is from a fixed set.
emit() {
  local ok="$1" result="$2" state="$3" detail="$4"
  detail="$(printf '%s' "$detail" | tr -cd 'A-Za-z0-9 ._:/@=-' | cut -c1-200)"
  printf '{"ok":%s,"action":"%s","result":"%s","state":"%s","detail":"%s"}\n' \
    "$ok" "$ACTION" "$result" "$state" "$detail"
}

# Read LoadState and ActiveState in one call and parse BY KEY — `systemctl show` does not
# promise the properties come back in the order they were asked for.
#
# LoadState matters on its own: `systemctl show` on a unit that does not exist still exits
# 0 and reports ActiveState=inactive. Trusting ActiveState alone would report a UNINSTALLED
# server as merely "stopped", which is the same class of lie this agent exists to end.
read_state() {
  local raw line load="" active=""
  raw="$("$SYSTEMCTL" --user show --property=LoadState --property=ActiveState -- "$UNIT" 2>/dev/null)" || return 1
  while IFS= read -r line; do
    case "$line" in
      LoadState=*)   load="${line#LoadState=}" ;;
      ActiveState=*) active="${line#ActiveState=}" ;;
    esac
  done <<< "$raw"
  [ -n "$load" ] || return 1
  case "$load" in
    loaded) ;;
    *) printf 'NOTLOADED %s' "$load"; return 0 ;;
  esac
  case "$active" in
    active|inactive|failed|activating|deactivating|reloading) printf 'OK %s' "$active" ;;
    *) printf 'OK unknown' ;;
  esac
}

read -r request || request=""
# Filter to lowercase letters BEFORE matching: strips CR from a \r\n client, trailing
# whitespace, and anything else. "rm -rf /" becomes "rmrf", which is not in the set below.
candidate="$(printf '%s' "$request" | tr -cd 'a-z')"
case "$candidate" in
  status|start|stop|restart) ACTION="$candidate" ;;
  *) emit false rejected unknown "unknown action"; exit 0 ;;
esac

if [ -z "$UNIT" ]; then
  emit false unreachable unknown "TBD_AGENT_UNIT not set in the service unit"
  exit 0
fi
if ! command -v "$SYSTEMCTL" >/dev/null 2>&1; then
  emit false unreachable unknown "systemctl not available"
  exit 0
fi

probe="$(read_state)" || { emit false unreachable unknown "systemd did not answer"; exit 0; }
case "$probe" in
  NOTLOADED*) emit false unreachable unknown "unit not installed: ${probe#NOTLOADED }"; exit 0 ;;
esac

# `status` only observes; the caller reads `state` for the answer.
if [ "$ACTION" = "status" ]; then
  emit true accepted "${probe#OK }" "observed"
  exit 0
fi

verb_rc=0
"$SYSTEMCTL" --user "$ACTION" -- "$UNIT" >/dev/null 2>&1 || verb_rc=$?

# THE DWELL. Not politeness — a Reforger server that mis-starts exits 0 a few seconds in
# (STAGING-SERVER.md:246-250), so a state read taken immediately after `start` returns
# `active` for a server that is already dying. Reading the state only AFTER the dwell is
# what makes `accepted` mean something.
if [ "$ACTION" != "stop" ] && [ "$DWELL" != "0" ]; then
  sleep "$DWELL"
fi

probe="$(read_state)" || { emit false unreachable unknown "systemd did not answer after $ACTION"; exit 0; }
case "$probe" in
  NOTLOADED*) emit false unreachable unknown "unit vanished during $ACTION"; exit 0 ;;
esac
state="${probe#OK }"

# The verdict is the OBSERVED state, never $verb_rc. A zero exit over a dead unit is
# exactly the defect this agent exists to stop reporting.
case "$ACTION" in
  start|restart)
    if [ "$state" = "active" ]; then
      emit true accepted "$state" "unit active after $ACTION"
    else
      emit false rejected "$state" "unit is $state after $ACTION; systemctl rc=$verb_rc"
    fi ;;
  stop)
    if [ "$state" = "inactive" ] || [ "$state" = "failed" ]; then
      emit true accepted "$state" "unit stopped"
    else
      emit false rejected "$state" "unit is $state after stop; systemctl rc=$verb_rc"
    fi ;;
esac
"##;

/// The API-side specification the bash carried inline (lines 597–666), preserved verbatim.
///
/// It is a `const` rather than a comment so that `grep -r "WHAT THE API SLICE MUST BUILD"` keeps
/// working for whoever picks up the API half, exactly as it did against the shell script.
#[allow(dead_code)]
pub const API_SLICE_SPEC: &str = "\
── WHAT THE API SLICE MUST BUILD ────────────────────────────────────────────

apps/website/api/** is NOT this slice's to touch. The host half above is complete and
proven; the API half is mechanical from here.

1. CONFIG — one new var in apps/website/api/src/config.rs:
      game_agent_socket: env::var(\"GAME_AGENT_SOCKET\").unwrap_or_default()
   Empty = no transport, and `send_rcon` keeps answering 503. Fail closed. Populate it in
   the API's systemd unit (docs/website/HOME_SERVER.md:282) as %t/tbd-reforger-agent.sock.

2. CLIENT — new apps/website/api/src/services/game_agent.rs. No new dependency: tokio is
   already in the tree and `tokio::net::UnixStream` is all this needs.
      pub enum AgentAction { Status, Start, Stop, Restart }   // Display -> the wire verb
      #[derive(Deserialize)] pub struct AgentReply {
          pub ok: bool, pub action: String, pub result: AgentResult,
          pub state: String, pub detail: String }
      #[derive(Deserialize)] #[serde(rename_all=\"lowercase\")]
      pub enum AgentResult { Accepted, Rejected, Unreachable }
      pub async fn send(sock: &Path, a: AgentAction) -> anyhow::Result<AgentReply>
   Body: connect, write \"<verb>\\n\", read exactly one line, serde_json::from_str.
   TIMEOUT MUST EXCEED THE DWELL — the agent sleeps TBD_AGENT_DWELL_S (default 8) before
   answering start/restart, on purpose. Use 20s. A timeout shorter than the dwell would
   turn every honest slow answer into a false \"unreachable\".

3. HANDLER — apps/website/api/src/handlers/admin.rs `send_rcon` (currently ends in the
   unconditional Err(SERVICE_UNAVAILABLE, RCON_NO_TRANSPORT) at :628). Map the validated
   RconCommand, then map the reply — the mapping is three-way, because that is the delivery
   result T-269 asked for:
      RconCommand::Restart                   -> AgentAction::Restart
      RconCommand::Kick / ChangeMap / Custom -> STILL 503, unchanged (see SCOPE GAP)
      AgentResult::Accepted    -> 202 {\"accepted\":true,\"delivered\":true,\"state\":<state>}
      AgentResult::Rejected    -> 409 — the agent ran it and the unit did NOT get there
      AgentResult::Unreachable -> 503 RCON_NO_TRANSPORT
      transport error/timeout  -> 503, same shape
   THE AUDIT ROW MUST RECORD THE OUTCOME, NOT THE ATTEMPT — that is the specific defect
   T-269 called out. Write it AFTER the agent answers, Info on Accepted and Warn otherwise,
   with the observed `state` in the detail.

4. ADDRESSING — for THIS deployment nothing is needed in the `servers` table: one host, one
   socket, path from config. The migration becomes REQUIRED the moment a second game host
   exists, and then it is:
      ALTER TABLE servers ADD COLUMN agent_socket text;   -- local socket path, or
      ALTER TABLE servers ADD COLUMN agent_endpoint text; -- host:port for a remote agent
   plus a real credential column for the remote case, because the OS stops vouching for the
   peer the moment the channel leaves the box.

── SCOPE GAP, DECIDED ───────────────────────────────────────────────────────

* restart / start / stop / status are the unit's lifecycle. The agent covers them completely
  and safely, and only these four are reachable over the socket.
* change_map and custom need a live admin channel INTO a running server. Nothing in this repo
  has one. Either is strictly larger than this ticket and must not be smuggled into the agent
  — the agent's safety argument rests entirely on it accepting no free text.
* kick CANNOT BE BUILT AT ALL YET: `RconInput` has no player field (admin.rs:422-428), so
  apps/website/frontend/src/server_control.rs:44 posts a bare {\"action\":\"kick\"} that names
  nobody. That is a UI + model gap, upstream of any transport question.
";

/// The agent's tunables. All five were `: "${VAR:=default}"` in the bash.
#[derive(Debug, Clone)]
pub struct AgentEnv {
    /// Unit the agent controls. Interpolated into `Environment=` in the `@.service`.
    pub unit: String,
    /// Socket file name under `%t` (`$XDG_RUNTIME_DIR`).
    pub socket: String,
    /// Seconds to let a started unit prove it stays up before the state is read. Matches the
    /// `sleep 8` the deploy already uses for the same reason. The selftest drives this to 0.
    pub dwell_s: String,
    /// Absolute path the agent script lands on ON THE HOST. Referenced by `ExecStart=`, which
    /// systemd requires to be absolute, so it cannot be derived at render time from a relative
    /// path.
    pub remote_path: String,
    /// Install the agent as part of a real deploy. DEFAULT OFF, deliberately: the install step
    /// mutates a live host and T-289 could not exercise it. The RENDER is proven by
    /// `--agent-selftest`; the INSTALL is not, so it must be opted into by someone watching it.
    pub install: bool,
}

impl Default for AgentEnv {
    fn default() -> AgentEnv {
        AgentEnv::from_env()
    }
}

impl AgentEnv {
    pub fn from_env() -> AgentEnv {
        fn var_or(k: &str, d: &str) -> String {
            match std::env::var(k) {
                Ok(v) if !v.is_empty() => v,
                _ => d.to_string(),
            }
        }
        AgentEnv {
            unit: var_or("TBD_AGENT_UNIT", "tbd-reforger.service"),
            socket: var_or("TBD_AGENT_SOCKET", "tbd-reforger-agent.sock"),
            dwell_s: var_or("TBD_AGENT_DWELL_S", "8"),
            // Default matches the /home/sam/tbd/ prefix `cargo xtask deploy website` enforces.
            remote_path: var_or(
                "TBD_AGENT_REMOTE_PATH",
                "/home/sam/tbd/tbd-reforger-agent.sh",
            ),
            install: var_or("TBD_INSTALL_AGENT", "0") == "1",
        }
    }

    /// `validate_agent_names` — unit/socket names are interpolated into systemd unit files. Keep
    /// them to a charset that cannot carry a newline, a quote or a directive: fail closed rather
    /// than emit a unit file whose meaning depends on someone's env var.
    ///
    /// Note the two charsets DIFFER (`@` is legal in a unit name — it is systemd's template
    /// separator — and not in the socket file name). That is the bash's distinction, kept.
    pub fn validate_names(&self) -> Result<(), u8> {
        let unit_ok = !self.unit.is_empty()
            && self
                .unit
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '@' | '-'));
        if !unit_ok {
            eprintln!(
                "FAIL: TBD_AGENT_UNIT='{}' — only A-Za-z0-9._@- allowed.",
                self.unit
            );
            return Err(1);
        }
        let sock_ok = !self.socket.is_empty()
            && self
                .socket
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !sock_ok {
            eprintln!(
                "FAIL: TBD_AGENT_SOCKET='{}' — only A-Za-z0-9._- allowed.",
                self.socket
            );
            return Err(1);
        }
        Ok(())
    }

    /// `SocketMode=0600` in `%t` (`$XDG_RUNTIME_DIR`, mode 0700, owned by the run user) IS the
    /// credential: one uid can open it, and that uid is the API's. `Accept=yes` gives each
    /// connection its own short-lived instance, so a wedged request cannot block the next.
    pub fn socket_unit(&self) -> String {
        format!(
            "[Unit]\n\
             Description=TBD Reforger host control agent socket (T-289)\n\
             Documentation=man:systemd.socket(5)\n\
             \n\
             [Socket]\n\
             ListenStream=%t/{}\n\
             SocketMode=0600\n\
             Accept=yes\n\
             \n\
             [Install]\n\
             WantedBy=sockets.target\n",
            self.socket
        )
    }

    pub fn service_unit(&self) -> String {
        format!(
            "[Unit]\n\
             Description=TBD Reforger host control agent connection (T-289)\n\
             Documentation=man:systemd.socket(5)\n\
             \n\
             [Service]\n\
             Type=oneshot\n\
             ExecStart={}\n\
             Environment=TBD_AGENT_UNIT={}\n\
             Environment=TBD_AGENT_DWELL_S={}\n\
             StandardInput=socket\n\
             StandardOutput=socket\n\
             StandardError=journal\n",
            self.remote_path, self.unit, self.dwell_s
        )
    }
}

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

pub(super) fn write_or_die(path: &Path, body: &str) -> Result<(), u8> {
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
fn note(fail: &mut bool, v: Verdict) {
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
        tbd_gate::gate::require(
            "agent script missing its state re-read (read_state)",
            &Pattern::literal("read_state"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::require(
            "agent must gate on LoadState, not ActiveState alone",
            &Pattern::literal("LoadState"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::require(
            "socket must be 0600 — the file mode IS the credential",
            &Pattern::literal("SocketMode=0600"),
            &[&sock],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::require(
            "socket must live in %t ($XDG_RUNTIME_DIR)",
            &Pattern::literal("ListenStream=%t/"),
            &[&sock],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::require(
            "service must name the unit it controls",
            &Pattern::literal(&format!("Environment=TBD_AGENT_UNIT={}", env.unit)),
            &[&svc],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::require(
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
        tbd_gate::gate::require(
            "agent must accept exactly the four process verbs",
            &Pattern::literal("status|start|stop|restart) ACTION=\"$candidate\" ;;"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::ban(
            "agent must not grow a custom/passthrough case arm",
            &Pattern::literal("custom)"),
            &[&sh],
        ),
    );
    note(
        &mut fail,
        tbd_gate::gate::ban(
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
        tbd_gate::gate::ban(
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

#[cfg(test)]
mod tests {
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
}
