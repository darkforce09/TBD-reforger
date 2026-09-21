//! ── SCHEMA ──────────────────────────────────────────────────────────────────────────────────
//!
//! The wave gate's schema step. It runs the SET of contract sub-gates, not one of them.
//!
//! WHY NOT A SINGLE `cargo xtask schema validate`, WHICH IS THE OBVIOUS SHAPE AND IS VACUOUS:
//! `validate` is the golden-mission/registry suite and never opens `prefab-classify.json`, so a
//! contract change that adds an object kind with no class-enum mapping passes `validate` with
//! rc=0 while `schema map-object-enums` exits 1 on it. A one-liner step would print PASS over the
//! exact class of change this step exists to catch — a tool reporting success over an input it
//! never examined.
//!
//! The set is `cargo xtask ci schema-validate` plus `cargo xtask ci verify-citations`, i.e.
//! `cargo xtask ci ci-local-schema`.
//!
//! ── THE CROSS-CHECK'S SECOND SOURCE ─────────────────────────────────────────────────────────
//!
//! The tripwire below diffs two independent lists of the sub-gate names: [`VALIDATE_GATES`] here,
//! and [`crate::commands::ci::task_runner::TASKS`]' own `schema-validate` row read through
//! [`crate::commands::ci::task_runner::validate_gate_names`] — the same list
//! `cargo xtask schema list-gates` prints and the same list `run_task` executes. Adding a sub-gate
//! to that table without adding it here fails closed, which is the whole point.
//!
//! DELIBERATELY NOT CHANGE-SCOPED. "Only run if a .json under contracts_v2 changed" would examine
//! nothing on a diff of zero contract files, and it would be wrong on the facts anyway: these
//! gates read `tools_v2/xtask/src/verifications/schemas/checks.rs`, `contracts_v2/rules/`,
//! `apps/mod/tbd-framework/` and `docs/specs/**`. Nine sub-gates cost ~1.4 s warm.

use std::path::{Path, PathBuf};

use super::{Ctx, host};
use crate::wprintln;

/// Must equal `cargo xtask ci schema-validate`'s sub-gate SET, in the `TASKS` row's order.
/// `citations` comes from `verify-citations` / `ci-local-schema` and is layered on after the
/// tripwire. `height-labels` stays in this list even when a worktree skips running it.
const VALIDATE_GATES: &[&str] = &[
    "validate",
    "map-object-golden",
    "map-glyphs",
    "height-labels",
    "map-object-enums",
    "type-inventory",
    "specification-consistency",
    "n6",
    "n10",
];
const EXTRA_GATES: &[&str] = &["citations"];

/// DEM path `height-labels` (and `terrain-alignment`) decode. Probe is PNG magic, not byte size —
/// size alone would green a truncated file and red a future compressor win.
const DEM: &str = "assets_v2/terrains/everon/dem/everon-dem-16bit.png";

/// True iff THIS tree's Everon DEM is a real PNG (not a git-lfs pointer, not missing).
fn dem_materialized() -> bool {
    let Ok(body) = std::fs::read(DEM) else {
        return false;
    };
    body.len() >= 8 && body[..8] == [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
}

/// The sub-gate names `cargo xtask ci schema-validate` will actually run, in table order.
///
/// This is what `cargo xtask schema list-gates` prints. It is read from [`crate::commands::ci::task_runner::TASKS`] rather
/// than shelled out to, because the two live in the same binary: a subprocess could only ever be
/// the SAME list one build-freshness hazard later (the hazard `gate_schema`'s content stamp
/// further down exists to fight). Empty means the `schema-validate` row is gone, which the caller
/// treats as a refusal — never as "no gates to check".
pub fn task_validate_gates() -> Vec<String> {
    crate::commands::ci::task_runner::find("schema-validate")
        .map(crate::commands::ci::task_runner::validate_gate_names)
        .unwrap_or_default()
}

pub fn gate_schema(ctx: &Ctx) -> i32 {
    // DRIFT TRIPWIRE. A hardcoded list is readable and greppable but it rots silently: when
    // `schema-validate` grows a tenth sub-gate and nobody adds it here, the wave gate goes on
    // printing PASS over whatever that gate checks. Diff the SET against the executable task table
    // every run and refuse when they disagree — including PARTIAL reads. Refusing only an EMPTY
    // read is not enough: a truncated list still passes a one-way ⊆ check while the task runs
    // sub-gates this step never hears about.
    let mk_gates = task_validate_gates();
    if mk_gates.is_empty() {
        wprintln!(
            "schema: read 0 sub-gates out of the schema-validate task (tools_v2/xtask/src/commands/ci/task_definitions.rs)."
        );
        wprintln!(
            "        The drift check is the only thing keeping this step's list honest, so a step that"
        );
        wprintln!(
            "        could not run it must not go on to report PASS. Fix the read, or the task row."
        );
        return 1;
    }
    let mut mk_sorted = mk_gates.clone();
    mk_sorted.sort();
    let mut want_sorted: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
    want_sorted.sort();
    if mk_sorted != want_sorted {
        wprintln!("schema: schema-validate task set disagrees with GATE_SCHEMA_VALIDATE_GATES.");
        wprintln!("        list-gates: {}", mk_sorted.join(" "));
        wprintln!("        VALIDATE_GATES: {}", want_sorted.join(" "));
        wprintln!(
            "        A narrowed read or a tenth sub-gate would keep printing PASS over unchecked"
        );
        wprintln!("        contracts. Fail closed: sync the list, or fix the task row.");
        return 1;
    }

    // Runtime run-set: every VALIDATE gate, minus height-labels only when THIS tree's DEM is not a
    // materialized PNG, plus citations. Never a forever-exclusion list.
    let mut run_gates: Vec<String> = Vec::new();
    let mut skipped = String::new();
    for g in VALIDATE_GATES {
        if *g == "height-labels" && !dem_materialized() {
            skipped = "height-labels".into();
            wprintln!("schema: height-labels SKIP in this tree — {DEM} is not a materialized PNG");
            wprintln!(
                "        (LFS pointer or missing). On main with a real DEM this sub-gate RUNS; do not"
            );
            wprintln!(
                "        treat a worktree skip as 'red on main' or chase `xtask ci lfs-dem` for that."
            );
            continue;
        }
        run_gates.push((*g).to_string());
    }
    run_gates.extend(EXTRA_GATES.iter().map(|s| (*s).to_string()));
    if run_gates.is_empty() {
        wprintln!("schema: run-set is empty after per-context filtering — refusing vacuous PASS.");
        return 1;
    }

    // ---- make sure the xtask we are about to trust is THIS tree's ----
    //
    // A PRIVATE TARGET DIR. Cargo's freshness test is "is any source NEWER than the artifact?", so
    // sibling worktrees sharing `target/` clobber each other: a neighbour rebuilds `target/debug/
    // xtask` from ITS sources, this tree's older sources then look fresh against that newer
    // artifact, and `cargo run` executes the neighbour's binary with no rebuild and no warning.
    // The clobber is one-directional and therefore easy to miss.
    //
    // ONE dir, not one per tree (a per-tree dir grows without bound at ~1.7 GB each), plus a
    // CONTENT stamp: when this tree's xtask *and its path deps* hash differently from whatever last
    // built here, the dir is thrown away and rebuilt. Every crate xtask depends on BY PATH must be
    // a stamp root, or two trees can share this target dir under one stamp while a path dep
    // differs. Content, not mtime — mtime is the thing that lies.
    let stamp_roots = [
        "tools_v2/xtask/src",
        "apps/website/map-engine/src",
        "tools_v2/developer-tools/src",
    ];
    let mut srcs: Vec<PathBuf> = Vec::new();
    for r in stamp_roots {
        for e in walkdir::WalkDir::new(r).into_iter().flatten() {
            if e.file_type().is_file() && e.path().extension().map(|x| x == "rs").unwrap_or(false) {
                srcs.push(e.path().to_path_buf());
            }
        }
    }
    if srcs.is_empty() {
        wprintln!(
            "schema: found no stamp inputs under {} — cannot tell whose binary would run.",
            stamp_roots.join(" + ")
        );
        return 1;
    }
    // Byte order, so the concatenation is stable across locales.
    srcs.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    let mut blob: Vec<u8> = Vec::new();
    for s in &srcs {
        if let Ok(b) = std::fs::read(s) {
            blob.extend_from_slice(&b);
        }
    }
    for m in [
        "tools_v2/xtask/Cargo.toml",
        "apps/website/map-engine/Cargo.toml",
        "tools_v2/developer-tools/Cargo.toml",
        "Cargo.lock",
    ] {
        if let Ok(b) = std::fs::read(m) {
            blob.extend_from_slice(&b);
        }
    }
    let stamp = cksum(&blob);
    let stampfile = Path::new(&ctx.gate_schema_target).join(".tbd-xtask-src");
    if std::fs::read_to_string(&stampfile).unwrap_or_default() != stamp {
        let _ = std::fs::remove_dir_all(&ctx.gate_schema_target);
        if std::fs::create_dir_all(&ctx.gate_schema_target).is_err() {
            wprintln!("schema: cannot create {}", ctx.gate_schema_target);
            return 1;
        }
    }

    // Build once and separately, so a compile error reads as a compile error rather than as nine
    // identical schema failures. The step runner shows the tail, and a broken xtask fails all nine
    // otherwise.
    let build_argv = ctx.host.hostrun_argv(&host::v(&[
        "env",
        &format!("CARGO_TARGET_DIR={}", ctx.gate_schema_target),
        "cargo",
        "build",
        "-q",
        "-p",
        "xtask",
    ]));
    let (build_out, build_rc) = host::capture(&build_argv);
    if build_rc != 0 {
        let lines: Vec<&str> = build_out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(12)) {
            wprintln!("{l}");
        }
        wprintln!("schema: xtask failed to BUILD (rc {build_rc}) — no sub-gate was run.");
        if build_rc == 124 {
            return 124;
        }
        return 1;
    }
    // `printf '%s\n' "$stamp" > "$stampfile"` — written only after a successful build.
    let _ = std::fs::write(&stampfile, format!("{stamp}\n"));

    let want = run_gates.len();
    let mut ran = 0usize;
    let mut timedout = false;
    let mut failed = String::new();
    let mut detail = String::new();
    for g in &run_gates {
        let argv = ctx.host.hostrun_argv(&host::v(&[
            "env",
            &format!("CARGO_TARGET_DIR={}", ctx.gate_schema_target),
            "cargo",
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "schema",
            g,
        ]));
        let (out, rc) = host::capture(&argv);
        ran += 1;
        if rc == 0 {
            continue;
        }
        // 124 is hostrun's timeout, not a broken schema. Propagated below so run() can say so.
        if rc == 124 {
            timedout = true;
        }
        failed.push(' ');
        failed.push_str(g);
        detail.push_str(&format!("\n── schema {g} (rc {rc}) ──\n"));
        let lines: Vec<&str> = out.lines().collect();
        let tail: Vec<&str> = lines
            .iter()
            .skip(lines.len().saturating_sub(6))
            .copied()
            .collect();
        detail.push_str(&tail.join("\n"));
    }

    // NON-VACUITY. An empty run-set, or a loop that exits early, reaches the verdict below having
    // validated nothing — and would print PASS. That is the defect this function was added to fix,
    // one layer in. Count what actually executed and refuse to interpret a set that did not run.
    if ran == 0 || ran != want {
        wprintln!(
            "schema: executed {ran} of {want} sub-gate(s) — refusing to report on a set it did not run."
        );
        return 1;
    }

    // Summary LAST, on purpose: both step runners print `tail -15` of a failed step, so a verdict
    // printed first is the line that gets cut when several sub-gates fail at once.
    let run_list = run_gates.join(" ");
    if !failed.is_empty() {
        wprintln!("{detail}");
        if !skipped.is_empty() {
            wprintln!(
                "schema: FAILED{failed}  ({ran} sub-gates run; context-skipped: {skipped} — DEM not materialized here)"
            );
        } else {
            wprintln!("schema: FAILED{failed}  ({ran} sub-gates run)");
        }
        if timedout {
            return 124;
        }
        return 1;
    }
    if !skipped.is_empty() {
        wprintln!("schema: {ran} sub-gates OK ({run_list}; context-skipped: {skipped})");
    } else {
        wprintln!("schema: {ran} sub-gates OK ({run_list})");
    }
    0
}

/// POSIX `cksum` — CRC-32 (poly 0x04C11DB7, MSB-first) over the bytes then over the length,
/// complemented, rendered as `<crc><bytes>`.
///
/// Reimplemented rather than shelled out because the stamp file is SHARED with the bash gate during
/// the overlap: if the two disagreed about the stamp, each would throw away the other's
/// `target-gate-schema` and pay a 14 s cold rebuild every alternate run. `tr -d ' '` in the bash
/// joined the two fields, so the rendering is `crc` immediately followed by `length`.
fn cksum(data: &[u8]) -> String {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = (i as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 {
                (c << 1) ^ 0x04C1_1DB7
            } else {
                c << 1
            };
        }
        *slot = c;
    }
    let mut crc: u32 = 0;
    for b in data {
        crc = (crc << 8) ^ table[(((crc >> 24) as u8) ^ *b) as usize];
    }
    let mut n = data.len() as u64;
    while n != 0 {
        crc = (crc << 8) ^ table[(((crc >> 24) as u8) ^ (n as u8)) as usize];
        n >>= 8;
    }
    format!("{}{}", !crc, data.len())
}

#[cfg(test)]
#[path = "tests/schema/tests.rs"]
mod tests;
