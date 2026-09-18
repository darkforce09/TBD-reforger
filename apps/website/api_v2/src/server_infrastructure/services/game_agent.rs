//! Client for the game host's control agent — the API half of the game-server channel.
//!
//! # What is on the other end
//!
//! A token-free, OS-guarded agent rendered by `cargo xtask deploy staging`. It is socket-activated
//! by systemd (`Accept=yes`, `StandardInput=socket`), so each connection gets its own short-lived
//! `bash` process whose entire contract is:
//!
//! ```text
//!   in : status | start | stop | restart          (one line)
//!   out: {"ok":<bool>,"action":"<verb>","result":"<r>","state":"<s>","detail":"<text>"}
//! ```
//!
//! `result` is `accepted` | `rejected` | `unreachable`; `state` is the unit's systemd
//! `ActiveState` **as re-read after the action**.
//!
//! # Why there is no credential here
//!
//! The socket lives at `%t/…` — `$XDG_RUNTIME_DIR`, mode `0700`, owned by the run user — and
//! the unit sets `SocketMode=0600`. The API and the game server are sibling
//! `systemctl --user` units under one uid on one box (`docs/mod/STAGING-SERVER.md:3`,
//! `docs/website/HOME_SERVER.md:282`, `TBD_BACKEND_URL=http://127.0.0.1:8080`), so **the
//! operating system is the credential**: exactly one uid can `connect(2)` that path. There is
//! no shared secret to store, rotate or leak, and nothing to add to `servers` for this
//! deployment. Across a same-uid socket there is no network hop, so there is nothing for an
//! endpoint-and-secret migration to secure. (A *second* game host reintroduces both — see the
//! migration sketch in the staging deploy command, §ADDRESSING.)
//!
//! # Why the timeout is 20s and not 5
//!
//! The agent deliberately **sleeps `TBD_AGENT_DWELL_S` (default 8) before answering
//! `start`/`restart`**, because a Reforger server that mis-starts exits 0 a few seconds in
//! (`docs/mod/STAGING-SERVER.md:246-250`) — a state read taken immediately after `start`
//! reports `active` for a server that is already dying. The dwell is what makes `accepted`
//! mean something.
//!
//! So a client timeout shorter than the dwell would turn **every honest slow answer into a
//! false `unreachable`**, and this module would ship the exact defect it exists to end: a tool
//! reporting a verdict about a thing it never waited to look at. [`AGENT_TIMEOUT`] is pinned
//! above the dwell by a unit test that reads the dwell out of the agent template itself — raise
//! the dwell there and this crate goes red.

use std::fmt;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Wall-clock budget for one connect → write → read round-trip.
///
/// **MUST exceed the agent's dwell** (`TBD_AGENT_DWELL_S`, default 8s). See the module docs;
/// the relationship is a test, not a comment.
pub const AGENT_TIMEOUT: Duration = Duration::from_secs(20);

/// One of the agent's four process verbs.
///
/// There is deliberately no `Custom`/free-text variant. The agent's entire safety argument is
/// that it filters the request to `[a-z]` and then matches a fixed four-element set, so no
/// operator-supplied text can reach a command. Widening this enum without widening the agent
/// would only manufacture requests it rejects; widening *both* is a different ticket
/// (the staging deploy command, §SCOPE GAP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAction {
    Status,
    Start,
    Stop,
    Restart,
}

impl AgentAction {
    /// The literal the agent matches on. A `&'static str` from a closed set, never request
    /// bytes — that is what keeps the wire free of operator input.
    pub fn verb(self) -> &'static str {
        match self {
            AgentAction::Status => "status",
            AgentAction::Start => "start",
            AgentAction::Stop => "stop",
            AgentAction::Restart => "restart",
        }
    }
}

impl fmt::Display for AgentAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.verb())
    }
}

/// The agent's verdict about what it observed **after** running the verb.
///
/// `#[serde(rename_all = "lowercase")]` matches the wire literals. There is intentionally no
/// `#[serde(other)]` catch-all: an unrecognised verdict must fail the parse and become a
/// transport error (→ 503), never a silent "accepted". A client that guessed at an unknown
/// verdict would be reporting success over an answer it did not understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentResult {
    /// The verb ran **and** the unit was re-read in the state the action intended.
    Accepted,
    /// The verb is unknown, or it ran and the unit did **not** get there. This is the
    /// `systemctl exits 0 over a dead server` case — the reason the agent exists.
    Rejected,
    /// systemd could not be reached, or the unit is not installed.
    Unreachable,
}

/// One line of agent answer, parsed.
///
/// Every field is required. A reply missing `state` — the field that carries the *observed*
/// truth — must not parse into a defaulted empty string, because a caller would then format an
/// audit row claiming an outcome it was never told. Missing field ⇒ parse error ⇒ 503.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentReply {
    /// The agent's own boolean. **Not the authority** — see [`AgentResult`]. Kept because it
    /// is on the wire and dropping it here would let the struct parse a reply whose `ok`
    /// disagreed with its `result` without anyone ever being able to notice.
    pub ok: bool,
    /// Echo of the verb the agent matched.
    pub action: String,
    /// The verdict. This is what callers must branch on.
    pub result: AgentResult,
    /// systemd `ActiveState` observed after the action: `active` | `inactive` | `failed` |
    /// `activating` | `deactivating` | `reloading` | `unknown`.
    pub state: String,
    /// Human-readable note, charset-restricted by the agent to characters that cannot break
    /// its hand-rolled JSON.
    pub detail: String,
}

/// Connect to `sock`, send one verb, read one line of JSON back.
///
/// `Err` means the **channel** failed — no listener, a wedged peer, a timeout, a truncated or
/// unparseable line. It never means "the server is unhealthy"; that is an `Ok(reply)` whose
/// `result` is [`AgentResult::Rejected`]. Callers must keep those apart, because "I could not
/// ask" and "I asked and the answer was no" send an operator to different places.
///
/// The write half is shut down after the verb so the agent's `read -r request` cannot block
/// waiting for input that is never coming — the same half-close the agent's own socket selftest
/// client performs.
pub async fn send(sock: &Path, action: AgentAction) -> anyhow::Result<AgentReply> {
    let line = tokio::time::timeout(AGENT_TIMEOUT, exchange(sock, action))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "game agent at {} did not answer '{action}' within {}s",
                sock.display(),
                AGENT_TIMEOUT.as_secs()
            )
        })??;

    let reply: AgentReply = serde_json::from_str(line.trim()).map_err(|e| {
        anyhow::anyhow!(
            "game agent at {} sent an unparseable reply to '{action}': {e} (raw: {:?})",
            sock.display(),
            line.trim()
        )
    })?;
    Ok(reply)
}

/// The untimed round-trip. Split out so [`send`] can wrap exactly this in the timeout.
async fn exchange(sock: &Path, action: AgentAction) -> anyhow::Result<String> {
    let stream = UnixStream::connect(sock)
        .await
        .map_err(|e| anyhow::anyhow!("cannot connect to game agent at {}: {e}", sock.display()))?;
    let (rd, mut wr) = stream.into_split();

    wr.write_all(action.verb().as_bytes()).await?;
    wr.write_all(b"\n").await?;
    wr.flush().await?;
    // Half-close: the agent reads exactly one line and we send exactly one, so signalling EOF
    // removes any chance of both sides waiting on the other.
    wr.shutdown().await?;

    let mut line = String::new();
    let read = BufReader::new(rd).read_line(&mut line).await?;
    if read == 0 {
        anyhow::bail!(
            "game agent at {} closed the connection without answering '{action}'",
            sock.display()
        );
    }
    Ok(line)
}

#[cfg(test)]
#[path = "tests/game_agent.rs"]
mod tests;
