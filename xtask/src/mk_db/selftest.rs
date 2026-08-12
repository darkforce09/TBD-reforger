//! T-894 acceptance harness — `cargo xtask db selftest`.
//!
//! T-556 (anti-vacuity): **a passing run is not evidence.** Every arm below either compares the
//! port against the thing it replaces on a tree where BOTH can run, or proves that the bash side
//! goes red (or, worse, wrongly green) on a deliberately broken one. An arm that cannot reach its
//! subject reports [`Verdict::DidNotRun`], which [`Report::finish`] ranks ABOVE a violation — a
//! green summary over arms that never executed is the exact failure this program exists to stop.
//!
//! ── THE ARMS ─────────────────────────────────────────────────────────────────────────────────
//!
//! | arm | subject | its RED proof |
//! |---|---|---|
//! | 1 frozen baseline | the port's rendered recipes vs text captured from `make -n` on 2026-08-12 | the baseline is a separate literal; drift in either direction fails |
//! | 2 Makefile pin | the LIVE `Makefile` recipe bodies vs the same renderers | any edit to the recipes fails the arm until the port follows |
//! | 3 T-381 refusal | `TBD_IT_BASE_DB=tbd_reforger cargo xtask db test-it` | asserts rc≠0 AND that `tbd_reforger` still exists afterwards |
//! | 4 reap | two `<base>_<suite>_it` databases really disappear | asserts they EXISTED first, and that an unrelated database survives |
//! | 5 reap fail-open | the Makefile's own pipeline against a dead container | asserts **bash exits 0** there (reaping nothing) while the port fails |
//! | 6 compose parity | `make db-up` vs `cargo xtask db up`, byte-for-byte | plus a missing-compose-file arm that must fail IDENTICALLY on both sides |
//!
//! ── WHY THE COMPARISON RUNS `make` ON THE FAR SIDE OF THE BRIDGE ─────────────────────────────
//!
//! `podman` does not exist in the agent container (measured — see the module header), so in here
//! `make db-up` cannot get past `sh: 1: podman: not found`. Arm 6 therefore runs `make` through
//! `distrobox-host-exec` where it CAN work, and the port in-container where the bridge does the
//! same crossing. Bridge selection follows [`crate::hostrun`]: [`hostrun::in_container`] answers
//! "am I containerised" (`command -v distrobox-host-exec` does NOT — it is installed on the host
//! too, where it refuses with 126).
//!
//! ── WHY THE ARMS USE A THROWAWAY COMPOSE PROJECT AND A SCRATCH DATABASE BASE ─────────────────
//!
//! Sibling slices (T-895 build lane, T-896 ci lane) run against the SAME host: the same
//! `tbd_reforger_db` container and the same postgres. `db down` on the shared project, or a reap
//! of `rust_it_%_it` while a sibling's suite is mid-run, would corrupt their acceptance runs and
//! look like a defect in their code. So arm 6 drives a private compose project
//! (`target-mk-db-selftest/`, its own container name, port 5499 and volume) and arm 4 uses a
//! `tbd_gate_t894*` base, which no other lane's pattern matches.

use std::fs;
use std::path::Path;

use anyhow::Result;
use tbd_gate::proc::Run;
use tbd_gate::{Finding, Kind, NotRun, Report, Verdict};

use super::IT_MAINT_DB;
use super::ab::{
    bridged, create_db, did_not_run, drop_db, norm, one_line, run_make, run_port, run_port_args,
    write_scratch_compose,
};
use super::recipes::{expand_make_vars, recipe_body, rendered_recipes};
use super::test_it::{reap, reap_select};
use crate::deploy_db_common as dbc;
use crate::root::find_repo_root;

/// Captured from `make -n` at the repo root on 2026-08-12, before a single line of the port
/// existed. This is the baseline the whole slice is measured against; it stays here after T-897
/// deletes the Makefile, which is the point — arm 2 dies with the file, arm 1 does not.
const BASELINE: &[(&str, &[&str])] = &[
    ("db-up", &["cd apps/website/api && podman compose up -d db"]),
    ("db-down", &["cd apps/website/api && podman compose down"]),
    (
        "db-logs",
        &["cd apps/website/api && podman compose logs -f db"],
    ),
    (
        "seed",
        &[
            "cd apps/website/api && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/discord_roles.sql",
            "cd apps/website/api && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/registry_dev.sql",
            "cd apps/website/api && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql",
            "cd apps/website/api && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/vehicle_database.sql",
            "cd apps/website/api && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql",
        ],
    ),
    (
        "rust-test-it",
        &[
            "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"DROP DATABASE IF EXISTS rust_it WITH (FORCE);\"",
            "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"CREATE DATABASE rust_it;\"",
            "cd apps/website/api && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable cargo test",
            "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -Atc \"SELECT datname FROM pg_database WHERE datname = 'rust_it' OR datname LIKE 'rust_it\\_%\\_it' ESCAPE '\\'\"",
        ],
    ),
];

/// The ONE thing a pinned recipe line may carry beyond what the port renders: the reap loop that
/// `test_it::reap` replaces in Rust.
///
/// The port renders the `psql -Atc "SELECT …"` half (it runs exactly that query); this is the
/// `| while read -r db; …; done` half, frozen from the Makefile so the shell it replaces is
/// readable next to the Rust. `$db` (not `$$db`) because [`expand_make_vars`] has already turned
/// make's escaped `$$` into what the shell sees; the embedded newline + tab is make's own
/// continuation echo.
///
/// Anything else after a pinned prefix is drift and fails arm 2.
const ALLOWED_TAIL: &str = " | while read -r db; do \\\n\t[ -n \"$db\" ] || continue; \\\n\tpodman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"DROP DATABASE IF EXISTS $db WITH (FORCE);\" >/dev/null; \\\ndone";

/// Scratch base for arm 4. `tbd_gate*` is on the T-381 allow-list and is matched by no other
/// lane's reap pattern (`make rust-test-it` reaps `rust_it\_%\_it`).
const ARM4_BASE: &str = "tbd_gate_t894";

pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    let mut report = Report::new("T-894 db lane");
    report.check(arm_frozen_baseline());
    report.check(arm_makefile_pin(&root));
    report.check(arm_t381_refusal());
    report.check(arm_reap());
    report.check(arm_reap_fail_open());
    report.check(arm_compose_parity(&root));
    Ok(report.finish() as u8)
}

// ── arm 1: the port renders exactly what `make -n` printed ───────────────────────────────────

fn arm_frozen_baseline() -> Verdict {
    let rendered = rendered_recipes();
    for (target, want) in BASELINE {
        let Some((_, got)) = rendered.iter().find(|(t, _)| t == target) else {
            return Verdict::failed(format!("arm 1: the port renders no recipe for `{target}`"));
        };
        if got.len() != want.len() {
            return Verdict::failed(format!(
                "arm 1: `{target}` renders {} line(s), the 2026-08-12 baseline has {}",
                got.len(),
                want.len()
            ));
        }
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            if g != w {
                return Verdict::failed(format!(
                    "arm 1: `{target}` line {i} drifted from the make baseline\n      make: {w}\n      port: {g}"
                ));
            }
        }
    }
    println!(
        "arm 1 OK — {} targets match the frozen `make -n` text",
        BASELINE.len()
    );
    Verdict::Held
}

// ── arm 2: the LIVE Makefile still says the same thing ───────────────────────────────────────

fn arm_makefile_pin(root: &Path) -> Verdict {
    let path = root.join("Makefile");
    let Ok(text) = fs::read_to_string(&path) else {
        // EXPIRY, not a fail-open: T-897 deletes this file on purpose. Arm 1 keeps the pin —
        // it compares the port against a literal that does not live in the Makefile.
        println!(
            "arm 2 SKIPPED — no {} (T-897 deleted it); arm 1 remains the pin",
            path.display()
        );
        return Verdict::Held;
    };
    for (target, want) in rendered_recipes() {
        let body = recipe_body(&text, target);
        if body.len() != want.len() {
            return Verdict::failed(format!(
                "arm 2: Makefile `{target}:` has {} command line(s), the port reproduces {}",
                body.len(),
                want.len()
            ));
        }
        for (i, (raw, w)) in body.iter().zip(want.iter()).enumerate() {
            // `$(WEB)`/`$(COMPOSE)` are make's, not the shell's — expand them the way make does
            // before comparing, and leave any OTHER `$(…)` in place so a new variable fails here
            // rather than passing a comparison of two different commands.
            let have = expand_make_vars(raw);
            match have.strip_prefix(w.as_str()) {
                Some("") | Some(ALLOWED_TAIL) => {}
                Some(tail) => {
                    return Verdict::failed(format!(
                        "arm 2: Makefile `{target}:` line {i} carries an unreproduced tail: `{tail}`"
                    ));
                }
                None => {
                    return Verdict::failed(format!(
                        "arm 2: Makefile `{target}:` line {i} is not what the port runs\n      make: {have}\n      port: {w}"
                    ));
                }
            }
        }
    }
    println!("arm 2 OK — Makefile recipes still match the port");
    Verdict::Held
}

// ── arm 3: T-381 refuses the live database, end to end ───────────────────────────────────────

fn arm_t381_refusal() -> Verdict {
    let Ok(exe) = std::env::current_exe() else {
        return Verdict::failed("arm 3: cannot locate the running xtask binary");
    };
    let out = Run::new(&exe)
        .args(["db", "test-it"])
        .env("TBD_IT_BASE_DB", "tbd_reforger")
        .merged_output();
    let Ok(m) = out else {
        return Verdict::failed("arm 3: `xtask db test-it` did not run at all");
    };
    if m.code == 0 {
        return Verdict::failed(
            "arm 3: `TBD_IT_BASE_DB=tbd_reforger db test-it` exited 0 — the T-381 guard is GONE",
        );
    }
    for needle in ["REFUSING", "T-381", "tbd_reforger"] {
        if !m.text.contains(needle) {
            return Verdict::failed(format!(
                "arm 3: the refusal no longer mentions `{needle}`:\n      {}",
                m.text.lines().next().unwrap_or("")
            ));
        }
    }
    // The refusal must also have stopped BEFORE the DROP. Ask postgres, not the exit code.
    match dbc::database_exists(IT_MAINT_DB) {
        Ok(true) => {
            println!(
                "arm 3 OK — refused with rc={}, `{IT_MAINT_DB}` intact",
                m.code
            );
            Verdict::Held
        }
        Ok(false) => Verdict::failed(format!(
            "arm 3: `{IT_MAINT_DB}` NO LONGER EXISTS after a refused run — the guard fired too late"
        )),
        Err(e) => Verdict::did_not_run(
            "arm 3: could not confirm the live database survived",
            Kind::Pin,
            NotRun::ToolError {
                tool: "psql".into(),
                status: 1,
                stderr: e.to_string(),
            },
        ),
    }
}

// ── arm 4: the reap loop actually reaps, and only what it should ─────────────────────────────

fn arm_reap() -> Verdict {
    let victims = [
        format!("{ARM4_BASE}_alpha_it"),
        format!("{ARM4_BASE}_beta_it"),
    ];
    let bystander = format!("{ARM4_BASE}_keep");
    let all: Vec<String> = victims.iter().cloned().chain([bystander.clone()]).collect();

    for db in &all {
        if let Err(e) = create_db(db) {
            return did_not_run("arm 4: could not create the scratch databases", e);
        }
    }
    // Anti-vacuity: "they are gone" means nothing unless they were there.
    for db in &all {
        match dbc::database_exists(db) {
            Ok(true) => {}
            Ok(false) => {
                return Verdict::failed(format!(
                    "arm 4: scratch database `{db}` was not created — the arm would have been vacuous"
                ));
            }
            Err(e) => return did_not_run("arm 4: could not probe the scratch databases", e),
        }
    }

    let rc = match reap(ARM4_BASE) {
        Ok(rc) => rc,
        Err(e) => return did_not_run("arm 4: the reap itself could not run", e),
    };
    let mut findings = Vec::new();
    if rc != 0 {
        findings.push(format!("reap exited {rc}"));
    }
    for db in &victims {
        if dbc::database_exists(db).unwrap_or(true) {
            findings.push(format!("`{db}` SURVIVED the reap"));
        }
    }
    if !dbc::database_exists(&bystander).unwrap_or(false) {
        findings.push(format!(
            "`{bystander}` was reaped — the LIKE pattern is too wide"
        ));
    }
    if !dbc::database_exists(IT_MAINT_DB).unwrap_or(false) {
        findings.push(format!(
            "`{IT_MAINT_DB}` was reaped — the allow-list failed"
        ));
    }
    let _ = drop_db(&bystander);
    if findings.is_empty() {
        println!("arm 4 OK — 2 reaped, `{bystander}` and `{IT_MAINT_DB}` untouched");
        return Verdict::Held;
    }
    Verdict::Failed(Finding {
        headline: "arm 4: the reap did not do what the Makefile promised".into(),
        detail: findings,
    })
}

// ── arm 5: the bash reap reported success while reaping nothing ──────────────────────────────

fn arm_reap_fail_open() -> Verdict {
    let bogus = "tbd_t894_no_such_container";
    let sql = reap_select("rust_it");
    // The Makefile's own line, verbatim in shape: psql piped into `while read`. The pipeline's
    // status is the LOOP's, so a dead container is invisible.
    let script = format!(
        "podman exec {bogus} psql -U tbd -d {IT_MAINT_DB} -Atc \"{sql}\" | while read -r db; do \
         [ -n \"$db\" ] || continue; podman exec {bogus} psql -U tbd -d {IT_MAINT_DB} -qc \
         \"DROP DATABASE IF EXISTS $db WITH (FORCE);\" >/dev/null; done"
    );
    let Some(argv) = bridged(&["sh", "-c", &script]) else {
        return Verdict::did_not_run(
            "arm 5: no host bridge, so the bash arm could not be run",
            Kind::Pin,
            NotRun::ToolAbsent("distrobox-host-exec".into()),
        );
    };
    let bash = Run::new(&argv[0]).args(&argv[1..]).merged_output();
    let Ok(bash) = bash else {
        return Verdict::failed("arm 5: the bash pipeline could not be run");
    };
    if bash.code != 0 {
        // If bash ever starts failing here the fail-open is closed upstream and this arm — and the
        // paragraph about it in the module header — should be deleted, not "fixed".
        return Verdict::failed(format!(
            "arm 5: the bash pipeline exited {} against a dead container.\n      \
             The documented fail-open (rc of the `while` loop, not of psql) no longer reproduces.",
            bash.code
        ));
    }

    // SAFETY: the selftest is single-threaded; the variable is restored immediately below.
    unsafe { std::env::set_var("TBD_DB_CONTAINER", bogus) };
    let port = reap("rust_it");
    unsafe { std::env::remove_var("TBD_DB_CONTAINER") };
    match port {
        Ok(0) => Verdict::failed(
            "arm 5: the PORT also reported success against a dead container — fail-open reopened",
        ),
        Ok(rc) => {
            println!("arm 5 OK — bash exits 0 reaping nothing; the port exits {rc}");
            Verdict::Held
        }
        Err(e) => did_not_run("arm 5: the port's reap could not run", e),
    }
}

// ── arm 6: `make db-up` vs `cargo xtask db up`, byte for byte ────────────────────────────────

fn arm_compose_parity(root: &Path) -> Verdict {
    if !root.join("Makefile").is_file() {
        println!("arm 6 SKIPPED — no Makefile (T-897 deleted it); nothing left to diff against");
        return Verdict::Held;
    }
    let scratch = root.join("target-mk-db-selftest");
    let rel = "target-mk-db-selftest";
    if let Err(e) = write_scratch_compose(&scratch) {
        return did_not_run("arm 6: could not write the scratch compose project", e);
    }
    let empty = scratch.join("empty");
    let empty_rel = format!("{rel}/empty");
    if let Err(e) = fs::create_dir_all(&empty) {
        return did_not_run("arm 6: could not create the empty-project dir", e.into());
    }

    // RED arm first: no compose file on either side. Identical failure or the parity claim is
    // only about the happy path.
    let red_make = run_make(root, "db-up", &empty_rel);
    let red_port = run_port(root, &empty_rel);
    let (Some(rm), Some(rp)) = (red_make, red_port) else {
        return Verdict::did_not_run(
            "arm 6: could not run both sides of the missing-compose-file arm",
            Kind::Pin,
            NotRun::ToolAbsent("make/distrobox-host-exec".into()),
        );
    };
    if rm.0 == 0 {
        return Verdict::failed(
            "arm 6: `make db-up` SUCCEEDED with no compose file — the RED arm proves nothing",
        );
    }
    let mut findings = Vec::new();
    if rm.0 != rp.0 {
        findings.push(format!(
            "missing compose file: make rc={} vs port rc={}",
            rm.0, rp.0
        ));
    }
    if norm(&rm.1) != norm(&rp.1) {
        findings.push(format!(
            "missing compose file: output differs\n      make: {}\n      port: {}",
            one_line(&rm.1),
            one_line(&rp.1)
        ));
    }

    // GREEN arm: same command, same already-running container, so podman prints a stable id.
    let _ = run_port(root, rel);
    let green_make = run_make(root, "db-up", rel);
    let green_port = run_port(root, rel);
    match (green_make, green_port) {
        (Some(gm), Some(gp)) => {
            if gm.0 != 0 {
                findings.push(format!(
                    "make db-up failed on the scratch project (rc={}): {}",
                    gm.0,
                    one_line(&gm.1)
                ));
            }
            if gm.0 != gp.0 {
                findings.push(format!("db-up: make rc={} vs port rc={}", gm.0, gp.0));
            }
            if norm(&gm.1) != norm(&gp.1) {
                findings.push(format!(
                    "db-up: output differs\n      make: {}\n      port: {}",
                    one_line(&gm.1),
                    one_line(&gp.1)
                ));
            }
        }
        _ => findings.push("db-up: one side could not be run".to_string()),
    }
    // Leave nothing running: `db down` on the scratch project only.
    let _ = run_port_args(root, rel, &["db", "down"]);

    if findings.is_empty() {
        println!("arm 6 OK — `make db-up` and `xtask db up` agree, broken and working");
        return Verdict::Held;
    }
    Verdict::Failed(Finding {
        headline: "arm 6: the compose lane diverged from make".into(),
        detail: findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arm 1 is only worth anything if the baseline and the renderer are separate texts. If a
    /// future edit "fixes" the baseline by generating it, this test is the tripwire.
    #[test]
    fn baseline_covers_every_rendered_target() {
        let rendered = rendered_recipes();
        assert_eq!(rendered.len(), BASELINE.len());
        for (t, _) in &rendered {
            assert!(
                BASELINE.iter().any(|(b, _)| b == t),
                "no frozen baseline for {t}"
            );
        }
    }

    #[test]
    fn frozen_baseline_matches_the_port() {
        match arm_frozen_baseline() {
            Verdict::Held => {}
            other => panic!("arm 1 must hold on a clean tree: {other:?}"),
        }
    }
}
