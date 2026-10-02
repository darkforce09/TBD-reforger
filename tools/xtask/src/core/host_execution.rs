//! The container↔host bridge. This module IS the bridge; there is no shell shim beside it.
//!
//! It sits here, not in `playtest_server/host.rs`, so the playtest server and the wave driver
//! depend on ONE implementation. Before the lift there were two, and the second was about to be
//! written from the same bash a third time.
//!
//! ── WHY A BRIDGE IS STILL NEEDED — AND WHICH HALF OF THE SHIM IS OBSOLETE ────────────────────────
//!
//! READ THIS BEFORE CONCLUDING THE SHIM IS DEAD. The host bridge gives two
//! reasons. Exactly one of them has expired, and deleting the module on the strength of the expired
//! one would take the live half with it.
//!
//! Its header, verbatim, is the record:
//!
//! > Agent sessions run inside a `debian:12` podman container (`claude-desktop`): glibc 2.36 and
//! > NO C toolchain at all (no cc/gcc/ld). The real machine is Bazzite / Fedora 44: glibc 2.43
//! > with gcc. Consequences, both measured:
//! >
//! >   * `cargo build` in-container dies with ``linker `cc` not found``.
//! >   * Host-built binaries (target/debug/xtask, ArmaReforgerServer, Workbench) refuse to run
//! >     in-container with ``version `GLIBC_2.39' not found``.
//! >
//! > Both failures LOOK like "the repo is broken" and are not. A session that trusts the
//! > in-container error will "fix" a working toolchain — that happened once already and cost a
//! > 2.6 GiB `cargo clean`.
//!
//! **1. The TOOLCHAIN half is OBSOLETE.** `build-essential` is installed in the agent container now
//! and `cargo build` succeeds natively in-container. Nothing here routes cargo, rustc or a linker
//! through the bridge, and nothing should start: doing so would move a working native build onto a
//! slower path for a reason that stopped being true.
//!
//! **2. The host-BINARY half is STILL TRUE, and is the whole reason this module exists.** Steam, the
//! Workbench and `ArmaReforgerServer` are installed on the host and are linked against host glibc
//! (2.43). Nothing in the container can exec them — that is the ``GLIBC_2.39' not found`` line
//! above, and it is not fixable from this side at any price. Those processes are reachable ONLY
//! through `distrobox-host-exec`. So: anything that needs Steam, a game binary, or a host process
//! group goes through here, and NOTHING else does.
//!
//! ── `command -v` IS NOT A CONTAINER TEST — THE 126 TRAP ──────────────────────────────────────────
//!
//! `distrobox-host-exec` is installed on BOTH sides of the bridge: `/usr/bin/distrobox-host-exec`
//! exists in the container AND on the host. On the host it refuses. MEASURED 2026-07-26 on the host,
//! the wave driver records:
//!
//! ```text
//! $ distrobox-host-exec echo hi
//! You must run  distrobox-host-exec inside a container!      (exit 126)
//! ```
//!
//! A caller that cannot tell 126 from a real failure reports an ordinary step FAIL — OBSERVED 10/10
//! steps red, which reads as a catastrophically broken tree and sends whoever is holding the pager
//! hunting a phantom for an hour.
//!
//! So presence of the binary is NOT the question. [`Host::detect`] answers "am I containerised?"
//! with distrobox's own test (`/run/.containerenv` or `/.dockerenv`) and answers "is a bridge
//! available?" separately, and [`Host::argv`] prepends the bridge **only when both are true**. On the
//! host the bridge is not merely unavailable, it is UNNECESSARY — the binaries are native there,
//! which is the entire reason the bridge exists in the other direction. `bridge_is_never_used_on_the_metal`
//! pins that; it is the regression test for this trap, so do not delete it as redundant.
//!
//! ── THE `| head` GOTCHA, AND WHY THE PORT CANNOT HIT IT ──────────────────────────────────────────
//!
//! The bridge's own contract, verbatim:
//!
//! > GOTCHA (measured): under `set -euo pipefail`, `hostrun CMD | head -N` aborts the calling
//! > script. `head` closes the pipe after N lines, the bridge takes SIGPIPE and reports 127, and
//! > pipefail turns that into a fatal error even though CMD actually succeeded. Capture first:
//! >   out="$(hostrun CMD)"; echo "$out" | head -1
//! > `| tail`, `| cat`, and `| grep` are safe — they drain stdin.
//!
//! Here that hazard is not avoided by discipline, it is **unrepresentable**: [`Host::capture`]
//! returns a captured `String` and every `head -N` in the original became a `.lines().take(N)` over
//! that string. There is no pipe to close and no rc to misread. This is why the Rust shape differs
//! from the bash on purpose, and why the bash's "capture first, slice afterwards" rule has no
//! counterpart here — it is structural, not a convention someone has to remember.
//!
//! The one thing that DOES still matter is that the rc is read honestly, which is what
//! [`verification_core::proc`] is for: it surfaces a signalled or timed-out child as `NotRun` rather than
//! folding it into an exit code, so a bridge that died cannot be mistaken for a command that
//! answered.
//!
//! ── WHY WE SPAWN THE BRIDGE RATHER THAN REIMPLEMENT IT ───────────────────────────────────────────
//!
//! `distrobox-host-exec` is a host-side re-entry mechanism (it talks to `host-spawn`/flatpak-spawn
//! over the session bus). Reimplementing that is a large, fragile job with no upside; the shim's own
//! job was only ever "pick the right one and exec it", which is four lines. So this module spawns it.
//!
//! This module is the only bridge: callers go
//! through [`Host`], not a sourced shell shim. The wave driver used one until
//! Deleted the bash driver.
//!
//! cwd is preserved by `distrobox-host-exec`, so relative paths behave the same either way.

use std::process::{Command, Stdio};

use verification_core::proc::{self, Run};

/// The two bridges `_host_bridge()` will accept, in the order it tries them.
///
/// bash:
/// ```text
/// _host_bridge() {
///   if command -v distrobox-host-exec …; then echo "distrobox-host-exec"
///   elif command -v host-spawn …;        then echo "host-spawn"
///   fi
/// }
/// ```
const BRIDGES: &[&str] = &["distrobox-host-exec", "host-spawn"];

/// bash `in_container()` — true when this process is inside a container (podman/docker/distrobox).
///
/// ```text
/// in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ]; }
/// ```
///
/// This is distrobox's own test (`distrobox-host-exec:130`), copied rather than reinvented so the
/// two can never disagree about what "in a container" means.
///
/// NOTE for the wave driver: it carries a THIRD clause,
/// `|| [ -n "${container:-}" ]`, which the bridge does not. The two-clause form here
/// is the one both existing Rust callers were built and measured against, so the lift keeps it
/// exactly. Widening it is a behaviour change and belongs in its own ticket with its own measurement.
pub fn in_container() -> bool {
    std::path::Path::new("/run/.containerenv").exists()
        || std::path::Path::new("/.dockerenv").exists()
}

/// A resolved view of the host bridge.
#[derive(Debug, Clone)]
pub struct Host {
    /// `Some(program)` when a bridge is on `PATH`. `None` means "no bridge available", which only
    /// matters when [`Host::is_in_container`] is true — see the 126 trap in the module docs: this
    /// being `Some` says nothing at all about which side of the bridge we are on.
    bridge: Option<String>,
    in_container: bool,
    /// SELFTEST ONLY — models bash's `hostrun() { return 127; }` subshell override.
    ///
    /// S1 of `--selftest` exists because the defect is invisible on every passing run: the
    /// liveness probe only lied when the bridge flaked. There is no way to make a real bridge flake
    /// on demand, so the bash overrode the function; this flag is the same trick with a type.
    broken: bool,
}

impl Host {
    /// `in_container` + `_host_bridge`, evaluated once.
    pub fn detect() -> Host {
        Host {
            bridge: BRIDGES
                .iter()
                .find(|b| proc::which(b).is_ok())
                .map(|b| b.to_string()),
            in_container: in_container(),
            broken: false,
        }
    }

    /// Build a `Host` describing a specific situation, for tests and for callers that already know
    /// which side they are on. Everything else should use [`Host::detect`].
    // DEAD UNTIL THE SECOND CALLER LANDS. This item and the eight below it are the surface the
    // the wave driver consumes; the module sits here so the driver has one implementation to
    // call instead of a third copy of the bash. The playtest server does not need them, so the bin
    // target sees them as unused until `wave` lands — at which point every one of these
    // `allow(dead_code)`s should be deleted rather than kept "just in case".
    #[allow(dead_code)]
    pub fn new(bridge: Option<String>, in_container: bool) -> Host {
        Host {
            bridge,
            in_container,
            broken: false,
        }
    }

    /// A copy of this host whose every call fails with rc 127 and no output — S1's broken bridge.
    pub fn broken(&self) -> Host {
        Host {
            broken: true,
            ..self.clone()
        }
    }

    /// Are we containerised? The question [`Host::has_bridge`] does NOT answer.
    #[allow(dead_code)]
    pub fn is_in_container(&self) -> bool {
        self.in_container
    }

    /// Is a bridge binary on `PATH`? On its own this is NOT a container test — see the 126 trap.
    #[allow(dead_code)]
    pub fn has_bridge(&self) -> bool {
        self.bridge.is_some()
    }

    /// Is a bridge available (or unnecessary because we are on the metal)?
    fn usable(&self) -> bool {
        !self.broken && (!self.in_container || self.bridge.is_some())
    }

    /// bash `require_host` — assert the host is reachable before a long pipeline starts, so failures
    /// land early and with the right message. Prints the diagnostic and returns false on failure.
    ///
    /// ```text
    /// require_host() {
    ///   if ! in_container; then return 0; fi
    ///   if [ -z "$(_host_bridge)" ]; then
    ///     echo "require_host: no host bridge …" >&2
    ///     return 127
    ///   fi
    /// }
    /// ```
    pub fn require_host(&self) -> bool {
        if !self.in_container {
            return true;
        }
        if self.bridge.is_none() || self.broken {
            eprintln!("{REQUIRE_HOST_REFUSAL}");
            return false;
        }
        true
    }

    /// The bridge program named in operator-facing instructions.
    ///
    /// PRESERVED ODDITY: the bash hardcoded `distrobox-host-exec` in the STRAY SERVER block and in
    /// `assert_no_live_server`'s refusal even on a machine where `host-spawn` was the resolved
    /// bridge. Reproduced — those strings are copy-paste recovery commands that an operator has
    /// pasted before, and `BRIDGES[0]` is what they will have in their shell history.
    pub fn instruction_name(&self) -> &'static str {
        BRIDGES[0]
    }

    /// Build the real argv: `[bridge] cmd…` in a container, bare `cmd…` on the metal.
    ///
    /// The `if self.in_container` is the 126 trap's cure and is load-bearing. A bridge that exists
    /// on the metal is still not used there.
    fn argv(&self, cmd: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = Vec::with_capacity(cmd.len() + 1);
        if self.in_container
            && let Some(b) = &self.bridge
        {
            v.push(b.clone());
        }
        v.extend(cmd.iter().map(|s| s.to_string()));
        v
    }

    /// bash `hostrun "$@"` with both streams captured, and **silent** on refusal.
    ///
    /// Returns `None` when the bridge is unusable — the honest form of hostrun's `return 127`. bash
    /// also printed a heredoc diagnostic on that path, and it is deliberately NOT reproduced HERE:
    /// every `hostrun` call site in the playtest lane redirects stderr to `/dev/null`
    /// (`2>/dev/null` or `>/dev/null 2>&1`), so that text never reached a terminal and emitting it
    /// here would add output no baseline has. The rc and the empty capture are what those callers
    /// actually read.
    ///
    /// The loud form the bash header describes lives in [`Host::run`], which is what a caller that
    /// does NOT swallow stderr should use.
    pub fn capture(&self, cmd: &[&str]) -> Option<String> {
        if !self.usable() {
            return None;
        }
        let argv = self.argv(cmd);
        let (prog, args) = argv.split_first()?;
        // `merged_output` and not `output`: the two callers that read this text want it in the order
        // the far side produced it, and re-joining two separately-drained streams invents an
        // interleaving the child never emitted (see `verification_core::proc::Run::merged_output`).
        match Run::new(prog).args(args).merged_output() {
            Ok(m) => Some(m.text),
            // A bridge that could not be spawned, was signalled, or timed out is NOT an answer.
            // `None` propagates as "unknown" at the probe, which is the whole point.
            Err(_) => None,
        }
    }

    /// bash `hostrun "$@"` in its LOUD form: run direct on the metal, via the bridge in a container,
    /// and when containerised with NO bridge print the real diagnosis and return 127.
    ///
    /// This is the entry point for callers that show the operator stderr — the wave driver among
    /// them. The point of the refusal text is that the alternative is a linker or `GLIBC_2.39` error
    /// that LOOKS like a broken repo; see the module docs for what that cost the last time.
    ///
    /// Returns the child's raw exit code, or 127 for the refusal, or 127 if the child could not be
    /// run at all. Output is inherited, not captured — use [`Host::capture`] when you want the text.
    #[allow(dead_code)]
    pub fn run(&self, cmd: &[&str]) -> i32 {
        if !self.usable() {
            eprintln!("{}", self.refusal(cmd));
            return NO_BRIDGE_RC;
        }
        let argv = self.argv(cmd);
        let Some((prog, args)) = argv.split_first() else {
            eprintln!("{}", self.refusal(cmd));
            return NO_BRIDGE_RC;
        };
        match Run::new(prog).args(args).status() {
            Ok(code) => code,
            // Signalled / timed out / never spawned. `status()` reserves those for `NotRun` rather
            // than inventing an exit code, so we supply the same 127 the bash refusal used.
            Err(_) => NO_BRIDGE_RC,
        }
    }

    /// The heredoc `hostrun` prints when it is containerised with no bridge.
    ///
    /// Split from [`Host::run`] so the text is testable without a container.
    #[allow(dead_code)]
    pub fn refusal(&self, cmd: &[&str]) -> String {
        refusal_text(cmd, &container_glibc())
    }

    /// bash `hostrun … >/dev/null 2>&1 || true` — fire and forget, rc discarded.
    ///
    /// Used for the signals in `kill_run`, where bash explicitly `|| true`s the result: a `kill`
    /// that failed tells you nothing useful, because the ONLY trustworthy evidence of death is a
    /// subsequent probe. That fail-open is deliberate and is preserved.
    pub fn signal_quietly(&self, cmd: &[&str]) {
        if !self.usable() {
            return;
        }
        let argv = self.argv(cmd);
        if let Some((prog, args)) = argv.split_first() {
            let _ = Run::new(prog).args(args).output();
        }
    }

    /// bash `VAR="$(hostrun … 2>/dev/null | tr -d '[:space:]')"`.
    pub fn capture_trimmed(&self, cmd: &[&str]) -> String {
        let text = self.capture(cmd).unwrap_or_default();
        // `tr -d '[:space:]'` deletes every whitespace character anywhere, not just at the ends.
        text.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Spawn a bridge command in the BACKGROUND with both streams redirected to `sink`.
    ///
    /// bash: `hostrun env -C "$SERVER_DIR" setsid sh -c '…' _ "$RUN_DIR" >"$SRV_OUT" 2>&1 &`
    ///
    /// This is the one call that cannot go through [`Run`]: the launcher must stay alive after we
    /// return, its output must land in a FILE the polling loop reads, and we must not capture it
    /// into memory. Note what is NOT done here — no `setsid` on our side:
    ///
    /// * The SERVER is `setsid`-ed on the FAR side by the inner `setsid sh -c`, which is what makes
    ///   the pidfile a PROCESS GROUP LEADER and what the whole kill discipline rests on.
    /// * The local bridge proxy is deliberately left in OUR process group, exactly as bash's plain
    ///   `&` left it, so an operator's Ctrl-C reaches it too.
    pub fn spawn_background(
        &self,
        cmd: &[&str],
        sink: std::fs::File,
    ) -> std::io::Result<std::process::Child> {
        let argv = self.argv(cmd);
        let (prog, args) = argv
            .split_first()
            .ok_or_else(|| std::io::Error::other("no host bridge to spawn"))?;
        let err = sink.try_clone()?;
        Command::new(prog)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::from(err))
            .spawn()
    }
}

/// bash `hostrun`'s and `require_host`'s shared failure code.
///
/// 127 is "command not found" and is what the bash returned from both refusal paths.
#[allow(dead_code)]
pub const NO_BRIDGE_RC: i32 = 127;

/// bash `require_host`'s one-line refusal, verbatim.
const REQUIRE_HOST_REFUSAL: &str = "require_host: no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine.";

/// The `hostrun` refusal heredoc.
///
/// The bash, verbatim:
///
/// ```text
/// hostrun: running inside a container with no host bridge available.
///
///   Needed: distrobox-host-exec (or host-spawn) to reach the real machine.
///   This container has glibc $(ldd --version …) and no C toolchain,
///   so '$1' would fail with a misleading linker/GLIBC error rather than a useful one.
///
///   Run this command on the host instead:
///       $*
/// ```
///
/// ONE DELIBERATE DEVIATION, and it is the only edit to this text. The bash said "and no C
/// toolchain". That clause is measured FALSE today —
/// `build-essential` is installed in the agent container and `cargo build` works natively there.
/// Printing it now would tell the reader the exact thing this module exists to stop them believing,
/// in the one message they see at the moment they are most likely to act on it. So the clause is
/// replaced with the reason that is still true: the host binaries. Everything else — wording,
/// indentation, blank lines, the `'$1'` quoting, the six-space last line — is byte-faithful, and
/// `the_refusal_is_the_bash_heredoc` pins it.
#[allow(dead_code)]
fn refusal_text(cmd: &[&str], glibc: &str) -> String {
    // bash `$1`: the first word of the command. Empty argv cannot reach the heredoc in bash (there
    // would be nothing to run), but a Rust caller can pass `&[]`, so it degrades to an empty string
    // rather than panicking.
    let first = cmd.first().copied().unwrap_or("");
    // bash `$*`: every word, space-joined, unquoted.
    let all = cmd.join(" ");
    format!(
        "hostrun: running inside a container with no host bridge available.\n\
         \n\
         \x20 Needed: distrobox-host-exec (or host-spawn) to reach the real machine.\n\
         \x20 This container has glibc {glibc}, and Steam, the Workbench and ArmaReforgerServer are\n\
         \x20 installed on the host against a newer one,\n\
         \x20 so '{first}' would fail with a misleading linker/GLIBC error rather than a useful one.\n\
         \n\
         \x20 Run this command on the host instead:\n\
         \x20     {all}"
    )
}

/// bash `ldd --version 2>/dev/null | head -1 | grep -o '[0-9]\+\.[0-9]\+$' || echo '?'`.
///
/// Note the `| head -1` here is the gotcha the module docs describe, and note equally that it is
/// gone: this captures first and slices with `.lines().next()`, so there is no pipe for `head` to
/// close and no SIGPIPE to misread as failure.
#[allow(dead_code)]
fn container_glibc() -> String {
    let text = match Run::new("ldd").arg("--version").output() {
        Ok(o) => o.stdout,
        // bash discarded stderr and fell through to `|| echo '?'` — an `ldd` that is absent or
        // broken is not worth a second error message inside an error message.
        Err(_) => return "?".to_string(),
    };
    text.lines()
        .next()
        .and_then(trailing_version)
        .unwrap_or_else(|| "?".to_string())
}

/// `grep -o '[0-9]\+\.[0-9]\+$'` over one line.
///
/// Anchored at end of line, and leftmost-longest like grep: for `ldd (GNU libc) 2.43` this is
/// `2.43`, and for a line ending `2.36-9+deb12u14` there is no match at all (the trailing run is
/// `14`, which has no dot) — which is exactly why the bash reads `head -1`, since only the FIRST
/// `ldd --version` line ends in the bare version.
#[allow(dead_code)]
fn trailing_version(line: &str) -> Option<String> {
    let b = line.as_bytes();
    // Walk back over the trailing run of digits and dots — the only characters the pattern can match.
    let mut start = b.len();
    while start > 0 && (b[start - 1].is_ascii_digit() || b[start - 1] == b'.') {
        start -= 1;
    }
    let run = &line[start..];
    // The pattern needs `digits . digits` ending at `$`, so the minor part is everything after the
    // LAST dot and must be non-empty digits.
    let dot = run.rfind('.')?;
    let minor = &run[dot + 1..];
    if minor.is_empty() || !minor.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // The major part is the maximal digit run immediately before that dot — `grep` stops there
    // because `[0-9]\+` cannot cross a `.`.
    let head = &run[..dot];
    let major_start = head.len() - head.bytes().rev().take_while(u8::is_ascii_digit).count();
    let major = &head[major_start..];
    if major.is_empty() {
        return None;
    }
    Some(format!("{major}.{minor}"))
}

#[cfg(test)]
#[path = "tests/host_execution/tests.rs"]
mod tests;
