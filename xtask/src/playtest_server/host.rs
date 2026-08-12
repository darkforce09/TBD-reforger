//! The container↔host bridge — the live half of `scripts/lib/hostrun.sh`.
//!
//! ── WHY A BRIDGE IS STILL NEEDED (and which half of the shim is obsolete) ─────────────────────
//!
//! `hostrun.sh` was written for T-181.0 and gave two reasons:
//!
//! 1. **No C toolchain in the container.** `cargo build` died with ``linker `cc` not found``.
//!    **This half is OBSOLETE** — build-essential is installed in the agent container now, and this
//!    module deliberately does NOT route cargo through the bridge.
//! 2. **Host-built binaries will not run in-container.** ``version `GLIBC_2.39' not found``.
//!    **This half is STILL TRUE and is why this module exists.** `ArmaReforgerServer` and Steam live
//!    on the host, at host glibc. Nothing in the container can exec them.
//!
//! Both failures LOOK like "the repo is broken" and are not. A session that trusts the in-container
//! error will "fix" a working toolchain — that happened once already and cost a 2.6 GiB
//! `cargo clean`. So: anything that needs Steam, a game binary, or a host process group goes
//! through here, and NOTHING else does.
//!
//! ── THE `| head` GOTCHA, AND WHY THE PORT CANNOT HIT IT ───────────────────────────────────────
//!
//! `hostrun.sh`'s header records: under `set -euo pipefail`, `hostrun CMD | head -N` aborts the
//! calling script. `head` closes the pipe after N lines, the bridge takes SIGPIPE and reports 127,
//! and `pipefail` turns that into a fatal error even though CMD actually succeeded. The bash
//! workaround was "capture first, slice afterwards".
//!
//! Here that hazard is not avoided by discipline, it is unrepresentable: [`Host::run`] returns a
//! captured `String` and every `head -N` in the original is a `.lines().take(N)` over that string.
//! There is no pipe to close and no rc to misread. The one thing that DOES still matter is that the
//! rc is read honestly, which is what [`tbd_gate::proc`] is for.
//!
//! ── WHY WE SPAWN THE BRIDGE RATHER THAN REIMPLEMENT IT ───────────────────────────────────────
//!
//! `distrobox-host-exec` is a host-side re-entry mechanism (it talks to `host-spawn`/flatpak-spawn
//! over the session bus). Reimplementing that is a large, fragile job with no upside; the shim's own
//! job was only ever "pick the right one and exec it", which is four lines. So this module spawns
//! it. `scripts/lib/hostrun.sh` is left ON DISK untouched — `scripts/platform/wave.sh` still sources
//! it, and T-853's own wave plan lists it as "dies last".

use std::process::{Command, Stdio};

use tbd_gate::proc::{self, Run};

/// The two bridges `_host_bridge()` will accept, in the order it tries them.
const BRIDGES: &[&str] = &["distrobox-host-exec", "host-spawn"];

/// A resolved view of the host bridge.
#[derive(Debug, Clone)]
pub struct Host {
    /// `Some(program)` when a bridge is on `PATH`. `None` means "no bridge available", which only
    /// matters when [`Host::in_container`] is true.
    bridge: Option<String>,
    in_container: bool,
    /// SELFTEST ONLY — models bash's `hostrun() { return 127; }` subshell override.
    ///
    /// S1 of `--selftest` exists because T-608's defect was invisible on every passing run: the
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
            // hostrun.sh: `[ -f /run/.containerenv ] || [ -f /.dockerenv ]`
            in_container: std::path::Path::new("/run/.containerenv").exists()
                || std::path::Path::new("/.dockerenv").exists(),
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

    /// Is a bridge available (or unnecessary because we are on the metal)?
    fn usable(&self) -> bool {
        !self.broken && (!self.in_container || self.bridge.is_some())
    }

    /// bash `require_host` — assert the host is reachable before a long pipeline starts, so failures
    /// land early and with the right message. Prints the diagnostic and returns false on failure.
    pub fn require_host(&self) -> bool {
        if !self.in_container {
            return true;
        }
        if self.bridge.is_none() || self.broken {
            eprintln!(
                "require_host: no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine."
            );
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
    fn argv(&self, cmd: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = Vec::with_capacity(cmd.len() + 1);
        if self.in_container {
            if let Some(b) = &self.bridge {
                v.push(b.clone());
            }
        }
        v.extend(cmd.iter().map(|s| s.to_string()));
        v
    }

    /// bash `hostrun "$@"` with both streams captured.
    ///
    /// Returns `None` when the bridge is unusable — the honest form of hostrun's `return 127`. bash
    /// also printed a heredoc diagnostic on that path, and it is deliberately NOT reproduced: every
    /// `hostrun` call site in this script redirects stderr to `/dev/null` (`2>/dev/null` or
    /// `>/dev/null 2>&1`), so that text never reached a terminal and emitting it here would add
    /// output no baseline has. The rc and the empty capture are what callers actually read.
    pub fn capture(&self, cmd: &[&str]) -> Option<String> {
        if !self.usable() {
            return None;
        }
        let argv = self.argv(cmd);
        let (prog, args) = argv.split_first()?;
        // `merged_output` and not `output`: the two callers that read this text want it in the order
        // the far side produced it, and re-joining two separately-drained streams invents an
        // interleaving the child never emitted (see `tbd_gate::proc::Run::merged_output`).
        match Run::new(prog).args(args).merged_output() {
            Ok(m) => Some(m.text),
            // A bridge that could not be spawned, was signalled, or timed out is NOT an answer.
            // `None` propagates as "unknown" at the probe, which is the whole point of T-608.
            Err(_) => None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn on_metal() -> Host {
        Host {
            bridge: None,
            in_container: false,
            broken: false,
        }
    }

    #[test]
    fn on_the_metal_commands_run_directly_and_need_no_bridge() {
        let h = on_metal();
        assert!(h.require_host(), "no bridge is needed outside a container");
        assert_eq!(h.argv(&["echo", "hi"]), vec!["echo", "hi"]);
        assert_eq!(h.capture(&["echo", "hi"]).unwrap(), "hi\n");
    }

    #[test]
    fn in_a_container_the_bridge_is_prepended() {
        let h = Host {
            bridge: Some("distrobox-host-exec".into()),
            in_container: true,
            broken: false,
        };
        assert_eq!(
            h.argv(&["kill", "-9", "--", "-42"]),
            vec!["distrobox-host-exec", "kill", "-9", "--", "-42"]
        );
    }

    #[test]
    fn a_containerised_host_with_no_bridge_is_rc_127_and_silent() {
        // hostrun's `return 127` path. The capture must be None — NOT an empty success, which is
        // what let a bridge failure read as "the process is dead" (T-608).
        let h = Host {
            bridge: None,
            in_container: true,
            broken: false,
        };
        assert!(h.capture(&["echo", "hi"]).is_none());
        assert_eq!(h.capture_trimmed(&["echo", "hi"]), "");
    }

    #[test]
    fn a_broken_bridge_answers_nothing_even_when_one_exists() {
        // S1's mechanism: the bridge is present and would work, and we still get no answer. This is
        // the ONLY way to reproduce T-608's trigger, because a real bridge cannot be made to flake.
        let metal = on_metal().broken();
        assert!(metal.capture(&["echo", "hi"]).is_none());
        // `require_host` short-circuits on the metal before any bridge question is asked — bash's
        // `if ! in_container; then return 0; fi` did the same, and S1 never calls it on the broken
        // host anyway (the override was scoped to the `kill_run` subshell).
        assert!(metal.require_host());
        // Inside a container, a broken bridge does fail the preflight.
        let boxed = Host {
            bridge: Some("distrobox-host-exec".into()),
            in_container: true,
            broken: true,
        };
        assert!(
            !boxed.require_host(),
            "a broken bridge must not pass preflight"
        );
        assert!(boxed.capture(&["echo", "hi"]).is_none());
    }

    #[test]
    fn capture_trimmed_deletes_all_whitespace_like_tr_d() {
        let h = on_metal();
        assert_eq!(
            h.capture_trimmed(&["printf", "  192.168.0.117 \n"]),
            "192.168.0.117"
        );
    }

    #[test]
    fn instruction_name_is_the_paste_ready_bridge_not_the_resolved_one() {
        // PRESERVED ODDITY — see `instruction_name`.
        let h = Host {
            bridge: Some("host-spawn".into()),
            in_container: true,
            broken: false,
        };
        assert_eq!(h.instruction_name(), "distrobox-host-exec");
    }
}
