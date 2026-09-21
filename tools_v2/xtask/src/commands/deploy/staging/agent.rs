//! The host control agent.
//!
//! WHAT THIS IS. `POST /api/v1/admin/servers/:id/rcon` answers 503 `RCON_NO_TRANSPORT`
//! (the RCON console handler) because the API has no channel to the game host. This module renders
//! the host half of that channel; the API half lives in `apps/website/api_v2`.
//!
//! ── THE FACT THAT DECIDES THE DESIGN ─────────────────────────────────────────────────────────
//!
//! The game server shares a host with the API. `docs/mod/STAGING-SERVER.md` puts "API + Postgres
//! and Arma Reforger dedicated server" on one machine; `tools_v2/xtask/deploy/deploy.env.example`
//! gives ONE ssh host for both the website deploy and the staging deploy;
//! `docs/website/HOME_SERVER.md` makes the API `~/.config/systemd/user/tbd-website-api.service`;
//! this command restarts `tbd-reforger.service` through the same `systemctl --user`; and
//! `TBD_BACKEND_URL` defaults to `http://127.0.0.1:8080` — the mod reaches the API on LOOPBACK.
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
//!   (the RCON console handler) carries operator-supplied free text — that is remote code execution
//!   with an admin checkbox in front of it. It is also not possible on the box: `deploy.env` is
//!   gitignored AND rsync-excluded, so the credential exists only on a developer's PC.
//! * BattlEye / Reforger RCON over UDP. `ss -lntu` on the host binds only :8080 / :3000 / :5434
//!   (+ :5432) — 19999 is never bound; the renderer emits NO `rcon` key and `"battlEye": false`.
//!   DECISIVE: RCON only reaches a server that is ALREADY RUNNING. It structurally cannot do
//!   `start`, which is half of what this agent exists to deliver.
//! * A queued-command table the mod polls. Needs a migration plus mod-side polling that does not
//!   exist — and a dead server polls nothing, so again it cannot `start`.
//!
//! ── WHY THE AGENT RE-READS THE UNIT, WHICH IS THE ENTIRE POINT ───────────────────────────────
//!
//! `systemctl --user restart tbd-reforger.service` EXITS 0 OVER A SERVER THAT IS DEAD. Not
//! hypothetical on this host — `docs/mod/STAGING-SERVER.md` documents it: with `-a2sPort`
//! equal to `-bindPort` the engine logs "Unable to start replication" → "Game destroyed" and
//! exits status 0, so `Restart=on-failure` does NOT restart it. A restart followed by a bare
//! `sleep 8` therefore reports success over a server it never examined.
//!
//! So the agent NEVER derives its answer from the exit status of the verb. It runs the verb,
//! waits out the dwell, and RE-READS the unit's LoadState/ActiveState from systemd.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Result;
use verification_core::{Pattern, Verdict};

/// The agent script, rendered byte-for-byte onto the game host.
///
/// The script is byte-identical on every host and the per-server addressing lives in the systemd
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
# TBD Reforger host control agent — RENDERED by `cargo xtask deploy staging`.
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
# no representation here BY DESIGN; the renderer's module header carries the scope note.
set -uo pipefail

UNIT="${TBD_AGENT_UNIT:-}"
SYSTEMCTL="${TBD_AGENT_SYSTEMCTL:-systemctl}"
DWELL="${TBD_AGENT_DWELL_S:-8}"
ACTION="unknown"

# The only variable content in the output is $detail. Restrict it to a charset containing
# no JSON metacharacter, so this hand-rolled JSON cannot emit an invalid document — the
# failure this header warns about. Every other field is from a fixed set.
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

/// The agent's five tunables, each read from an environment variable with a default.
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
    /// mutates a live host no test may touch. The RENDER is proven by
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
    /// Note the two charsets DIFFER: `@` is legal in a unit name — it is systemd's template
    /// separator — and illegal in the socket file name.
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
             Description=TBD Reforger host control agent socket\n\
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
             Description=TBD Reforger host control agent connection\n\
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

#[cfg(test)]
#[path = "tests/agent/tests.rs"]
mod tests;

mod render_agent_files;
pub use render_agent_files::render_agent_files;
pub use render_agent_files::render_and_validate;
pub use render_agent_files::validate_agent_files;
pub(super) use render_agent_files::write_or_die;
