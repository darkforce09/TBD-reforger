//! Client for T-289's host control agent — the API half of the game-server channel.
//!
//! # What is on the other end
//!
//! T-289 shipped a token-free, OS-guarded agent rendered by `scripts/mod/deploy-staging.sh`
//! (`render_agent_files`). It is socket-activated by systemd (`Accept=yes`,
//! `StandardInput=socket`), so each connection gets its own short-lived `bash` process whose
//! entire contract is:
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
//! deployment. T-269 asked for an endpoint + secret migration because it assumed a network
//! hop; across a same-uid socket there is no hop. (A *second* game host reintroduces both —
//! see the migration sketch in `deploy-staging.sh` §ADDRESSING.)
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
//! above the dwell by [`tests::timeout_must_exceed_the_agents_dwell`], which reads the dwell
//! out of `deploy-staging.sh` itself — raise the dwell there and this crate goes red.

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
/// (`deploy-staging.sh` §SCOPE GAP).
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
/// waiting for input that is never coming — the same half-close T-289's own socket selftest
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
mod tests {
    use super::*;

    /// The renderer that owns the other end of this protocol. Pinning against the real file
    /// (not a copy) is what makes the dwell relationship below a live constraint.
    ///
    /// T-853 REPOINTED THIS. It was `scripts/mod/deploy-staging.sh`, which is now
    /// `cargo xtask deploy staging`; the agent template moved to
    /// `xtask/src/deploy_staging/agent.rs`. This is an `include_str!`, so it is a COMPILE-TIME
    /// dependency across a crate boundary — deleting the script broke `website-api`, which no
    /// `cargo test -p xtask` would ever have caught. The wave gate did, on `clippy api`.
    ///
    /// It is not a cargo dependency (website-api does not depend on xtask), which is the point:
    /// the pin exists precisely because the two sides are otherwise unconnected.
    const DEPLOY_STAGING_SH: &str =
        include_str!("../../../../../xtask/src/deploy_staging/agent.rs");

    #[test]
    fn verbs_are_the_agents_four_literals() {
        assert_eq!(AgentAction::Status.verb(), "status");
        assert_eq!(AgentAction::Start.verb(), "start");
        assert_eq!(AgentAction::Stop.verb(), "stop");
        assert_eq!(AgentAction::Restart.verb(), "restart");
        assert_eq!(AgentAction::Restart.to_string(), "restart");
        // And the agent must still accept exactly those four. If T-289's case arm is edited,
        // this client is speaking to a protocol that no longer exists.
        assert!(
            DEPLOY_STAGING_SH.contains(r#"status|start|stop|restart) ACTION="$candidate" ;;"#),
            "the rendered agent no longer matches exactly the four verbs this client sends"
        );
    }

    /// T-595 Class-R — **the timeout must exceed the dwell.**
    ///
    /// Read the dwell out of `deploy-staging.sh` rather than hardcoding 8 here, because the
    /// failure this guards against is somebody *raising the dwell* and leaving this client
    /// timing out under it. A hardcoded 8 would keep passing while every honest slow answer
    /// started reading as `unreachable` — a check that looked at the wrong thing.
    #[test]
    fn timeout_must_exceed_the_agents_dwell() {
        // T-853: the bash declared the default twice — `: "${TBD_AGENT_DWELL_S:=8}"` as the
        // script's own env default, and `DWELL="${TBD_AGENT_DWELL_S:-8}"` inside the agent it
        // rendered. This used to read the first. It now reads the second, which is the one that
        // actually governs how long the agent sleeps before answering. If those two ever
        // disagreed, the agent's own line is the one this client must not time out under.
        let marker = "TBD_AGENT_DWELL_S:-";
        let at = DEPLOY_STAGING_SH
            .find(marker)
            .expect("the rendered agent must still declare TBD_AGENT_DWELL_S");
        let rest = &DEPLOY_STAGING_SH[at + marker.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let dwell: u64 = digits
            .parse()
            .unwrap_or_else(|_| panic!("TBD_AGENT_DWELL_S default is not a number: {digits:?}"));

        assert!(
            dwell > 0,
            "a zero dwell would mean T-289 stopped waiting for the unit to prove it stays up"
        );
        assert!(
            AGENT_TIMEOUT.as_secs() > dwell,
            "AGENT_TIMEOUT is {}s but the agent dwells {dwell}s before answering start/restart. \
             A timeout at or under the dwell turns every honest slow answer into a false \
             `unreachable` — the client would be lying about a server that is fine.",
            AGENT_TIMEOUT.as_secs()
        );
    }

    /// The four literal lines T-289's `emit()` produces, byte-for-byte.
    #[test]
    fn parses_the_agents_exact_replies() {
        let accepted: AgentReply = serde_json::from_str(
            r#"{"ok":true,"action":"restart","result":"accepted","state":"active","detail":"unit active after restart"}"#,
        )
        .expect("accepted reply parses");
        assert!(accepted.ok);
        assert_eq!(accepted.result, AgentResult::Accepted);
        assert_eq!(accepted.state, "active");

        // The ticket's own case: systemctl exited 0, the unit is dead.
        let rejected: AgentReply = serde_json::from_str(
            r#"{"ok":false,"action":"restart","result":"rejected","state":"failed","detail":"unit is failed after restart; systemctl rc=0"}"#,
        )
        .expect("rejected reply parses");
        assert_eq!(rejected.result, AgentResult::Rejected);
        assert_eq!(rejected.state, "failed");
        assert!(rejected.detail.contains("rc=0"));

        let unreachable: AgentReply = serde_json::from_str(
            r#"{"ok":false,"action":"status","result":"unreachable","state":"unknown","detail":"unit not installed: not-found"}"#,
        )
        .expect("unreachable reply parses");
        assert_eq!(unreachable.result, AgentResult::Unreachable);
    }

    /// An answer this client does not understand must NOT become a verdict.
    #[test]
    fn unrecognised_or_truncated_replies_fail_closed() {
        // A verdict from a future agent we cannot interpret.
        assert!(
            serde_json::from_str::<AgentReply>(
                r#"{"ok":true,"action":"restart","result":"probably","state":"active","detail":"x"}"#
            )
            .is_err(),
            "an unknown `result` must fail the parse, never fall through to accepted"
        );
        // `state` missing — the field that carries the observed truth.
        assert!(
            serde_json::from_str::<AgentReply>(
                r#"{"ok":true,"action":"restart","result":"accepted","detail":"x"}"#
            )
            .is_err(),
            "a reply with no `state` must not parse into a defaulted empty string"
        );
        assert!(serde_json::from_str::<AgentReply>("not json at all").is_err());
    }

    /// No listener ⇒ `Err`, and quickly. This is the "socket absent / unit not installed"
    /// case the handler turns into a 503, and it must not spend the full timeout.
    #[tokio::test]
    async fn a_socket_with_no_listener_is_an_error_not_a_reply() {
        let path = std::env::temp_dir().join("t595-no-such-agent.sock");
        let _ = std::fs::remove_file(&path);
        let started = std::time::Instant::now();
        let err = send(&path, AgentAction::Status)
            .await
            .expect_err("connecting to a nonexistent socket must fail");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "ENOENT must surface immediately, not after the {}s timeout",
            AGENT_TIMEOUT.as_secs()
        );
        assert!(
            err.to_string().contains("cannot connect"),
            "the error must say the channel failed, got: {err}"
        );
    }
}
