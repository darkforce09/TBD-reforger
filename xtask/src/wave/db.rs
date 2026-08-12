//! The gate's integration database, and the one test step that refuses to call a skip a pass.

use super::{Ctx, host, ledger, lock::GateState};
use crate::{werr, wprint, wprintln};

/// `podman exec tbd_reforger_db psql …`, bridged when we are in the container.
///
/// Same host/container test as `hostrun`, and for the same reason: `command -v` alone is TRUE on
/// the host, where prefixing this with the bridge makes every psql call exit 126.
fn psql_argv(ctx: &Ctx, db: &str, flags: &[&str], sql: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if ctx.host.bridge {
        v.push("distrobox-host-exec".into());
    }
    v.extend(host::v(&[
        "podman",
        "exec",
        "tbd_reforger_db",
        "psql",
        "-U",
        "tbd",
        "-d",
        db,
    ]));
    v.extend(flags.iter().map(|s| (*s).to_string()));
    v.push(sql.to_string());
    v
}

/// Bring up a gate-private test database.
///
/// Its own DB, not the Makefile's `rust_it`: slice agents run `cargo xtask db test-it` concurrently, and that
/// target DROPs and recreates `rust_it`, so sharing it would make the gate race them.
///
/// T-411 / T-490: the IT database is per-wave (`tbd_gate_w<N>`), create-if-missing, with DBs older
/// than the last two waves dropped under the gate lock. NOT a per-run name (that leaks a DB every
/// kill) and NOT a timed wipe (that turns a permanent ratchet into an intermittent flake).
///
/// T-490: do NOT derive N from `current_wave` when a packing counter exists. `current_wave` is the
/// lowest plan wave with any deferred/open ticket — a Wave-3 deferral pins `tbd_gate_w3` forever
/// while the factory is packing Wave 35. Prefer `docs/platform/factory_pack_wave` (positive
/// integer, bumped on promote) so residue isolation tracks packing progress.
pub fn gate_wave_number(ctx: &Ctx) -> Option<String> {
    let mut w: Option<String> = None;
    if let Ok(v) = std::env::var("TBD_GATE_WAVE") {
        if !v.is_empty() {
            w = Some(v);
        }
    }
    if w.is_none() {
        let pack_file = ctx.root.join("docs/platform/factory_pack_wave");
        if pack_file.is_file() {
            // Single integer, optional trailing whitespace/newline. Reject empty, zero,
            // non-numeric.
            let pack: String = std::fs::read_to_string(&pack_file)
                .unwrap_or_default()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let ok = !pack.is_empty()
                && pack.bytes().all(|b| b.is_ascii_digit())
                && !pack.starts_with('0');
            if ok {
                w = Some(pack);
            }
        }
        if w.is_none() {
            let cw = ledger::current_wave(ctx);
            if cw == "done" {
                // All plan tickets shipped — pin to the highest wave number still in the plan.
                let mut nums: Vec<i64> = ledger::plan_rows(ctx)
                    .iter()
                    .filter_map(|r| r.split('\t').next())
                    .filter(|c| !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit()))
                    .filter_map(|c| c.parse().ok())
                    .collect();
                nums.sort_unstable();
                w = nums.last().map(|n| n.to_string());
            } else {
                w = Some(cw);
            }
        }
    }
    let val = w.unwrap_or_default();
    if val.is_empty() || !val.bytes().all(|b| b.is_ascii_digit()) {
        let shown = if val.is_empty() {
            "<empty>".to_string()
        } else {
            val.clone()
        };
        werr!("gate: cannot derive numeric wave for gate DB (got '{shown}')");
        return None;
    }
    Some(val)
}

/// Drop `tbd_gate_w*` databases older than the last two waves (keep N and N-1). Only names matching
/// `^tbd_gate_w[0-9]+$` — never `tbd_gate_it`, `tbd_gate_migrate`, or operator `TBD_GATE_DB` names.
///
/// T-534: the wave DB is no longer the only thing to reap. `cargo test -p website-api` now gives
/// each test BINARY its own database, derived as `<base>_<suite>_it` by
/// `apps/website/api/tests/common/mod.rs` (`per_binary_database_name`) — so one gate run against
/// `tbd_gate_w60` also leaves `tbd_gate_w60_admin_field_it`, `…_events_it`, … 25 of them, measured.
/// They are dropped and recreated on every run, so they do not grow per run — but without this they
/// would accumulate 25 per WAVE forever, because the old `^tbd_gate_w[0-9]+$` pattern matched none
/// of them. The wave number is now parsed out of the leading segment so a derived name is reaped
/// with the wave it belongs to, on exactly the same keep-N-and-N-1 policy.
///
/// The pattern is still anchored and still cannot name `tbd_gate_it`, `tbd_gate_migrate` or an
/// operator `TBD_GATE_DB`: it requires `tbd_gate_w<digits>` followed by end-of-name OR by a
/// `_<suite>_it` tail. Widening it further would put a DROP in reach of names this function was
/// never meant to touch — the header above is the contract, keep it narrow.
pub fn prune_old_gate_wave_dbs(ctx: &Ctx, wave: i64) {
    let keep_min = if wave > 0 { wave - 1 } else { 0 };
    // Listing needs -Atc (tuples-only); CREATE/DROP keep -qc.
    let list = psql_argv(
        ctx,
        "tbd_reforger",
        &["-Atc"],
        "SELECT datname FROM pg_database WHERE datname ~ '^tbd_gate_w[0-9]+(_[a-z0-9_]+_it)?$';",
    );
    let (out, _rc) = host::capture(&list);
    let re = regex::Regex::new(r"^tbd_gate_w([0-9]+)(_[a-z0-9_]+_it)?$").expect("static regex");
    for name in out.lines() {
        if name.is_empty() {
            continue;
        }
        // tbd_gate_w60 -> 60; tbd_gate_w60_admin_field_it -> 60. Anything else is skipped.
        let Some(c) = re.captures(name) else { continue };
        let n: i64 = c[1].parse().unwrap_or(0);
        if n < keep_min {
            wprintln!(
                "gate: dropping stale wave DB {name} (current wave {wave}; keeping w{keep_min}+)"
            );
            let drop = psql_argv(
                ctx,
                "tbd_reforger",
                &["-qc"],
                &format!("DROP DATABASE IF EXISTS {name} WITH (FORCE);"),
            );
            let _ = host::capture(&drop);
        }
    }
}

/// Prepare the gate database, and REFUSE the destructive prune without the lock.
///
/// T-575 — THE SECOND VARIABLE AND ITS DATABASE ARE GONE. This used to force-drop and recreate
/// `tbd_gate_migrate` and export `MIGRATE_TEST_DATABASE_URL` at it, because `tests/db_migrate.rs`
/// exercises the migration chain from empty and could not share a DB the other suites had already
/// migrated. T-558 moved `db_migrate.rs` AND `models_fromrow.rs` onto
/// `common::require_test_database_url`, so each gets its own `<base>_<suite>_it` off
/// `TEST_DATABASE_URL` (the T-534 shape) and NEITHER reads the variable any more.
///
/// Verified repo-wide before deleting, not assumed: the only surviving mentions of
/// `MIGRATE_TEST_DATABASE_URL` are the two `//!` doc comments in those same two test files
/// recording that they no longer share it, plus ticket registry prose. `std::env::var` for it: zero
/// hits. So the export named a variable nothing read, pointed at a database nothing opened, and the
/// `DROP … WITH (FORCE)` that preceded it could only ever have terminated sessions on a database
/// with no legitimate user. Deleted rather than left as harmless: a live-looking destructive
/// statement is exactly what a future reader will preserve on the assumption it matters.
///
/// THE DROP BELOW IS DESTRUCTIVE AND IS ONLY SAFE UNDER THE GATE LOCK — read before moving this
/// call, and before adding a fourth caller. It is now the per-wave IT DB prune alone
/// ([`prune_old_gate_wave_dbs`]), which is the same destructive class the migrate reset was and
/// keeps the same assert: `DROP DATABASE … WITH (FORCE)` terminates every session on the target,
/// and a `tbd_gate_w<N>` the prune considers stale can still be the DB ANOTHER GATE is testing
/// against.
///
/// It is closed by the flock, not by anything here — which means it was only ever as good as the
/// lock ACTUALLY being held, and before T-406 it was not: `take_gate_lock` returned 0 after failing
/// to lock, so on a full disk (252 MB free, recorded in `cmd_reclaim`'s header) this ran
/// unserialised. Assert the invariant rather than assume it.
///
/// `GATE_UNSERIALISED=1` is the deliberate escape hatch (`TBD_GATE_ALLOW_UNSERIALISED=1`): the
/// operator accepted a degraded verdict, and the full gate must still be able to prepare its
/// databases. T-409: the hatch used to return 0 from `take_gate_lock` without setting
/// `GATE_LOCK_HELD`, so this refused and every full-gate run under the hatch printed
/// `GATE: FAIL — UNSERIALISED` regardless of the code.
pub fn ensure_gate_db(ctx: &Ctx, state: &GateState) -> i32 {
    if std::env::var("TEST_DATABASE_URL")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return 0; // operator override wins
    }
    let mut wave: Option<String> = None;
    let pinned = std::env::var("TBD_GATE_DB").unwrap_or_default();
    if !pinned.is_empty() {
        // Operator-pinned full URL. Create-if-missing that database; do not prune wave DBs.
        let url = pinned.clone();
        let after_slash = url.rsplit('/').next().unwrap_or("").to_string();
        let db_name = after_slash.split('?').next().unwrap_or("").to_string();
        let safe = !db_name.is_empty()
            && db_name
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false)
            && db_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !safe {
            wprintln!(
                "gate: TBD_GATE_DB database name '{db_name}' is not a safe SQL identifier — refusing."
            );
            return 2;
        }
        // already-exists is fine
        let _ = host::capture(&psql_argv(
            ctx,
            "tbd_reforger",
            &["-qc"],
            &format!("CREATE DATABASE {db_name};"),
        ));
        unsafe { std::env::set_var("TEST_DATABASE_URL", &url) };
    } else {
        let Some(w) = gate_wave_number(ctx) else {
            return 2;
        };
        let db_name = format!("tbd_gate_w{w}");
        let url = format!("postgres://tbd:tbd@localhost:5434/{db_name}?sslmode=disable");
        let _ = host::capture(&psql_argv(
            ctx,
            "tbd_reforger",
            &["-qc"],
            &format!("CREATE DATABASE {db_name};"),
        ));
        unsafe { std::env::set_var("TEST_DATABASE_URL", &url) };
        wave = Some(w);
    }

    if !state.held() && !state.unserialised() {
        wprintln!(
            "gate: REFUSING to prune stale wave databases — the gate lock is NOT held, so a concurrent"
        );
        wprintln!(
            "        gate may be connected to one of them and WITH (FORCE) would kill its test run."
        );
        wprintln!("        ensure_gate_db must be called after take_gate_lock.");
        return 2;
    }
    if !state.held() && state.unserialised() {
        wprintln!(
            "gate: WARNING — pruning stale wave databases WITHOUT the lock (TBD_GATE_ALLOW_UNSERIALISED)."
        );
        wprintln!("        A concurrent gate may be connected to one; WITH (FORCE) would kill it.");
    }
    // Prune only on the default per-wave path — never when the operator pinned TBD_GATE_DB.
    if pinned.is_empty() {
        if let Some(w) = wave {
            prune_old_gate_wave_dbs(ctx, w.parse().unwrap_or(0));
        }
    }
    0
}

/// `cargo test -p website-api`, but a run where the DB tests skipped is a FAILURE, not a pass.
/// CARGO_TARGET_DIR IS PRIVATE HERE — read before removing it.
///
/// `cargo test` BUILDS AND THEN RUNS a binary. With the shared dir, the binary this step runs can be
/// one ANOTHER WORKTREE built: same package name and version across worktrees means the same
/// artifact hash, so they clobber. T-235 measured it three ways — its test binary ran another
/// worktree's 4-test build TWICE under a stable hash with changing contents, `target/debug/api`
/// changed size with its own source unchanged, and a compile failed against a stale rlib then
/// succeeded on retry with no edit.
///
/// Consequence, which is why this is a BLOCKER: THE GATE CAN PASS ON CODE IT NEVER COMPILED. T-233
/// reported 126 passed / 0 failed and its test fails on a clean database — a stale or foreign binary
/// that never contained the test produces exactly that, and it was reverted.
///
/// THIS PARAGRAPH USED TO END: "and `cargo check`/`clippy` do not need one because they emit no
/// binary to run." THAT WAS WRONG, it was the whole of T-421, and it is corrected here rather than
/// deleted because it is a reasonable-sounding inference that someone will otherwise make again.
/// The exposure is not about RUNNING anything. Cargo decides freshness by MTIME, so a check step
/// returns a verdict about a file it never opened whenever the file's mtime does not exceed the
/// recorded output's — no execution required.
pub fn gate_test_api(ctx: &Ctx) -> i32 {
    let argv = ctx.host.hostrun_argv(&host::v(&[
        "env",
        &format!(
            "CARGO_TARGET_DIR={}",
            ctx.main_root.join("target-gate-api").display()
        ),
        "CARGO_INCREMENTAL=0",
        "cargo",
        "test",
        "-p",
        "website-api",
        "--quiet",
        "--",
        "--nocapture",
    ]));
    let (out, rc) = host::capture(&argv);
    let skips = out.lines().filter(|l| l.starts_with("skip:")).count();
    wprint!("{out}");
    if !out.ends_with('\n') && !out.is_empty() {
        wprintln!();
    }
    if rc != 0 {
        return rc;
    }
    if skips > 0 {
        wprintln!("REFUSING to call this a pass: {skips} DB-backed test(s) SKIPPED.");
        wprintln!(
            "TEST_DATABASE_URL={} — is postgres up on :5434? (cargo xtask db up)",
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "<unset>".into())
        );
        return 1;
    }
    0
}
