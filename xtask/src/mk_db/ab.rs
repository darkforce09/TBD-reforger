//! T-894 — the A/B plumbing: how each side of a comparison is run, and what may be normalised.
//!
//! Kept apart from `selftest`'s arms on purpose. The arms decide WHAT is compared; this file
//! decides what counts as "the same output", and that is where a parity harness quietly dies —
//! one over-eager `sed` in the normaliser and every arm passes forever. So the rules are few,
//! each one is justified against something measured, and each is unit-tested next to the code:
//!
//! * 64-hex container ids ([`norm`]) — podman mints a new one per creation, so two runs of a
//!   `create` cannot share one. Nothing else is erased: the compose provider banner, its ANSI
//!   escapes and every error string are compared literally.
//! * make's own `make: *** [target] Error N` line ([`make_error_rc`]) — not recipe output, and
//!   the only place the child's real rc survives, since make then exits 2 for every failure. It
//!   is PARSED into the rc the port is compared against rather than discarded.
//!
//! Nothing here normalises timing, paths or ordering. If a future arm needs that, it needs a
//! measurement in this header first.

use std::fs;
use std::path::Path;

use anyhow::Result;
use tbd_gate::proc::{self, Run};
use tbd_gate::{Kind, NotRun, Verdict};

use super::IT_MAINT_DB;
use crate::deploy_db_common as dbc;
use crate::hostrun;

/// Bridge-aware argv, following [`hostrun::Host::argv`]'s rule: prepend the bridge ONLY when
/// containerised. `None` means "containerised with no usable bridge" — never silently local.
pub(crate) fn bridged(cmd: &[&str]) -> Option<Vec<String>> {
    if !hostrun::in_container() {
        return Some(cmd.iter().map(|s| s.to_string()).collect());
    }
    let host = hostrun::Host::detect();
    if !host.require_host() {
        return None;
    }
    let bridge = host.instruction_name();
    proc::which(bridge).ok()?;
    let mut v = vec![bridge.to_string()];
    v.extend(cmd.iter().map(|s| s.to_string()));
    Some(v)
}

/// `(child rc, output without make's own decoration)` from `make <target> WEB=<rel>` at the repo
/// root, on the side of the bridge where `podman` exists.
///
/// Two things are taken off the make side before any diff, and NEITHER is swept under the rug —
/// both are the divergences documented in `mk_db`'s header:
///
/// * `make: *** [Makefile:70: db-up] Error 255` is make's own line, not the recipe's output. It is
///   also the ONLY place the child's real exit status appears, because make itself then exits 2
///   for every failure. So it is parsed, not merely dropped: the number in it becomes the rc the
///   port's rc is compared against, which makes this a STRONGER assertion than `rc == rc` would
///   have been.
/// * `make: [Makefile:205: rust-test-it] Error 127 (ignored)` — the `-`-prefixed variant. It does
///   not abort the target, so it never carries the outcome; dropped without becoming the rc.
pub(crate) fn run_make(root: &Path, target: &str, web_rel: &str) -> Option<(i32, String)> {
    let web = format!("WEB={web_rel}");
    let argv = bridged(&["make", target, &web])?;
    let m = Run::new(&argv[0])
        .args(&argv[1..])
        .cwd(root)
        .merged_output()
        .ok()?;
    let mut child_rc = m.code;
    let mut kept = Vec::new();
    for line in m.text.lines() {
        match make_error_rc(line) {
            Some(Some(rc)) => child_rc = rc,
            Some(None) => {}
            None => kept.push(line),
        }
    }
    let mut text = kept.join("\n");
    if m.text.ends_with('\n') && !text.is_empty() {
        text.push('\n');
    }
    Some((child_rc, text))
}

/// Classify one line of make's output.
///
/// `Some(Some(rc))` — a fatal `make: *** [target] Error rc`; `Some(None)` — the `(ignored)`
/// variant; `None` — not make's decoration at all, i.e. real recipe output.
pub(crate) fn make_error_rc(line: &str) -> Option<Option<i32>> {
    let rest = line.strip_prefix("make: ")?;
    let fatal = rest.starts_with("*** [");
    if !fatal && !rest.starts_with('[') {
        return None;
    }
    let n = rest.rsplit_once("] Error ")?.1;
    if n.ends_with("(ignored)") {
        return Some(None);
    }
    n.trim().parse().ok().map(Some)
}

pub(crate) fn run_port(root: &Path, web_rel: &str) -> Option<(i32, String)> {
    run_port_args(root, web_rel, &["db", "up"])
}

pub(crate) fn run_port_args(root: &Path, web_rel: &str, args: &[&str]) -> Option<(i32, String)> {
    let exe = std::env::current_exe().ok()?;
    Run::new(exe)
        .args(args)
        .cwd(root)
        .env("TBD_MK_WEB", web_rel)
        .merged_output()
        .ok()
        .map(|m| (m.code, m.text))
}

/// podman prints freshly-minted 64-hex ids; two runs of the same command cannot share them.
/// Everything else — including the compose provider banner and its ANSI codes — is compared raw.
pub(crate) fn norm(text: &str) -> String {
    text.split_whitespace()
        .map(|w| {
            if w.len() == 64 && w.chars().all(|c| c.is_ascii_hexdigit()) {
                "<ID>"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn one_line(text: &str) -> String {
    text.replace('\n', " ⏎ ")
}

pub(crate) fn create_db(db: &str) -> Result<()> {
    let (rc, _, err) = dbc::ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            dbc::db_user(),
            "-d".into(),
            IT_MAINT_DB.into(),
            "-qc".into(),
            format!("CREATE DATABASE {db};"),
        ],
    )?;
    if rc != 0 && !err.contains("already exists") {
        anyhow::bail!("CREATE DATABASE {db} exited {rc}: {err}");
    }
    Ok(())
}

pub(crate) fn drop_db(db: &str) -> Result<()> {
    dbc::ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            dbc::db_user(),
            "-d".into(),
            IT_MAINT_DB.into(),
            "-qc".into(),
            format!("DROP DATABASE IF EXISTS {db} WITH (FORCE);"),
        ],
    )?;
    Ok(())
}

pub(crate) fn did_not_run(msg: &str, e: anyhow::Error) -> Verdict {
    Verdict::did_not_run(
        msg,
        Kind::Pin,
        NotRun::ToolError {
            tool: "podman/psql".into(),
            status: 1,
            stderr: e.to_string(),
        },
    )
}

/// A private clone of `apps/website/api/docker-compose.yml`: different container name, port and
/// volume, so no arm here can stop or wipe the DB a sibling slice is using.
pub(crate) fn write_scratch_compose(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("docker-compose.yml"),
        "# T-894 selftest scratch project — created by `cargo xtask db selftest`, safe to delete.\n\
         services:\n  \
           db:\n    \
             image: docker.io/library/postgres:18-alpine\n    \
             container_name: tbd_t894_scratch_db\n    \
             environment:\n      \
               POSTGRES_USER: tbd\n      \
               POSTGRES_PASSWORD: tbd\n      \
               POSTGRES_DB: tbd_reforger\n    \
             ports:\n      \
               - \"5499:5432\"\n    \
             volumes:\n      \
               - tbd_t894_scratch_pgdata:/var/lib/postgresql\n\
         volumes:\n  \
           tbd_t894_scratch_pgdata:\n",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Container ids differ per creation; nothing else may be normalised away.
    #[test]
    fn norm_only_erases_ids() {
        let id = "a".repeat(64);
        assert_eq!(norm(&format!("started {id}")), "started <ID>");
        assert_eq!(norm("Error: no such file"), "Error: no such file");
    }

    /// make's decoration must be recognised exactly — and a recipe line that merely mentions an
    /// error must not be mistaken for it, or arm 6 would drop real output from the make side.
    #[test]
    fn make_error_lines_are_told_from_recipe_output() {
        assert_eq!(
            make_error_rc("make: *** [Makefile:70: db-up] Error 255"),
            Some(Some(255))
        );
        assert_eq!(
            make_error_rc("make: [Makefile:205: rust-test-it] Error 127 (ignored)"),
            Some(None)
        );
        assert_eq!(make_error_rc("Error: executing podman-compose: 255"), None);
        assert_eq!(make_error_rc("make: nothing to be done"), None);
    }
}
