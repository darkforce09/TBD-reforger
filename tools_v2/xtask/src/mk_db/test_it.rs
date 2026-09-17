//! T-894 — `make rust-test-it` as `cargo xtask db test-it` (Makefile:201-212).
//!
//! Split into its own file because this is the one genuinely non-trivial recipe left in the
//! Makefile: four command lines, two of which carry make prefixes that change their semantics
//! (`-` = ignore this line's status, `@` = do not echo), and a `while read -r db` loop over
//! `psql -Atc` output that drops databases. The compose lane next door is four one-liners.
//!
//! ── THE FOUR LINES, AND WHAT EACH ONE HIDES ──────────────────────────────────────────────────
//!
//! ```text
//! -podman exec tbd_reforger_db psql … -qc "DROP DATABASE IF EXISTS rust_it WITH (FORCE);"
//!  podman exec tbd_reforger_db psql … -qc "CREATE DATABASE rust_it;"
//!  cd apps/website/api && TEST_DATABASE_URL=…/rust_it?sslmode=disable cargo test
//! @podman exec … -Atc "SELECT … LIKE 'rust_it\_%\_it' ESCAPE '\'" | while read -r db; do …; done
//! ```
//!
//! 1. The leading `-` is why a first run works at all: `DROP … IF EXISTS` still exits non-zero
//!    when the CONTAINER is unreachable, and make is told to carry on regardless. Measured
//!    in-container, where podman is absent: `make: [Makefile:205: rust-test-it] Error 127
//!    (ignored)` — then line 2 fails the same way and aborts. Preserved: the port ignores the
//!    first line's status and honours the second's.
//! 2. `ESCAPE '\'` makes the underscores in `rust_it\_%\_it` LITERAL. Without it, `_` is SQL's
//!    single-character wildcard and the pattern would match names nobody meant to drop. This is
//!    the T-534 per-binary naming (`<base>_<suite>_it`, `apps/website/api/tests/common/mod.rs`)
//!    read back out.
//! 3. `[ -n "$db" ] || continue` guards the empty line `read` yields on a blank result set.
//! 4. `>/dev/null` is on the DROP's stdout only — psql's stderr stays on the terminal.
//!
//! ── WHAT THE PORT CHANGES, AND WHY EACH CHANGE IS NOT A PARITY BREAK ─────────────────────────
//!
//! - **The reap query's rc is read.** In bash the pipeline reports the `while` loop's status, so
//!   `psql` dying (container down, wrong name, bad credentials) produced a GREEN target that
//!   reaped nothing. `selftest`'s arm 5 runs the Makefile's own pipeline against a dead container
//!   and asserts it still exits 0 — that is the fail-open, measured, not asserted from reading.
//! - **Every name is re-checked against the T-381 allow-list before its DROP.** The Makefile is
//!   safe by construction (its base is the literal `rust_it`); this port accepts `TBD_IT_BASE_DB`
//!   so the selftest can use a scratch base that does not race sibling slices, and that knob is
//!   exactly the "stray env var" T-381 exists to stop. Guarded twice: once on the base, once per
//!   returned name.
//! - **The reap runs even when the suite fails.** make aborts the recipe on a red `cargo test`,
//!   skipping the T-558 prune on precisely the runs that leave leftovers. Output is unchanged
//!   (the reap is silent), so this costs nothing in parity — see [`join_rc`].

use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::{IT_BASE_DB, IT_MAINT_DB, echo, finish_status, runtime, web};
use crate::deploy_db_common as dbc;

/// The reap SELECT, byte-identical to Makefile:210 once `{base}` is substituted.
///
/// `LIKE 'rust_it\_%\_it' ESCAPE '\'` — the backslashes make the underscores LITERAL, so this is
/// "the base, an underscore, anything, then `_it`" (T-534's per-binary databases) and not the
/// single-character wildcard `_` would otherwise be.
pub(crate) fn reap_select(base: &str) -> String {
    format!(
        "SELECT datname FROM pg_database WHERE datname = '{base}' OR datname LIKE '{base}\\_%\\_it' ESCAPE '\\'"
    )
}

/// `TBD_IT_BASE_DB` or the Makefile's literal `rust_it`, refused unless the T-381 allow-list
/// accepts it. Returns the refusal text so callers can print it AND tests can assert on it.
pub(crate) fn guarded_base() -> Result<String, String> {
    let base = std::env::var("TBD_IT_BASE_DB")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| IT_BASE_DB.to_string());
    if dbc::is_safe_scratch_database_name(&base) {
        return Ok(base);
    }
    Err(format!(
        "\
───────────────────────────────────────────────────────────────────────
REFUSING to run the integration suite against database `{base}` (T-381 allow-list).

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe

  `cargo xtask db test-it` DROPs its base database and every
  `<base>_<suite>_it` sibling before and after the run. Against the live
  dev database `tbd_reforger` that is unrecoverable without a backup.

  This is the same allow-list the integration harness carries at
  apps/website/api/tests/common/mod.rs:87, which already stopped one
  exported TEST_DATABASE_URL from wiping the live database.

  Unset TBD_IT_BASE_DB to use the default scratch database `rust_it`.
───────────────────────────────────────────────────────────────────────"
    ))
}

/// A `<runtime> exec <container> psql -U <user> -d <maint> …` command, pre-bridged.
fn psql_cmd(sql_flag: &str, sql: &str) -> (Command, String) {
    let (rt, logical) = runtime();
    let container = dbc::db_container();
    let user = dbc::db_user();
    let echo_line =
        format!("{logical} exec {container} psql -U {user} -d {IT_MAINT_DB} {sql_flag} \"{sql}\"");
    let mut cmd = Command::new(&rt[0]);
    cmd.args(&rt[1..]).args([
        "exec",
        &container,
        "psql",
        "-U",
        &user,
        "-d",
        IT_MAINT_DB,
        sql_flag,
        sql,
    ]);
    (cmd, echo_line)
}

/// The whole target, in order.
pub(crate) fn run() -> Result<u8> {
    let base = match guarded_base() {
        Ok(b) => b,
        Err(msg) => {
            eprintln!("{msg}");
            return Ok(1);
        }
    };
    let web = web()?;

    // Line 1 — leading `-`: make ignores this one's status (the database usually does not exist).
    let (mut drop_cmd, drop_echo) = psql_cmd(
        "-qc",
        &format!("DROP DATABASE IF EXISTS {base} WITH (FORCE);"),
    );
    echo(&drop_echo);
    let _ = drop_cmd.status();

    // Line 2 — no `-`: a failure here aborts the target.
    let (mut create_cmd, create_echo) = psql_cmd("-qc", &format!("CREATE DATABASE {base};"));
    echo(&create_echo);
    let st = create_cmd.status().context("psql CREATE DATABASE")?;
    let rc = finish_status("psql -qc CREATE DATABASE", st);
    if rc != 0 {
        return Ok(rc);
    }

    // Line 3 — the suite itself.
    let url = format!("postgres://tbd:tbd@localhost:5434/{base}?sslmode=disable");
    echo(&format!(
        "cd {} && TEST_DATABASE_URL={url} cargo test",
        web.rel
    ));
    let st = Command::new("cargo")
        .arg("test")
        .current_dir(&web.abs)
        .env("TEST_DATABASE_URL", &url)
        .status()
        .context("failed to spawn cargo test")?;
    let test_rc = finish_status("cargo test", st);

    // Line 4 — `@`-silent reap. Runs even when the suite failed: that is precisely the run that
    // leaks per-binary databases, and the Makefile skipped it there.
    let reap_rc = reap(&base)?;
    Ok(join_rc(test_rc, reap_rc))
}

/// Which rc survives when both the suite and the reap have an opinion.
///
/// Split out as a pure function so the "a failing suite must not skip the reap" decision is
/// unit-testable: `reap_rc` is an *argument*, so it cannot have been short-circuited away.
pub(crate) fn join_rc(test_rc: u8, reap_rc: u8) -> u8 {
    if test_rc != 0 { test_rc } else { reap_rc }
}

/// T-558's prune: drop the base database and every `<base>_<suite>_it` sibling.
///
/// The bash was `psql -Atc … | while read -r db; do [ -n "$db" ] || continue; psql -qc "DROP …"
/// >/dev/null; done`. Two things change: the query's rc is checked (the pipeline's was invisible —
/// see the module header), and each name is re-checked against the T-381 allow-list before its
/// DROP. `read -r` strips leading/trailing IFS whitespace, which is what the `.trim()` matches.
pub(crate) fn reap(base: &str) -> Result<u8> {
    let (rc, stdout, stderr) = dbc::ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            dbc::db_user(),
            "-d".into(),
            IT_MAINT_DB.into(),
            "-Atc".into(),
            reap_select(base),
        ],
    )?;
    if rc != 0 {
        // The bash swallowed this: `psql | while read` reports the LOOP's status, so a dead
        // container greened the target while reaping nothing.
        eprint!("{stderr}");
        eprintln!(
            "FATAL: reap query failed (psql rc={rc}). Refusing to report a completed prune from a query that never ran."
        );
        return Ok(if rc > 0 { rc.clamp(1, 255) as u8 } else { 1 });
    }
    let (rt, _) = runtime();
    let container = dbc::db_container();
    let mut worst = 0u8;
    for line in stdout.lines() {
        let db = line.trim();
        if db.is_empty() {
            continue;
        }
        if !dbc::is_safe_scratch_database_name(db) {
            eprintln!(
                "REFUSING to drop `{db}` — outside the T-381 allow-list (rust_it, tbd_gate*, *_cold, *_it, *_probe)."
            );
            worst = 1;
            continue;
        }
        let st = Command::new(&rt[0])
            .args(&rt[1..])
            .args([
                "exec",
                &container,
                "psql",
                "-U",
                &dbc::db_user(),
                "-d",
                IT_MAINT_DB,
                "-qc",
                &format!("DROP DATABASE IF EXISTS {db} WITH (FORCE);"),
            ])
            // bash: `>/dev/null` on stdout only — psql's stderr stays visible.
            .stdout(Stdio::null())
            .status()
            .context("psql DROP DATABASE")?;
        let rc = finish_status("psql -qc DROP DATABASE", st);
        if rc != 0 {
            worst = rc;
        }
    }
    Ok(worst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact SQL from Makefile:210. If this string moves, T-534's per-binary databases stop
    /// being reaped and nobody notices until postgres runs out of them.
    #[test]
    fn reap_select_is_the_makefile_pattern() {
        assert_eq!(
            reap_select("rust_it"),
            "SELECT datname FROM pg_database WHERE datname = 'rust_it' OR datname LIKE 'rust_it\\_%\\_it' ESCAPE '\\'"
        );
    }

    /// T-381: the guard is the only thing between a stray env var and the dev database.
    #[test]
    fn t381_refuses_the_live_database() {
        // SAFETY: single-threaded assertion on this module's own knob.
        unsafe { std::env::set_var("TBD_IT_BASE_DB", "tbd_reforger") };
        let got = guarded_base();
        unsafe { std::env::remove_var("TBD_IT_BASE_DB") };
        let msg = got.expect_err("tbd_reforger must be refused");
        assert!(msg.contains("T-381"), "refusal must cite T-381: {msg}");
        assert!(msg.contains("tbd_reforger"));
        assert_eq!(
            guarded_base().unwrap(),
            "rust_it",
            "default must be rust_it"
        );
    }

    /// A red suite must still reap — the case the prune exists for.
    #[test]
    fn a_failing_suite_still_reports_its_own_rc() {
        assert_eq!(join_rc(101, 0), 101);
        assert_eq!(join_rc(0, 1), 1);
        assert_eq!(join_rc(0, 0), 0);
    }
}
