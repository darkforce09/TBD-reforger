//! T-180.10 — the permanent Class-R coherency gate for ORBAT + Eden placement (T-853 port of
//! `scripts/verify-t180-coherency.sh`). Fail-fast, over map-engine-core, website-frontend and
//! map-engine-render.
//!
//! ── WHY THIS PORT CLOSES A LOOP ──────────────────────────────────────────────────────────────
//!
//! **T-216 fixed the four-outcome defect in THIS script first**, in wave 5, inline — and the fix
//! did not propagate. `scripts/mod/lib/gate-grep.sh` was extracted as a library precisely because
//! every `verify-t*.sh` written afterwards was born with the same two holes, and `crates/tbd-gate`
//! exists because a bash library can ask its callers to remember but cannot make them. So the gate
//! that first got it right is now built on the typed version of its own lesson: [`gate::ban`] /
//! [`gate::require`] hand back a [`Verdict`] with no `bool` conversion, and the [`translate`] match
//! is exhaustive — a cause added to [`NotRun`] breaks this file at compile time instead of quietly
//! becoming a pass. The three findings below are the script's own, carried over rather than
//! summarised: they record measurements nobody should have to re-take.
//!
//! ── 1. `grep -E`, NOT `rg`, AND THE STATUS IS READ (T-216) ───────────────────────────────────
//!
//! The three bans below used to read `if rg -n PAT FILE >/dev/null 2>&1; then fail; fi`. That form
//! reports OK for three different outcomes and can tell only one of them apart:
//!
//! ```text
//!   exit 0    match found            -> ban violated        -> correctly FAILED
//!   exit 1    no match               -> ban holds           -> correctly passed
//!   exit 2    TARGET FILE MISSING    -> check never ran     -> printed OK
//!   exit 127  SEARCH TOOL ABSENT     -> check never ran     -> printed OK
//! ```
//!
//! The last two are this program's signature defect — a tool reporting success over an input it
//! never examined — living inside the script written to catch it. MEASURED 2026-07-26: `rg` is
//! present in the dev container and ABSENT on the host, and cargo runs only on the host (glibc 2.36
//! vs 2.39, E0463), so every host run printed three OK lines for bans that had executed no
//! comparison at all; renaming `slots_gpu.rs` produced the same false green by the other route.
//! bash's cure was `grep -E` (present on both sides of that bridge) plus reading the raw status.
//! This port removes the search tool outright: [`Pattern`] is the `regex` crate compiled in with
//! `multi_line(true)`, so `^`/`$` stay LINE anchors exactly as in ERE while exit 127 stops being
//! reachable for the matcher at all. The patterns are byte-identical to the script's — `\(`, `\[`
//! and `|` mean the same in both engines — and every check names explicit files, so ripgrep's
//! recursion and gitignore defaults were never in play.
//!
//! ── 2. `--features "doc mission"`, NOT `doc` ALONE (T-216) ───────────────────────────────────
//!
//! `doc/store.rs`'s own tests call `crate::mission::compile::compile_payload` (store.rs:2589, 2601,
//! 2909, 2932 — the hydrate/compile round-trips T-344 added) and `mission` is a separate feature
//! gate at `lib.rs:23`, so `--features doc` cannot COMPILE the lib test target (`error[E0433]:
//! cannot find mission in crate` ×4). `set -euo pipefail` killed the script on the FIRST of the
//! seven doc-feature lines, so the tint/links lane, the derive gates, the compile boundary and the
//! whole website-frontend block had not run since T-344 — reproduced on main at `33a7aa85`. Adding
//! `mission` cannot weaken the gate: strictly more code compiled, selectors unchanged, one shared
//! test binary instead of a second feature set.
//!
//! ── 3. A SELECTOR THAT MATCHES NOTHING IS NOT A PASS (T-424) ─────────────────────────────────
//!
//! `cargo test --lib <selector>` exits 0 when the filter matches NOTHING. Measured 2026-07-27:
//! `cargo test -p map-engine-core --lib --features doc,mission -- zzz_no_such_test_exists_anywhere`
//! → `0 passed; 277 filtered out`, rc=0. Every selector here was once a bare `cargo test`, so a
//! typo or a rename printed `verify-t180 OK` having run zero assertions — the same defect as the
//! `if rg` bans. [`classify`] sums every `test result: … N passed` line and fails on 0, and
//! separately on no result line at all.
//!
//! ── OUTPUT IS A CONTRACT, AND SO IS THE EXIT CODE ────────────────────────────────────────────
//!
//! `Makefile:341` (`cargo xtask verify t180`) is the only executable caller, and T-853 accepts ports by
//! diffing stdout+stderr. So failures print bash's text verbatim — one `verify-t180 FAIL: …` line
//! on **stderr**, `ok` lines on stdout — not [`tbd_gate::Finding`]'s two-line render, and the exit
//! status is [`Verdict::into_exit_legacy_binary`]'s **1** for both failure kinds rather than the
//! four-outcome 2 that [`crate::gate_t439`] chose. Two deviations are deliberate and reachable only
//! when cargo itself does not run: see [`not_run_clause`].
//!
//! **The output is not reproducible run to run, and never was.** Two consecutive warm runs of the
//! *bash script* on 2026-08-12 differed on 9 of 803 lines, every one a wall-clock reading
//! (`Finished … in 0.07s` vs `0.06s`); cold runs add `Compiling <crate>` lines whose ORDER is the
//! build scheduler's, and `Running unittests` embeds `$CARGO_TARGET_DIR`. Nothing here can fix that
//! — it is cargo's own stdout passed through. Acceptance therefore diffs bash against Rust back to
//! back in one warm target dir with `in <float>s` normalised. The ordering that IS ours — checks,
//! `ok` lines, pin order — comes from the static tables below, never from a directory walk.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use regex::Regex;
use tbd_gate::{NotRun, Pattern, Verdict, gate};

// ── Targets, relative to the repo root ───────────────────────────────────────────────────────
// bash `cd "$ROOT"` first and passed these relative, so a "target file missing" line printed the
// relative path. Reproduced by joining onto `repo_root` to read and stripping back for the message,
// rather than mutating this process's cwd — tests run in parallel threads.
const EDITOR_OPS: &str = "apps/website/frontend/src/editor_ops.rs";
const ORBAT_RS: &str = "crates/map-engine-core/src/mission/orbat.rs";
const ORBAT_MGR: &str = "apps/website/frontend/src/orbat_manager.rs";
const EDEN_CHROME: &str = "apps/website/frontend/src/eden_chrome.rs";
const SLOTS_GPU: &str = "crates/map-engine-core/src/slots_gpu.rs";

/// One `ban`: message, ERE pattern, `-i`?, targets, and the `ok` line printed when it holds.
#[rustfmt::skip]
type BanRow = (&'static str, &'static str, bool, &'static [&'static str], &'static str);

#[rustfmt::skip]
const BANS: &[BanRow] = &[
    ("ensure_default_squad still present in editor_ops.rs",
     "ensure_default_squad", false, &[EDITOR_OPS],
     "no ensure_default_squad on place path"),
    ("orbat.rs still hardcodes loadout: String::new()",
     r"loadout: String::new\(\)", false, &[ORBAT_RS],
     "no loadout String::new() hardcode in derive"),
    ("Standardization / IFAK / Grenade Complement UI strings found (L8 omit)",
     "standardization|IFAK|Grenade Complement", true, &[ORBAT_MGR, EDEN_CHROME],
     "no Standardization UI strings"),
];

/// The three side-tint pins, all in `slots_gpu.rs`, all sharing one `ok` line. The RGBA triples are
/// the T-180 Class-R lock — BLUFOR/OPFOR/INDFOR must stay three visually distinct colours — and the
/// literal spacing is part of the pin, so reformatting the array is a change the gate should see.
#[rustfmt::skip]
const PINS: &[(&str, &str)] = &[
    ("SIDE_BLUFOR_RGBA pin missing", r"SIDE_BLUFOR_RGBA: \[u8; 4\] = \[173, 198, 255, 255\]"),
    ("SIDE_OPFOR_RGBA pin missing",  r"SIDE_OPFOR_RGBA: \[u8; 4\] = \[248, 113, 113, 255\]"),
    ("SIDE_INDFOR_RGBA pin missing", r"SIDE_INDFOR_RGBA: \[u8; 4\] = \[34, 197, 94, 255\]"),
];

const MEC: &str = "map-engine-core";
const MER: &str = "map-engine-render";
const FE: &str = "website-frontend";
/// One argv element, not two — and bash's `$*` re-joins it with a space, so the failure text reads
/// `--features doc mission`. Reproduced by [`shown`].
const DOCM: Option<&str> = Some("doc mission");
const MSN: Option<&str> = Some("mission");
const NOF: Option<&str> = None;

/// One `cargo_test_pin`: package, `--features` value, `--lib`?, selector, and the `ok` line to
/// print after it — `Some` only on the row that closes a section, exactly where bash's `ok` sat.
#[rustfmt::skip]
type PinRow = (&'static str, Option<&'static str>, bool, &'static str, Option<&'static str>);

#[rustfmt::skip]
const CARGO_PINS: &[PinRow] = &[
    // A / B / H — doc feature. `doc mission`, not `doc` alone; module docs §2.
    (MEC, DOCM, true, "place_", None),
    (MEC, DOCM, true, "set_leader_exclusive", None),
    (MEC, DOCM, true, "empty_squad_garbage_collected", None),
    (MEC, DOCM, true, "move_slot_bidirectional", None),
    (MEC, DOCM, true, "leader_invariant_holds", None),
    (MEC, DOCM, true, "attach_vehicle_roundtrip", None),
    (MEC, DOCM, true, "apply_faction_", Some("doc-feature place/mutator/apply gates")),
    // C / D / G / vehicle pack.
    (MEC, NOF, true, "side_tint_three_distinct", None),
    (MEC, NOF, true, "squad_link_", None),
    (MEC, NOF, true, "format_slot_line", None),
    (MEC, NOF, true, "pack_vehicle_instances", None),
    (MER, NOF, true, "mission_vehicles", Some("tint / links / slot_line / vehicles lane")),
    // I — mission feature derive / compile.
    (MEC, MSN, true, "derive_fills_loadout", None),
    (MEC, MSN, true, "derive_empty_loadout", None),
    (MEC, MSN, true, "derives_from_editor_sorted", None),
    (MEC, MSN, true, "compile_export_orbat_loadout", Some("derive/compile loadout gates")),
    // ── T-216 — THE COMPILE BOUNDARY. Read this before trimming the list above. ──────────────
    // Every selector up to here proves the editor can AUTHOR a T-180 value (doc::place_orbat,
    // doc::store), that the map can DRAW it (slots_gpu, map-engine-render), or that the ORBAT
    // derive keeps it (mission::orbat, mission::compile). Not one named a test in
    // `mission::flatten`, so the gate never crossed the edge where the document is handed to the
    // game server, and six values crossed nothing: a squad's leaderSlotId, a slot's tag / callsign
    // / rank / stance, and the whole vehicle roster. Measured 2026-07-26: a payload authoring all
    // six compiles to a document carrying none, with this gate printing ALL PASS. A gate is worth
    // nothing until you know what it looked at. These two are that missing edge — the ledger walks
    // each value from the saved payload to the serialized wire against mission.schema.json, so when
    // the contract widens (T-242) the newly-legal key's row turns red and the dead feature becomes
    // visible work; the second pins the compiled slot's key set, so nothing is added to or removed
    // from the website<->mod interface in silence.
    (MEC, MSN, true, "the_compile_boundary_ledger_is_checked_against_the_contract", None),
    (MEC, MSN, true, "a_compiled_slot_carries_exactly_these_keys", None),
    // T-482: the vehicle-floor test lives behind #[cfg(feature = "doc")] (the MissionDocCore writer
    // round-trip in flatten.rs), so mission-only matches 0 tests and this pin FAILs. Aligned with
    // the place_/attach_vehicle pins above rather than weakened.
    (MEC, DOCM, true, "the_vehicle_row_still_has_the_shape_this_module_reads",
        Some("compile-boundary ledger + compiled-slot key set + vehicle contract floor")),
    // E / F / G / H / I — FE. A bin crate, so no `--lib`: its tests live in src/main.rs.
    (FE, NOF, false, "eden_side", None),
    (FE, NOF, false, "apply_eden", None),
    (FE, NOF, false, "objects_chip", None),
    (FE, NOF, false, "open_arsenal", None),
    (FE, NOF, false, "g1_dialog", None),
    (FE, NOF, false, "orbat_", Some("website-frontend Eden/ORBAT gates")),
];

pub fn verify_t180(repo_root: &Path) -> Result<u8> {
    // bash's `fail()` exits 1 on the spot under `set -e`: the first failure ends the run, and both
    // halves below keep the script's order.
    if let Err(msg) = static_checks(repo_root) {
        return Ok(fail(&msg));
    }
    if let Err(msg) = cargo_checks(repo_root) {
        return Ok(fail(&msg));
    }
    println!("verify-t180: ALL PASS");
    Ok(0)
}

/// bash `ok()` — stdout.
fn ok(msg: &str) {
    println!("verify-t180 OK: {msg}");
}

/// bash `fail()` — stderr, then `exit 1`. Stdout is flushed first: it is a `LineWriter` so the
/// order already holds, but every caller merges the two streams with `2>&1` and the diff contract
/// should not rest on a buffering policy.
fn fail(msg: &str) -> u8 {
    let _ = std::io::stdout().flush();
    eprintln!("verify-t180 FAIL: {msg}");
    1 // Verdict::into_exit_legacy_binary()'s code, chosen deliberately — see the module docs.
}

// ── Static bans and pins ─────────────────────────────────────────────────────────────────────

fn static_checks(root: &Path) -> Result<(), String> {
    for (msg, pat, ci, rels, ok_line) in BANS {
        let files: Vec<_> = rels.iter().map(|r| root.join(r)).collect();
        let refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
        match pattern(pat, *ci) {
            Some(p) => translate(gate::ban(msg, &p, &refs), msg, root, Kind::Ban)?,
            // A pattern this file cannot compile is bash's `grep -E` rejecting it: exit 2.
            None => return Err(tool_status(msg, 2)),
        }
        ok(ok_line);
    }
    for (msg, pat) in PINS {
        let file = root.join(SLOTS_GPU);
        match pattern(pat, false) {
            Some(p) => translate(gate::require(msg, &p, &[&file]), msg, root, Kind::Pin)?,
            None => return Err(tool_status(msg, 2)),
        }
    }
    ok("RGBA side pins present");
    Ok(())
}

/// Which of bash's two "target file missing" sentences a check prints. `ban` and `require` worded
/// it differently, both wordings are scraped, so the difference survives the port.
enum Kind {
    Ban,
    Pin,
}

/// bash's `ban` continuation, spelled out on one line because a backslash-newline inside a
/// double-quoted bash string vanishes — the printed message never wrapped, and the diff knows it.
const BAN_MISSING: &str =
    "The ban could not run, and a moved or deleted file must not read as a clean result.";

/// `None` means the pattern would not compile — a bug in the tables above, and it must not read as
/// "the ban holds".
fn pattern(pat: &str, ci: bool) -> Option<Pattern> {
    let p = Pattern::regex(pat).ok()?;
    if ci {
        p.case_insensitive().ok()
    } else {
        Some(p)
    }
}

/// Render a [`Verdict`] as the line bash printed for the same input. Exhaustive on purpose:
/// [`Verdict`] has no `bool` conversion, so "did not run" cannot be folded into "passed" here by
/// accident, and a new [`NotRun`] variant in the library breaks this arm rather than going green.
fn translate(v: Verdict, msg: &str, root: &Path, kind: Kind) -> Result<(), String> {
    match v {
        Verdict::Held => Ok(()),
        // bash: `fail "$msg"` — the bare message, for a violated ban and an absent pin alike.
        Verdict::Failed(_) => Err(msg.to_string()),
        Verdict::DidNotRun(NotRun::TargetMissing(p), _) => {
            let path = p.strip_prefix(root).unwrap_or(&p).display();
            let why = match kind {
                Kind::Ban => BAN_MISSING,
                Kind::Pin => "The pin could not be checked.",
            };
            Err(format!("{msg} — target file missing: {path}. {why}"))
        }
        // All the library can otherwise report here is a file that exists and could not be read —
        // exactly the input on which `grep -E` errors and exits 2. `ToolAbsent` is unreachable now:
        // the matcher is compiled in, which is the T-620 class retired rather than asserted.
        Verdict::DidNotRun(..) => Err(tool_status(msg, 2)),
    }
}

fn tool_status(msg: &str, status: i32) -> String {
    format!(
        "{msg} — grep exited {status} (tool absent or bad pattern). Refusing to report OK on a \
         check that did not execute."
    )
}

// ── cargo_test_pin ───────────────────────────────────────────────────────────────────────────

fn cargo_checks(root: &Path) -> Result<(), String> {
    // bash: `export PATH="${HOME}/.cargo/bin:${PATH}"`. Kept because the cargo half runs only on
    // the host, where cargo is a rustup shim under $HOME rather than on a system PATH.
    let inherited = std::env::var("PATH").unwrap_or_default();
    let path = match std::env::var("HOME") {
        Ok(home) => format!("{home}/.cargo/bin:{inherited}"),
        Err(_) => inherited,
    };
    for (pkg, feats, lib, sel, ok_line) in CARGO_PINS {
        let mut args = vec!["test", "-p", pkg];
        if let Some(f) = feats {
            args.extend_from_slice(&["--features", f]);
        }
        if *lib {
            args.push("--lib");
        }
        args.extend_from_slice(&[sel, "--", "--quiet"]);
        cargo_test_pin(root, &path, &args)?;
        if let Some(m) = ok_line {
            ok(m);
        }
    }
    Ok(())
}

/// bash's `$*` inside `fail` — the pin's arguments joined by a space, quoting lost, so
/// `--features "doc mission"` renders as `--features doc mission`.
fn shown(args: &[&str]) -> String {
    args[1..].join(" ")
}

fn cargo_test_pin(root: &Path, path_env: &str, args: &[&str]) -> Result<(), String> {
    let label = shown(args);
    // `merged_output`, not `output`: cargo writes `Running unittests` to stderr while libtest
    // writes `running N tests` to stdout, and re-joining two separately-drained strings invents an
    // interleaving the child never produced. See the note on `Run::merged_output`.
    let run = tbd_gate::proc::Run::new("cargo")
        .args(args)
        .cwd(root)
        .env("PATH", path_env);
    match run.merged_output() {
        Ok(tbd_gate::proc::Merged { code, text, .. }) => {
            // bash: `printf '%s\n' "$out"`, where `$(…)` has already stripped EVERY trailing
            // newline. That is why libtest's blank line after `test result:` never appears between
            // two pins in the log — load-bearing for the diff, not cosmetic.
            println!("{}", text.trim_end_matches('\n'));
            classify(&label, code, &text)
        }
        Err(cause) => Err(format!("cargo test {label} — {}", not_run_clause(&cause))),
    }
}

/// The two deliberate deviations from bash, both unreachable while cargo runs at all.
///
/// bash captured `out="$(cargo test … 2>&1)" || status=$?` and reported every non-zero status the
/// same way: `cargo test … exited N`. That is wrong twice over. **A child killed by SIGKILL has no
/// exit code** — the shell synthesises 128+n and the `case` arm reads 137 as an ordinary numeric
/// failure, so under parallel worktrees "the OOM killer shot the gate" was reported as "the gate
/// found a problem"; and an absent cargo produced `exited 127`, the exact sentence the T-216 header
/// spends thirty lines explaining is a lie. Both are named here instead. The exit status is still
/// 1, so `cargo xtask verify t180` behaves identically; only the text differs, on inputs bash misdescribed.
/// Exhaustive: a new [`NotRun`] variant is a compile error, not a silent default.
fn not_run_clause(cause: &NotRun) -> String {
    let tail = "Refusing to report OK on a check that did not execute.";
    match cause {
        NotRun::TargetMissing(p) => format!("target missing: {}. {tail}", p.display()),
        NotRun::Unreadable { path, source } => {
            format!("unreadable: {} ({source}). {tail}", path.display())
        }
        NotRun::ToolAbsent(tool) => format!("`{tool}` is ABSENT. {tail}"),
        NotRun::ToolError { tool, status, .. } => format!("`{tool}` failed ({status}). {tail}"),
        NotRun::Signalled { tool, signal } => format!(
            "`{tool}` was killed by signal {signal} — the process died, it did not report. {tail}"
        ),
        NotRun::Timeout { tool, secs } => {
            format!("`{tool}` exceeded {secs}s and was killed. {tail}")
        }
    }
}

/// The three `cargo_test_pin` verdicts, as a pure function of what cargo returned.
///
/// Split from the spawn so the T-424 arms are testable without a two-minute build — the whole point
/// of the wrapper is the case where cargo exits **0**, and a test that had to compile
/// map-engine-core to reach it would not get written.
fn classify(label: &str, status: i32, out: &str) -> Result<(), String> {
    if status != 0 {
        return Err(format!("cargo test {label} exited {status}"));
    }
    // bash used sed+awk, not grep, "so pipefail cannot abort before we classify: no result line and
    // '0 passed' are different failures and both must be loud." They stay separate here.
    let counts = passed_counts(out);
    if counts.is_empty() {
        return Err(format!(
            "cargo test {label} — no 'test result: N passed' line. Refusing to report OK on a \
             check that did not execute."
        ));
    }
    if counts.iter().sum::<u64>() < 1 {
        return Err(format!(
            "cargo test {label} — 0 tests passed (selector matched nothing). A renamed/typo'd pin \
             must not silently empty."
        ));
    }
    Ok(())
}

/// Port of `sed -n 's/.*test result:.* \([0-9][0-9]*\) passed.*/\1/p'` — one entry per matching
/// LINE, because bash counted the lines with `wc -l` and summed them with `awk` and the two counts
/// answer different questions. `regex::Regex` rather than [`Pattern`]: this is a parse needing a
/// capture group, not a gate, and the per-line loop makes `multi_line` moot. The leading `.*` is
/// greedy in both engines, so a line carrying two `N passed` yields the LAST — reproduced, not
/// tidied away.
fn passed_counts(out: &str) -> Vec<u64> {
    let re = Regex::new(r"test result:.* ([0-9][0-9]*) passed").expect("literal pattern compiles");
    out.lines()
        .filter_map(|l| re.captures(l))
        .filter_map(|c| c[1].parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        here.parent().expect("xtask has a parent").to_path_buf()
    }

    /// A scratch repo root holding copies of the five files the static half reads.
    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tbd-t180-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for rel in [EDITOR_OPS, ORBAT_RS, ORBAT_MGR, EDEN_CHROME, SLOTS_GPU] {
            let dst = root.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(repo().join(rel), &dst).unwrap();
        }
        root
    }

    /// Run the static half over a perturbed tree and hand back the FAIL text bash would print.
    fn red(root: PathBuf) -> String {
        let got = static_checks(&root).expect_err("this perturbation must go red");
        let _ = std::fs::remove_dir_all(&root);
        got
    }

    /// Append to a target, so a banned pattern appears where it must not.
    fn red_append(name: &str, rel: &str, extra: &str, want: &str) {
        let root = scratch(name);
        let p = root.join(rel);
        std::fs::write(&p, std::fs::read_to_string(&p).unwrap() + extra).unwrap();
        assert_eq!(red(root), want);
    }

    /// Perturb `slots_gpu.rs` in place, so a pinned literal stops matching.
    fn red_sub(name: &str, from: &str, to: &str, want: &str) {
        let root = scratch(name);
        let p = root.join(SLOTS_GPU);
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.contains(from), "fixture text `{from}` is gone");
        std::fs::write(&p, body.replace(from, to)).unwrap();
        assert_eq!(red(root), want);
    }

    fn red_gone(name: &str, rel: &str) -> String {
        let root = scratch(name);
        std::fs::remove_file(root.join(rel)).unwrap();
        red(root)
    }

    #[test]
    fn the_real_tree_holds() {
        assert_eq!(static_checks(&repo()), Ok(()));
    }

    /// T-556 anti-vacuity: a gate that cannot fail checks nothing. One case per static arm,
    /// asserted against the tables so the right row is pinned as having fired; the exact bash
    /// wording — which is the diff contract — is asserted below.
    #[test]
    fn every_static_arm_can_go_red() {
        red_append("ban1", EDITOR_OPS, "\nensure_default_squad\n", BANS[0].0);
        red_append("ban2", ORBAT_RS, "\nloadout: String::new()\n", BANS[1].0);
        // `-i`, and in the SECOND target file: a ban over two files must read both of them.
        red_append("ban3", EDEN_CHROME, "\n// ifak pouch\n", BANS[2].0);
        red_append("ban3b", ORBAT_MGR, "\n// Grenade Complement\n", BANS[2].0);
        // One pin per side, perturbed three different ways: a value drift, a spacing drift (the
        // literal formatting is part of the lock) and a rename.
        red_sub("blufor", "173, 198, 255", "173, 198, 254", PINS[0].0);
        red_sub("opfor", "248, 113, 113,", "248,113,113,", PINS[1].0);
        red_sub("indfor", "SIDE_INDFOR_RGBA", "SIDE_INDEP_RGBA", PINS[2].0);
    }

    /// THE DEFECT THIS PORT INHERITS ITS EXISTENCE FROM. A target nobody read is not a clean
    /// target — and the two sentences differ because a ban and a pin send a reader elsewhere.
    #[test]
    fn a_missing_target_never_reads_as_a_pass() {
        let head = BANS[0].0;
        let want = format!("{head} — target file missing: {EDITOR_OPS}. {BAN_MISSING}");
        assert_eq!(red_gone("gone-ban", EDITOR_OPS), want);
        // The second file of the two-file ban, so the loop's ORDER is pinned as well.
        let second = red_gone("gone-ban2", EDEN_CHROME);
        let want = format!("missing: {EDEN_CHROME}");
        assert!(second.contains(&want), "{second}");
        let tail = "The pin could not be checked.";
        let want = format!("{} — target file missing: {SLOTS_GPU}. {tail}", PINS[0].0);
        assert_eq!(red_gone("gone-pin", SLOTS_GPU), want);
    }

    /// The three `cargo_test_pin` arms, including T-424's — cargo exits 0 and nothing ran — plus
    /// the two shapes that must still HOLD, so the classifier is not merely red on everything.
    #[test]
    fn every_cargo_pin_arm_can_go_red() {
        let label = "-p map-engine-core --lib zzz -- --quiet";
        let red = |status, out: &str| classify(label, status, out).unwrap_err();
        let empty = "\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; \
                     0 measured; 277 filtered out; finished in 0.00s\n\n";
        let want = format!(
            "cargo test {label} — 0 tests passed (selector matched nothing). A renamed/typo'd pin \
             must not silently empty."
        );
        assert_eq!(red(0, empty), want);
        // A compile error: cargo exits non-zero and never reaches libtest. Reported first, as bash
        // reported it — "it did not build" outranks "it printed no result line".
        let boom = "error[E0433]: cannot find `mission` in `crate`\n";
        assert_eq!(red(101, boom), format!("cargo test {label} exited 101"));
        // Exit 0 and no result line at all: cargo said nothing, and silence is not a pass.
        let want = format!(
            "cargo test {label} — no 'test result: N passed' line. Refusing to report OK on a \
             check that did not execute."
        );
        assert_eq!(red(0, "Finished `test` profile\n"), want);
        let one = "test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 513 filtered out\n";
        assert_eq!(classify(label, 0, one), Ok(()));
        // Two result lines with `0 passed` first: `wc -l` sees 2 and `awk` sums to 17, so it HOLDS.
        let two = format!("test result: ok. 0 passed; 1 filtered out\n{one}");
        assert_eq!(passed_counts(&two), vec![0, 17]);
        assert_eq!(classify(label, 0, &two), Ok(()));
        // Lines without the `test result:` prefix are not counted, whatever else they claim.
        assert!(passed_counts("all 9 passed; nothing to see\n").is_empty());
    }

    /// `$*` loses the quoting around `--features "doc mission"`, and the failure text must too.
    /// The pin table is also the gate's whole scope, and a silently shortened one is a silently
    /// weakened gate: 25 rows and 5 section `ok` lines, exactly as the script had.
    #[test]
    fn the_argv_rendering_and_the_pin_table_match_the_script() {
        let args = ["test", "-p", MEC, "--features", "doc mission", "--lib", "x"];
        let want = "-p map-engine-core --features doc mission --lib x";
        assert_eq!(shown(&args), want);
        assert_eq!(CARGO_PINS.len(), 25);
        assert_eq!(CARGO_PINS.iter().filter(|p| p.4.is_some()).count(), 5);
        // T-482: the vehicle-floor pin must keep `doc`, or it matches zero tests.
        let veh = CARGO_PINS
            .iter()
            .find(|p| p.3 == "the_vehicle_row_still_has_the_shape_this_module_reads");
        assert_eq!(veh.expect("the vehicle-floor pin is still listed").1, DOCM);
        // T-216 §2: no pin may ask for `doc` without `mission`.
        assert!(!CARGO_PINS.iter().any(|p| p.1 == Some("doc")));
    }

    /// `2>&1` is one pipe, not two strings glued together — the interleaving is the contract. The
    /// large case runs well past a 64 KiB pipe buffer on both streams at once: the six
    /// website-frontend pins each replay ~110 lines of warnings, so a wedged capture is not
    /// hypothetical. And bash reported an absent cargo as `exited 127` and a SIGKILLed one as
    /// `exited 137`; neither is an exit code, and neither may reach a caller as a plain failure.
    #[test]
    fn merged_capture_keeps_order_and_never_invents_an_exit_code() {
        let tmp = Path::new("/tmp");
        let path = std::env::var("PATH").unwrap_or_default();
        // T-853: the local `merged()` this used to exercise now lives in the library as
        // `Run::merged_output`, so a second cargo-running port inherits it instead of re-deriving
        // it. The assertions stay here because THIS gate is the one whose 803-line diff depends
        // on them.
        let sh = |script: &str| {
            tbd_gate::proc::Run::new("sh")
                .arg("-c")
                .arg(script)
                .cwd(tmp)
                .env("PATH", &path)
                .merged_output()
        };
        let m = sh("echo one; echo two >&2; echo three; exit 7").unwrap();
        assert_eq!(m.code, 7, "raw exit codes must never be collapsed");
        assert_eq!(m.text, "one\ntwo\nthree\n");
        let big = sh("seq 1 40000; seq 1 40000 >&2").unwrap();
        assert_eq!(big.code, 0);
        assert_eq!(big.text.lines().count(), 80000);
        let absent = tbd_gate::proc::Run::new("tbd-not-a-real-program-t180")
            .cwd(tmp)
            .env("PATH", "")
            .merged_output();
        assert!(matches!(absent, Err(NotRun::ToolAbsent(_))));
        let clause = not_run_clause(&absent.unwrap_err());
        assert!(clause.contains("is ABSENT"), "{clause}");
        assert!(clause.ends_with("did not execute."), "{clause}");
        match sh("kill -9 $$") {
            Err(NotRun::Signalled { signal, .. }) => assert_eq!(signal, 9),
            other => panic!("expected Signalled, got {other:?}"),
        }
    }
}
