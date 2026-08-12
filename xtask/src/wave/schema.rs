//! ── SCHEMA (T-420) ──────────────────────────────────────────────────────────────────────────
//!
//! Until this existed the gate validated NO schema at all. MEASURED on main at 33a7aa85:
//! `grep -c 'xtask schema' scripts/platform/wave.sh` -> 0, and `grep -n schema` -> zero hits in
//! 1249 lines. The eleven steps were cargo check / wasm32 / fmt / clippy x3 / test x3 / trunk /
//! ticket registry; not one read anything under `packages/tbd-schema`.
//!
//! Realised twice in one weekend:
//!   * wave 4 printed `GATE: PASS  11/11` on a wave whose HEADLINE deliverable was T-241's
//!     mission.schema.json change. The only evidence that schema was valid is that T-241's own
//!     agent ran the validator and said so. Agent reports are evidence, not testimony.
//!   * T-244 (wave 5) added a `vehicle` kind and would have merged with `make schema-validate` RED.
//!     Its slice gate passed for the worst possible reason: its diff is 0 `.rs` files, so fmt and
//!     clippy are change-scoped and examined nothing whatsoever.
//!
//! WHY THIS IS NOT ONE LINE OF `cargo xtask schema validate`, WHICH IS THE OBVIOUS FIX AND IS
//! VACUOUS. MEASURED 2026-07-26 against T-244's schema commit 25d551b6, from a detached probe
//! worktree:
//! ```text
//!     schema validate          rc=0   <- the obvious one-liner. GREEN.
//!     schema map-object-enums  rc=1   <- "prefab-classify rule[68]: kind 'vehicle' has no
//!                                        class-enum mapping" (x5)
//! ```
//! A `run "schema" hostrun cargo xtask schema validate` step would therefore have printed PASS over
//! the exact change that motivated this function: `validate` is the golden-mission/registry suite
//! and never opens prefab-classify.json. That is this program's signature defect — a tool reporting
//! success over an input it never examined — reproduced BY the fix for it. The step must run the
//! SET.
//!
//! The set is `make schema-validate` (Makefile:137) plus `make verify-citations` (Makefile:151),
//! i.e. `make ci-local-schema`. NOT ci.yml: its `schema` job (ci.yml:133,135) is `validate` +
//! `citations` only, so CI has the same hole and would not have caught T-244 either.
//!
//! DELIBERATELY NOT CHANGE-SCOPED. "Only run if a .json under packages/tbd-schema changed" is how
//! fmt and clippy came to examine nothing on T-244's diff, and it would be wrong on the facts
//! anyway: these gates read `xtask/src/schema_gates.rs`, `packages/tbd-schema/rules/`,
//! `apps/mod/tbd-framework/` and `docs/specs/**`. Nine sub-gates cost ~1.4 s warm.

use std::path::{Path, PathBuf};

use super::{Ctx, host};
use crate::wprintln;

/// `GATE_SCHEMA_VALIDATE_GATES` must equal `make schema-validate`'s sub-gate SET (order =
/// Makefile). `citations` comes from `make verify-citations` / ci-local-schema and is layered on
/// after the tripwire. `height-labels` stays in VALIDATE_GATES even when a worktree skips running
/// it.
const VALIDATE_GATES: &[&str] = &[
    "validate",
    "map-object-golden",
    "map-glyphs",
    "height-labels",
    "map-object-enums",
    "type-inventory",
    "t090-specs",
    "n6",
    "n10",
];
const EXTRA_GATES: &[&str] = &["citations"];

/// DEM path `height-labels` (and `terrain-alignment`) decode. Probe is PNG magic, not byte size —
/// size alone would green a truncated file and red a future compressor win.
const DEM: &str = "packages/map-assets/everon/dem/everon-dem-16bit.png";

/// True iff THIS tree's Everon DEM is a real PNG (not a git-lfs pointer, not missing).
fn dem_materialized() -> bool {
    let Ok(body) = std::fs::read(DEM) else {
        return false;
    };
    body.len() >= 8 && body[..8] == [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
}

/// Parse `make schema-validate` recipe names.
///
/// Survives blank lines, column-0 `#` comments, and backslash continuations — the three shapes that
/// made T-420's awk silently narrow (3-of-9 / 8-of-9) while GNU make still ran all nine. Ends the
/// recipe only on a real next target line.
pub fn makefile_validate_gates() -> Vec<String> {
    let Ok(body) = std::fs::read_to_string("Makefile") else {
        return Vec::new();
    };
    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    // `/^schema-validate:/ { i=1; next }`
    while i < lines.len() && !lines[i].starts_with("schema-validate:") {
        i += 1;
    }
    if i >= lines.len() {
        return out;
    }
    i += 1;
    let extract = regex::Regex::new(r"-p xtask -- schema ([a-z0-9-]*)").expect("static regex");
    while i < lines.len() {
        let l = lines[i];
        if l.trim().is_empty() {
            i += 1;
            continue;
        }
        if l.starts_with('#') {
            i += 1;
            continue;
        }
        if !l.starts_with('\t') {
            break; // a real next target line ends the recipe
        }
        let mut line = l.trim_start_matches('\t').to_string();
        // `while (line ~ /\\[[:space:]]*$/)` — join continuations, skipping comment/blank lines
        // WITHOUT consuming the trailing backslash again (awk's `continue` re-tests the condition).
        while ends_with_continuation(&line) {
            line = strip_continuation(&line);
            i += 1;
            if i >= lines.len() {
                break;
            }
            let nxt = lines[i].trim_start_matches('\t');
            if nxt.starts_with('#') || nxt.trim().is_empty() {
                // awk `continue`: the accumulated `line` no longer ends in a backslash here, so the
                // loop exits. Reproduced exactly — this is the shape that made the parse narrow.
                continue;
            }
            line.push_str(nxt);
        }
        if let Some(c) = extract.captures(&line) {
            out.push(c[1].to_string());
        }
        i += 1;
    }
    out
}

fn ends_with_continuation(s: &str) -> bool {
    let t = s.trim_end_matches([' ', '\t']);
    t.ends_with('\\')
}

fn strip_continuation(s: &str) -> String {
    let t = s.trim_end_matches([' ', '\t']);
    t.strip_suffix('\\').unwrap_or(t).to_string()
}

pub fn gate_schema(ctx: &Ctx) -> i32 {
    // DRIFT TRIPWIRE. A hardcoded list is readable and greppable but it rots silently, and the way
    // it rots is precisely this ticket: `make schema-validate` grows a tenth sub-gate, nobody adds
    // it here, and the wave gate goes on printing PASS over whatever that gate checks. Diff the SET
    // against the Makefile recipe every run and refuse when they disagree — including PARTIAL
    // parses. T-420 only refused an EMPTY parse; a blank/`#`/continuation mid-recipe narrowed the
    // awk output while make still ran all nine, and the one-way ⊆ check stayed green over the hole.
    let mk_gates = makefile_validate_gates();
    if mk_gates.is_empty() {
        wprintln!("schema: read 0 sub-gates out of the schema-validate recipe in Makefile.");
        wprintln!(
            "        The drift check is the only thing keeping this step's list honest, so a step that"
        );
        wprintln!(
            "        could not run it must not go on to report PASS. Fix the parse, or the recipe."
        );
        return 1;
    }
    let mut mk_sorted = mk_gates.clone();
    mk_sorted.sort();
    let mut want_sorted: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
    want_sorted.sort();
    if mk_sorted != want_sorted {
        wprintln!(
            "schema: Makefile schema-validate set disagrees with GATE_SCHEMA_VALIDATE_GATES."
        );
        wprintln!("        makefile: {}", mk_sorted.join(" "));
        wprintln!("        wave.sh:  {}", want_sorted.join(" "));
        wprintln!(
            "        A narrowed parse or a tenth sub-gate would keep printing PASS over unchecked"
        );
        wprintln!("        contracts. Fail closed: sync the list, or fix the recipe parse.");
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
                "        treat a worktree skip as 'red on main' or chase make lfs-dem for that."
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
    // A PRIVATE TARGET DIR, and it is not theoretical — it was MEASURED while this step was being
    // written, on this machine, with three sibling slices live:
    //   21:01:54  target/debug/xtask rebuilt by ANOTHER worktree (T-244, which owns
    //             xtask/schema_gates.rs this wave). `grep -ac vehicleClass target/debug/xtask` -> 2.
    //   21:0x     from THIS worktree, whose xtask sources contain zero `vehicleClass`:
    //               $ cargo build -p xtask        ->  Finished `dev` profile ... in 0.09s
    //               $ cargo run -q -p xtask -- schema map-object-golden
    //                 FAIL  S3 — prefabs-sample: no prefab example for kind 'vehicle'
    // MECHANISM: cargo's freshness test is "is any source NEWER than the artifact?". T-244's
    // schema_gates.rs is mtime 21:02:39; this tree's copy is 20:57:04, older than the 21:01:54
    // artifact — so cargo calls it fresh, never rebuilds, and `cargo run` executes the sibling's
    // binary. The clobber is one-directional and therefore easy to miss.
    //
    // ONE dir, not one per tree (a per-ticket dir grows without bound at ~1.7 GB each), plus a
    // CONTENT stamp: when this tree's xtask *and its path deps* hash differently from whatever last
    // built here, the dir is thrown away and rebuilt. T-420 stamped only xtask/src + xtask/Cargo.toml
    // + Cargo.lock while xtask depends on tbd-tools and map-engine-core BY PATH — two slice trees
    // could share GATE_SCHEMA_TARGET with the same stamp while map-engine-core differed (T-422
    // defect 3). Content, not mtime — mtime is the thing that lied.
    let stamp_roots = [
        "xtask/src",
        "crates/map-engine-core/src",
        "tools/tbd-tools/src",
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
            "schema: found no stamp inputs under xtask/ + map-engine-core/ + tbd-tools/ — cannot tell whose binary would run."
        );
        return 1;
    }
    // `LC_ALL=C sort` — byte order, so the concatenation is stable across locales.
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
        "xtask/Cargo.toml",
        "crates/map-engine-core/Cargo.toml",
        "tools/tbd-tools/Cargo.toml",
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
mod tests {
    use super::*;

    #[test]
    fn cksum_matches_the_coreutils_tool() {
        // If this drifts, the bash gate and this one fight over target-gate-schema and each pays a
        // cold rebuild. Compared against the real `cksum` so the interop claim is measured.
        use std::io::Write;
        let data = b"the quick brown fox\n";
        let mut child = std::process::Command::new("cksum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("cksum on PATH");
        child.stdin.take().unwrap().write_all(data).unwrap();
        let out = child.wait_with_output().unwrap();
        let want: String = String::from_utf8_lossy(&out.stdout)
            .trim_end()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert_eq!(cksum(data), want);
    }

    #[test]
    fn empty_input_matches_too() {
        assert_eq!(cksum(b""), format!("{}{}", 4294967295u32, 0));
    }

    #[test]
    fn the_makefile_recipe_parse_is_not_narrow() {
        // T-420's awk read 3 of 9 and the one-way subset check stayed green over the hole. The set
        // must match exactly, so an empty or partial parse is a hard fail in gate_schema.
        if Path::new("Makefile").is_file() {
            let mut got = makefile_validate_gates();
            got.sort();
            let mut want: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
            want.sort();
            assert_eq!(
                got, want,
                "the Makefile recipe parse disagrees with the pinned set"
            );
        }
    }
}
