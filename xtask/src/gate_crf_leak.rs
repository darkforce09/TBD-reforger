//! T-181.4/.52 oracle-leak guard — the T-853 port of `scripts/mod/verify-no-crf-leak.sh`.
//!
//! ── THIS GATE SHIPS **RED**, AND THAT IS THE CORRECT STATE ───────────────────────────────────
//!
//! On the tree as committed this exits **1** and prints, under the CRF GUID arm:
//!
//! ```text
//! FAIL: CRF-only asset GUIDs reused (not present in vanilla):
//!   {41174B59DA65659A}
//!   {8A239DEA19509B1B}
//! ```
//!
//! Both are referenced from `apps/mod/tbd-framework/Data/registry.json` (lines 1367 and 1747,
//! measured 2026-08-12) — a Radio virtual-arsenal slot and a Soviet grenade bandolier. Both also
//! appear in `apps/mod/crf_framework`'s own `.layout`/`.et` files, and neither is in any vanilla
//! `.pak`, so the "engine fact" exemption below does not cover them.
//!
//! **That is a real Arma Public License finding for a human to resolve, not a defect in this
//! port.** Do not exempt them, do not allow-list them, do not soften the arm to make CI green: the
//! port is accepted by diffing its stdout against the script's on this very failure, so a green
//! `xtask verify no-crf-leak` would mean the port is broken, not that the licence problem went
//! away. Resolving it is a licence decision — re-author the two prefab references from vanilla, or
//! record an attribution — and it belongs to whoever owns the mod's APL posture.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Third-party frameworks live on disk as READ-ONLY oracles, and hundreds of files of working code
//! next to a thinner implementation makes copy-paste the path of least resistance — so the script
//! makes the leak a build failure: no `<PREFIX>` identifier in `apps/mod/tbd-framework/**` outside
//! comments, and no GUID an oracle declares in its own `UI/`/`Prefabs/` assets reused in ours.
//! `crf_framework` is Arma Public License — attribution-bearing, but read-never-copy for us: we
//! design-mirror and cite, we do not vendor. `playable_selector` has **NO LICENCE AT ALL**, which
//! is strictly *worse* than APL: with no grant, default copyright applies and there is no
//! permission to copy, adapt or redistribute any of it. The command keeps the too-narrow name
//! `no-crf-leak` because `wave.sh`, the `Makefile`, `SLICE_WORKFLOW.md` and
//! `t181_event_mod_program.md` invoke it by that name; renaming drops it out of the wave runner.
//!
//! ── BASH ODDITIES PRESERVED ON PURPOSE ───────────────────────────────────────────────────────
//!
//! 1. **`find -L` is load-bearing.** In a slice worktree every oracle lane is a SYMLINK, and a
//!    bare `find <symlink>` does not descend — it reports the link, which is not `-type d`, so the
//!    search returns nothing. Measured: the gate then printed a cheerful "nothing to compare" for
//!    CRF while the real comparison never ran. [`asset_dirs`] follows links at every level.
//! 2. **The identifier pattern is anchored on a non-identifier char**, `(^|[^A-Za-z0-9_])`, so a
//!    short prefix cannot false-positive on a longer word — load-bearing for `PS_`, which a bare
//!    `grep` also finds inside `MAPS_`, `GROUPS_`, `OPS_` and `TIPS_`.
//! 3. **Comment-only lines are stripped before judging**: citing the oracle you design-mirrored is
//!    the practice we want. The filter runs over the *rendered* `path:line:text` — [`COMMENT_RE`].
//! 4. **A shared GUID present in a vanilla `.pak` is an ENGINE FACT, not a leak** — measured, all
//!    4 initial CRF hits were vanilla, and it is why only 2 of the 74 CRF-shared and 0 of the 18
//!    PS-shared GUIDs are reported.
//! 5. **`head -20`** on the hit list; `--exclude-dir=EnfusionMCP` and
//!    `--binary-files=without-match` on the identifier scan only — the GUID scan has neither.
//! 6. **Both SKIP wordings are deliberately NOT "OK"**, verbatim. Reaching the second means
//!    nothing was compared, which is how the symlink bug above hid itself.
//! 7. **No Steam install ⇒ every shared GUID is reported**, because `[ -d "$game" ] && grep …`
//!    short-circuits false. A false accusation in a fail-closed costume, but changing it changes
//!    what the gate prints on most CI runners: argue that separately, never inside a port.
//!
//! ── DELIBERATE DEVIATIONS, ALL UNREACHABLE ON THE LIVE TREE ──────────────────────────────────
//!
//! * **Hit order is sorted, not `readdir` order** — `grep -r`'s fts order is measured to be
//!   neither sorted nor stable across filesystems, so the script's own ordering is not
//!   reproducible. [`scan::walk_files`] sorts. Moot: both identifier arms are `OK (none)`.
//! * **A missing `tbd-framework`, or an absent `grep`, is exit 2 — not a green run.** In bash,
//!   `grep -rn … 2>/dev/null || true` over an absent tree prints `OK (none)` *and* `OK (nothing to
//!   compare)` and exits 0: the fail-open defect `tbd-gate` exists to remove. Here, a [`NotRun`].
//! * **GNU grep's binary heuristic is approximated** by [`grep_visible`]; measured 2026-08-12,
//!   `tbd-framework` holds one binary file (`resourceDatabase.rdb`, NUL at byte 4) with zero
//!   matches of either pattern. grep also suppresses lines carrying encoding errors; no such text
//!   file exists in either tree, so that branch is left out rather than implemented wrong.
//!
//! Runtime is ~7m25s, almost all vanilla probe: a GUID that is a genuine miss reads all ~20 GB of
//! `data0*.pak`. No timeout — bash had none, and a deadline would turn a cold page cache into a
//! leak report. (`grep` is `/usr/bin/grep` 3.8; this shell's `ugrep` shim is a shell *function*,
//! so neither a `bash` script nor [`Run`] ever sees it — gate-grep.sh's `rg` finding again.)

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use tbd_gate::proc::Run;
use tbd_gate::{Kind, NotRun, Pattern, Verdict, scan};

// The script's `MOD` and `CRF`. `crf_framework` is gitignored, so it is absent on a fresh clone —
// which is what the advisory SKIP is for.
const MOD_REL: &str = "apps/mod/tbd-framework";
const CRF_REL: &str = "apps/mod/crf_framework";
/// bash line 39: the in-repo lane (a slice worktree's symlink) **overrides even `TBD_PS_ORACLE`**,
/// because that assignment is unconditional in the script. `${VAR:-…}`, so *empty* falls back too;
/// the fallback is the operator's own checkout, which no repo script provisions.
const PS_REPO_REL: &str = "apps/mod/playable_selector";
const PS_ENV: &str = "TBD_PS_ORACLE";
const PS_HOME_REL: &str = "Projects/Archive/Reforger_Lobby/PlayableSelector-main";
/// Where the vanilla `.pak` files live, relative to `$HOME`.
const VANILLA_HOME_REL: &str = ".local/share/Steam/steamapps/common/Arma Reforger/addons/data";
/// Injected dev-only tooling, gitignored; excluded from the identifier scan, not the GUID scan.
const EXCLUDE_DIR: &str = "EnfusionMCP";
/// Found rather than hardcoded because the lanes nest differently: `crf_framework/UI` vs
/// `PlayableSelector-main/PlayableSelector/UI`.
const ASSET_DIR_NAMES: &[&str] = &["UI", "Prefabs"];
/// bash `head -20`; GNU grep's initial read buffer, the window its up-front binary test looks at;
/// and the Enfusion asset GUID, uppercase hex only, exactly as the script spells it.
const HEAD: usize = 20;
const GREP_BUF: usize = 32 * 1024;
const GUID_RE: &str = r"\{[0-9A-F]{16}\}";
/// bash's `grep -vE` comment filter, applied to the rendered `path:line:text`.
///
/// `[^:]+` for the path is the script's, warts and all: a source file with a colon in its NAME
/// fails to match, so its comment lines would be reported as leaks. Zero such files exist
/// (measured 2026-08-12); reproduced rather than quietly repaired.
const COMMENT_RE: &str = r"^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)";
/// Tail of the advisory SKIP, and the epilogue line whose padding is the script's. Both hoisted
/// only so their call sites fit the line budget; the wording is the script's, verbatim.
const SKIP_TAIL: &str =
    "not present locally (gitignored / out-of-repo); GUID check is advisory here";
const EPILOGUE_PS: &str =
    "  PlayableSelector — NO LICENCE; default copyright, so no permission to copy at all.";
/// The identifier arm's two calls, in the script's order. The label is prose for the banner.
const IDENT_LANES: &[(&str, &str)] = &[
    ("CRF, Arma Public License", "CRF_"),
    ("PlayableSelector, NO LICENCE", "PS_"),
];

/// The four paths the script resolves before it checks anything.
///
/// Split out from [`verify_crf_leak`] so the tests can point every lane at a fixture without
/// mutating `HOME` — `std::env::set_var` is `unsafe` in edition 2024 and races other test threads.
struct Lanes {
    mod_dir: PathBuf,
    crf: PathBuf,
    ps: PathBuf,
    vanilla: PathBuf,
}

impl Lanes {
    fn from_env(repo_root: &Path) -> Lanes {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        // `PS_SRC="${TBD_PS_ORACLE:-$HOME/…}"`, then an unconditional in-repo override.
        let mut ps = match std::env::var_os(PS_ENV) {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => home.join(PS_HOME_REL),
        };
        if repo_root.join(PS_REPO_REL).is_dir() {
            ps = repo_root.join(PS_REPO_REL);
        }
        Lanes {
            mod_dir: repo_root.join(MOD_REL),
            crf: repo_root.join(CRF_REL),
            ps,
            vanilla: home.join(VANILLA_HOME_REL),
        }
    }
}

/// Every line the gate prints, streamed *and* retained.
///
/// The script's stdout is a contract — `wave.sh` scrapes it and T-853 accepts ports by diffing it
/// — so the tests assert exact text rather than a boolean. Retaining is what makes that possible;
/// streaming is what stops a 7-minute run looking hung.
struct Log {
    lines: Vec<String>,
    echo: bool,
}

impl Log {
    fn say(&mut self, line: impl Into<String>) {
        let line = line.into();
        if self.echo {
            println!("{line}");
        }
        self.lines.push(line);
    }
}

pub fn verify_crf_leak(repo_root: &Path) -> Result<u8> {
    let mut log = Log {
        lines: Vec::new(),
        echo: true,
    };
    Ok(run(&Lanes::from_env(repo_root), &mut log))
}

/// The script body: four checks, then the epilogue-and-exit-1 or the PASS line.
fn run(lanes: &Lanes, log: &mut Log) -> u8 {
    let mut fail = false;
    // One vanilla answer per bare GUID for the whole run. The two lanes share 13 GUIDs (measured)
    // and bash re-greps each; the paks cannot change mid-run, so memoising is provably the same
    // answer for a fraction of the I/O — and a *miss* costs 20 GB.
    let mut memo: HashMap<String, bool> = HashMap::new();

    for (label, prefix) in IDENT_LANES {
        match check_identifier_leak(log, &lanes.mod_dir, label, prefix) {
            Ok(hit) => fail |= hit,
            Err(cause) => return refuse(log, cause),
        }
    }
    for (label, oracle) in [("CRF", &lanes.crf), ("PlayableSelector", &lanes.ps)] {
        match check_guid_leak(log, lanes, label, oracle, &mut memo) {
            Ok(hit) => fail |= hit,
            Err(cause) => return refuse(log, cause),
        }
    }

    if fail {
        log.say("");
        log.say("Oracles are reference-only. Design-mirror them; never copy them.");
        log.say("  CRF              — Arma Public License; read, cite, do not vendor.");
        log.say(EPILOGUE_PS);
        log.say("See docs/mod/TBD_MOD_DESIGN.md §2 and docs/mod/SLICE_WORKFLOW.md §Oracle lanes.");
        return 1;
    }
    log.say("no-oracle-leak: PASS (CRF + PlayableSelector)");
    0
}

/// Exit **2**, not the script's 1: "the tree is dirty" and "I never read the tree" are different
/// operator actions. `wave.sh` tests `rc -eq 0`, so any nonzero is still FAIL there.
fn refuse(log: &mut Log, cause: NotRun) -> u8 {
    let msg = "no-oracle-leak could not examine the trees it was pointed at";
    log.say(Verdict::did_not_run(msg, Kind::Ban, cause).to_string());
    2
}

/* ───────────────────── arm 1: <prefix> identifiers in our own code ───────────────────── */

fn check_identifier_leak(
    log: &mut Log,
    mod_dir: &Path,
    label: &str,
    prefix: &str,
) -> Result<bool, NotRun> {
    log.say(format!(
        "==> {prefix} identifiers in tbd-framework code ({label})"
    ));
    let ident = pattern(&format!("(^|[^A-Za-z0-9_]){prefix}"))?;
    let comment = pattern(COMMENT_RE)?;

    let mut hits: Vec<String> = Vec::new();
    for file in scan::walk_files(&[mod_dir], outside_excluded_dir)? {
        let bytes = read(&file)?;
        for (line_no, line) in numbered(grep_visible(&bytes)) {
            if !ident.is_match(&line) {
                continue;
            }
            // Build `grep -rn`'s exact rendering first: the comment filter is anchored on it, not
            // on the source line, and `$MOD` was absolute so these paths are absolute too.
            let rendered = format!("{}:{line_no}:{line}", file.display());
            if !comment.is_match(&rendered) {
                hits.push(rendered);
            }
        }
    }

    if hits.is_empty() {
        log.say("  OK (none)");
        return Ok(false);
    }
    log.say(format!(
        "FAIL: {prefix} symbols found in the production mod:"
    ));
    for hit in hits.iter().take(HEAD) {
        log.say(hit.clone());
    }
    Ok(true)
}

/// `--exclude-dir=EnfusionMCP`. grep prunes the directory; [`scan::walk_files`] filters files, so
/// we still descend into it and discard — same output, a few stat calls more.
fn outside_excluded_dir(path: &Path) -> bool {
    !path.components().any(|c| c.as_os_str() == EXCLUDE_DIR)
}

/* ───────────────────── arm 2: oracle-declared asset GUIDs ───────────────────── */

fn check_guid_leak(
    log: &mut Log,
    lanes: &Lanes,
    label: &str,
    oracle: &Path,
    memo: &mut HashMap<String, bool>,
) -> Result<bool, NotRun> {
    log.say(format!(
        "==> {label} layout/prefab GUIDs reused in tbd-framework"
    ));
    // `[ -d ]` follows symlinks, and so does `is_dir`.
    if !oracle.is_dir() {
        log.say(format!("  SKIP — {label} {SKIP_TAIL}"));
        return Ok(false);
    }
    let dirs = asset_dirs(oracle)?;
    if dirs.is_empty() {
        // Deliberately NOT worded as OK: reaching here means we compared nothing, which is how the
        // `find -L` symlink bug hid itself.
        log.say(format!(
            "  SKIP — no UI/ or Prefabs/ dirs under {}; NO GUID comparison was made",
            oracle.display()
        ));
        return Ok(false);
    }

    let guid = Regex::new(GUID_RE).map_err(|e| broken_pattern(GUID_RE, e))?;
    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    let oracle_guids = guids_under(&guid, &refs)?;
    // Recomputed per lane, as in the script. 143 files; the repeat costs nothing.
    let ours = guids_under(&guid, &[lanes.mod_dir.as_path()])?;
    if oracle_guids.is_empty() || ours.is_empty() {
        log.say("  OK (nothing to compare)");
        return Ok(false);
    }

    // bash `comm -12` over two `sort -u` streams. A `BTreeSet` intersection is byte order, which
    // for these fixed-shape `{16 uppercase hex}` strings is exactly what en_AU.UTF-8 collation
    // produces — verified against the script's own output ordering.
    let mut leaks: Vec<&String> = Vec::new();
    for g in oracle_guids.intersection(&ours) {
        if in_vanilla(&lanes.vanilla, &g.replace(['{', '}'], ""), memo)? {
            continue; // present in vanilla -> engine fact, not an oracle leak
        }
        leaks.push(g);
    }

    if leaks.is_empty() {
        log.say("  OK (shared GUIDs are all vanilla engine facts)");
        return Ok(false);
    }
    log.say(format!(
        "FAIL: {label}-only asset GUIDs reused (not present in vanilla):"
    ));
    for g in &leaks {
        log.say(format!("  {g}"));
    }
    Ok(true)
}

/// `find -L "$oracle" -maxdepth 2 -type d \( -name UI -o -name Prefabs \)`.
///
/// Depth 0 is included because `find` includes the start point. Symlinks are followed at every
/// level — see the module docs; `maxdepth 2` is also what bounds a symlink cycle.
fn asset_dirs(oracle: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let mut out = Vec::new();
    if is_asset_dir(oracle) {
        out.push(oracle.to_path_buf());
    }
    for one in children(oracle)? {
        if !one.is_dir() {
            continue;
        }
        if is_asset_dir(&one) {
            out.push(one.clone());
        }
        for two in children(&one)? {
            if is_asset_dir(&two) {
                out.push(two);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn is_asset_dir(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| ASSET_DIR_NAMES.contains(&n));
    named && path.is_dir()
}

/// One directory level. `find` would print a suppressed error and carry on; refusing instead is
/// the anti-fail-open choice — a lane we could not read must not report "nothing to compare".
fn children(dir: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let bad = |source: std::io::Error| NotRun::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(bad)? {
        out.push(entry.map_err(bad)?.path());
    }
    Ok(out)
}

/// `grep -rhoE '\{[0-9A-F]{16}\}' … | sort -u`.
///
/// `Regex` directly rather than [`Pattern`], which exposes only `is_match` while `-o` needs every
/// match. Scanning whole text instead of line by line is identical here: the pattern holds no `.`
/// and no newline, so a match can never span a line break.
fn guids_under(guid: &Regex, roots: &[&Path]) -> Result<BTreeSet<String>, NotRun> {
    let mut out = BTreeSet::new();
    for file in scan::walk_files(roots, |_| true)? {
        let bytes = read(&file)?;
        for m in guid.find_iter(&String::from_utf8_lossy(grep_visible(&bytes))) {
            out.insert(m.as_str().to_string());
        }
    }
    Ok(out)
}

/// `[ -d "$game" ] && grep -qla "$bare" "$game"/*.pak 2>/dev/null`.
///
/// Every non-zero status is "not in vanilla", exactly as the `&&` chain read it: 1 is a clean
/// miss, 2 is grep erroring on an unexpanded glob. A tool that never ran is *not* folded in — it
/// propagates as [`NotRun`] and exits 2, where bash would have called 127 a leak, 92 times over.
fn in_vanilla(dir: &Path, bare: &str, memo: &mut HashMap<String, bool>) -> Result<bool, NotRun> {
    if let Some(hit) = memo.get(bare) {
        return Ok(*hit);
    }
    let paks = paks(dir);
    let hit = if paks.is_empty() {
        false // No Steam install, or a glob that matched nothing. Module docs oddity 7.
    } else {
        let probe = Run::new("grep").arg("-qla").arg(bare).args(&paks);
        probe.status()? == 0
    };
    memo.insert(bare.to_string(), hit);
    Ok(hit)
}

/// The shell glob `"$game"/*.pak`: sorted, dotfiles excluded, no type test — a directory named
/// `*.pak` would be handed to grep here exactly as bash hands it over.
fn paks(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let glob = |n: &str| n.ends_with(".pak") && !n.starts_with('.');
    let named = |p: &PathBuf| p.file_name().and_then(|n| n.to_str()).is_some_and(glob);
    let mut out: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(named).collect();
    out.sort();
    out
}

/* ───────────────────── grep-compatible reading ───────────────────── */

fn read(path: &Path) -> Result<Vec<u8>, NotRun> {
    std::fs::read(path).map_err(|source| NotRun::Unreadable {
        path: path.to_path_buf(),
        source,
    })
}

/// The bytes GNU grep 3.8 would actually produce output from.
///
/// Measured against `/usr/bin/grep` 3.8: a NUL inside the first read buffer flags the whole file
/// binary and it emits nothing at all; a NUL that arrives later lets the matches *before* it print
/// and swallows the rest. Both `--binary-files=without-match` (arm 1) and the default mode with
/// `-o` (arm 2) behave this way on stdout, which is all the script captures — it pipes stdout and
/// ends with `|| true`, so grep's exit status never reaches a decision.
fn grep_visible(bytes: &[u8]) -> &[u8] {
    if bytes[..bytes.len().min(GREP_BUF)].contains(&0) {
        return &[];
    }
    match bytes.iter().position(|b| *b == 0) {
        Some(cut) => &bytes[..cut],
        None => bytes,
    }
}

/// `grep -n`'s numbering: 1-based, split on `\n` only.
///
/// Not [`scan::grep_lines`], which uses `str::lines` and therefore **strips a trailing `\r`**.
/// grep keeps it, and `tbd-framework` does hold a CRLF file the Workbench MCP bridge wrote, so the
/// difference is one commit away from being observable.
fn numbered(bytes: &[u8]) -> Vec<(usize, String)> {
    let mut lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    // `split` yields a trailing empty piece for a file that ends in a newline; grep does not
    // count that as a line. A file NOT ending in one still has its last partial line counted.
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let n = |(i, l): (usize, &&[u8])| (i + 1, String::from_utf8_lossy(l).into_owned());
    lines.iter().enumerate().map(n).collect()
}

/// A pattern constant that will not compile is a bug in THIS file; it must not read as "no hits".
fn pattern(src: &str) -> Result<Pattern, NotRun> {
    Pattern::regex(src).map_err(|e| broken_pattern(src, e))
}

fn broken_pattern(src: &str, e: regex::Error) -> NotRun {
    NotRun::ToolError {
        tool: "regex".into(),
        status: 1,
        stderr: format!("{src}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "{AAAAAAAAAAAAAAAA}";
    const B: &str = "{BBBBBBBBBBBBBBBB}";
    const C: &str = "{0123456789ABCDEF}";

    /// A throwaway four-lane tree (`mod`/`crf`/`ps`/`vanilla`). Never inside the repo: a fixture
    /// under `apps/mod/` would be scanned by the gate it is testing.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str, files: &[(&str, &str)]) -> Fixture {
            let root = std::env::temp_dir().join(format!("tbd-crf-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("mod")).unwrap();
            for (rel, body) in files {
                let p = root.join(rel);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, body).unwrap();
            }
            Fixture(root)
        }
        fn run(&self) -> (u8, String) {
            gate(&self.0)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Exit code plus the exact stdout the script would have produced, for a four-lane root.
    fn gate(root: &Path) -> (u8, String) {
        let at = |s: &str| root.join(s);
        let lanes = Lanes {
            mod_dir: at("mod"),
            crf: at("crf"),
            ps: at("ps"),
            vanilla: at("vanilla"),
        };
        let mut log = Log {
            lines: Vec::new(),
            echo: false,
        };
        (run(&lanes, &mut log), log.lines.join("\n"))
    }

    // Every assertion here is "the gate printed exactly this", so the transcript is the only
    // useful failure message. Two helpers, so a one-line check stays a one-line check.
    fn has(out: &str, want: &str) {
        assert!(out.contains(want), "MISSING {want:?} in:\n{out}");
    }
    fn hasnt(out: &str, bad: &str) {
        assert!(!out.contains(bad), "UNEXPECTED {bad:?} in:\n{out}");
    }

    /// THE SKIP CONTRACT. An advisory skip, a comparison that never happened, and a comparison
    /// that ran and found nothing are three different facts; neither SKIP may soften to an "OK".
    #[test]
    fn the_three_no_finding_wordings_stay_distinct() {
        // Both oracles absent -> two advisory SKIPs, and the run is still a PASS.
        let (code, out) = Fixture::new("skip", &[("mod/Data/r.json", C)]).run();
        assert_eq!(code, 0, "{out}");
        assert_eq!(out.matches("  OK (none)").count(), 2, "{out}");
        has(&out, &format!("  SKIP — CRF {SKIP_TAIL}"));
        has(&out, &format!("  SKIP — PlayableSelector {SKIP_TAIL}"));
        hasnt(&out, "OK (shared GUIDs"); // never claim a comparison that did not happen
        hasnt(&out, "OK (nothing to compare)");
        has(&out, "no-oracle-leak: PASS (CRF + PlayableSelector)");

        // The lane EXISTS but holds no UI/ or Prefabs/. This is the wording that caught the
        // `find -L` symlink bug, so it must stay blunt rather than cheerful.
        let files = [("mod/Data/r.json", C), ("crf/S/W.c", "// no assets\n")];
        let fx2 = Fixture::new("nodirs", &files);
        let (_, out2) = fx2.run();
        let at = fx2.0.join("crf");
        let blunt = "  SKIP — no UI/ or Prefabs/ dirs under";
        let made = "NO GUID comparison was made";
        has(&out2, &format!("{blunt} {}; {made}", at.display()));
        hasnt(&out2, "OK (nothing to compare)");

        // The oracle has assets and we have none: the comparison ran and found nothing.
        let files = [("mod/S/T.c", "class T {}\n"), ("crf/UI/M.layout", A)];
        let (code3, out3) = Fixture::new("empty", &files).run();
        assert_eq!(code3, 0, "{out3}");
        has(&out3, "  OK (nothing to compare)");
    }

    /// A real leak, and the two ways the script refuses to call one.
    #[test]
    fn identifier_leaks_are_caught_but_comments_and_longer_words_are_not() {
        // Lines 1-4 are citations — the practice the gate exists to encourage. Line 5 is the leak.
        // Maps.c holds the four longer words a bare `grep PS_` would false-hit.
        let leaky = "//! CRF_PlayerCharacter.DisableAI port: mirrored, not copied\n\
                     /* CRF_Whatever in a block comment */\n\
                     * CRF_Whatever in a doc continuation\n\
                     # CRF_Whatever in a hash comment\n\
                     class TBD_SpawnManager { void go() { CRF_Registry.Get(); } }\n";
        let words = "int MAPS_C, GROUPS_M, OPS_T, TIPS_S;\n";
        let files = [("mod/S/Spawn.c", leaky), ("mod/S/Maps.c", words)];
        let (code, out) = Fixture::new("ident", &files).run();
        assert_eq!(code, 1, "{out}");
        has(&out, "FAIL: CRF_ symbols found in the production mod:");
        has(&out, "Spawn.c:5:class TBD_SpawnManager");
        for commented in [":1:", ":2:", ":3:", ":4:"] {
            hasnt(&out, commented); // a comment naming the oracle is allowed
        }
        // PS_ must not hit MAPS_/GROUPS_/OPS_/TIPS_.
        let ps_arm = "==> PS_ identifiers in tbd-framework code (PlayableSelector, NO LICENCE)";
        has(&out, &format!("{ps_arm}\n  OK (none)"));
        has(&out, "Oracles are reference-only.");
        has(&out, EPILOGUE_PS);
    }

    /// `head -20` truncation, and `--exclude-dir=EnfusionMCP` applying to arm 1 only.
    #[test]
    fn the_hit_list_truncates_and_the_mcp_bridge_is_exempt_from_arm_one() {
        let body: String = (0..25).map(|i| format!("x = CRF_X{i}();\n")).collect();
        let (code, out) = Fixture::new("head", &[("mod/S/Many.c", &body)]).run();
        assert_eq!(code, 1);
        assert_eq!(out.matches("Many.c:").count(), HEAD, "{out}");

        let emcp = "mod/S/WorkbenchGame/EnfusionMCP/EMCP.c";
        let mcp = Fixture::new("mcp", &[(emcp, "CRF_Thing t;\n{0123456789ABCDEF}\n")]);
        let (mcp_code, mcp_out) = mcp.run();
        assert_eq!(mcp_code, 0, "{mcp_out}");
        assert_eq!(mcp_out.matches("  OK (none)").count(), 2, "{mcp_out}");
        let re = Regex::new(GUID_RE).unwrap();
        let ours = guids_under(&re, &[mcp.0.join("mod").as_path()]).unwrap();
        assert!(ours.contains(C), "the GUID scan has no --exclude-dir");
    }

    /// The engine-fact filter over a real `grep -qla` subprocess: a GUID inside a `.pak` is
    /// vanilla and exempt, one that is not is the leak. This is the arm that ships red.
    #[test]
    fn vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid() {
        // The "pak" is read with `grep -a`, so a bare hex run is all that matters.
        let both = "{AAAAAAAAAAAAAAAA}\n{BBBBBBBBBBBBBBBB}\n";
        let files = [
            ("mod/Data/r.json", both),
            ("crf/UI/M.layout", both),
            ("vanilla/data001.pak", "\u{0}junk AAAAAAAAAAAAAAAA more\n"),
        ];
        let (code, out) = Fixture::new("vanilla", &files).run();
        assert_eq!(code, 1, "{out}");
        let head = "FAIL: CRF-only asset GUIDs reused (not present in vanilla):";
        has(&out, &format!("{head}\n  {B}"));
        hasnt(&out, &format!("  {A}")); // in vanilla -> an engine fact, not a leak

        // Module docs oddity 7: no Steam install, so the `&&` short-circuits and EVERY shared GUID
        // is reported. Pinned so the false-accusation behaviour cannot drift unnoticed.
        let bare = [("mod/Data/r.json", A), ("crf/Prefabs/P.et", A)];
        let (bare_code, bare_out) = Fixture::new("nosteam", &bare).run();
        assert_eq!(bare_code, 1, "{bare_out}");
        has(&bare_out, &format!("  {A}"));
    }

    /// `find -L` is load-bearing: in a slice worktree the oracle lane is a symlink, and a walker
    /// that does not descend prints the blunt SKIP while comparing nothing.
    #[test]
    fn asset_dirs_descend_through_symlinked_lanes() {
        // CRF nests UI at depth 1, PlayableSelector at depth 2 — why the script uses `find`.
        let files = [
            ("real/PlayableSelector/UI/M.layout", A),
            ("real/PlayableSelector/Prefabs/P.et", B),
            ("crf/UI/A.layout", C),
        ];
        let fx = Fixture::new("symlink", &files);
        std::os::unix::fs::symlink(fx.0.join("real"), fx.0.join("ps")).unwrap();
        let dirs = asset_dirs(&fx.0.join("ps")).unwrap();
        assert_eq!(dirs.len(), 2, "maxdepth 2 through the link: {dirs:?}");
        assert_eq!(asset_dirs(&fx.0.join("crf")).unwrap().len(), 1);
    }

    /// THE DEFECT THE CRATE EXISTS FOR. bash reads an absent mod tree as `OK (none)` + `OK
    /// (nothing to compare)` and exits 0; a tree nobody read is not a clean tree.
    #[test]
    fn an_absent_mod_tree_does_not_read_as_clean() {
        let (code, out) = gate(Path::new("/nonexistent/tbd-crf"));
        assert_eq!(code, 2, "{out}");
        has(&out, "FAIL: no-oracle-leak could not examine the trees");
        hasnt(&out, "OK (none)");
    }

    /// The grep-compatibility helpers and the lane resolution, measured against `/usr/bin/grep`.
    #[test]
    fn reading_and_lane_resolution_match_the_script() {
        assert_eq!(grep_visible(b"hi\0there"), b"", "NUL in the first buffer");
        let late = [b"a".repeat(GREP_BUF + 1), b"\0tail".to_vec()].concat();
        assert_eq!(grep_visible(&late).len(), GREP_BUF + 1, "a late NUL cuts");
        assert_eq!(grep_visible(b"plain\n"), b"plain\n");

        // grep keeps the CR; `str::lines` would eat it.
        let crlf = [(1, "a\r".to_string()), (2, "b\r".to_string())];
        assert_eq!(numbered(b"a\r\nb\r\n"), crlf);
        let partial = [(1, "one".to_string()), (2, "two".to_string())];
        assert_eq!(numbered(b"one\ntwo"), partial, "no trailing newline");
        assert!(numbered(b"").is_empty());

        // The comment filter is anchored on the RENDERED line, path included.
        let c = pattern(COMMENT_RE).unwrap();
        assert!(c.is_match("/a/b.c:5:  // CRF_X") && c.is_match("/a/b.c:5:\t* CRF_X"));
        assert!(c.is_match("/a/b.c:5:/* X") && c.is_match("/a/b.c:5:# X"));
        assert!(!c.is_match("/a/b.c:5:  CRF_X();"));
        assert!(!c.is_match("/a/o:d.c:5:  // X"), "`[^:]+` spans no colon");

        let ps = pattern("(^|[^A-Za-z0-9_])PS_").unwrap();
        for longer in ["MAPS_X", "GROUPS_X", "OPS_X", "TIPS_X", "xPS_y"] {
            assert!(!ps.is_match(longer), "{longer} is not a PS_ identifier");
        }
        for real in ["PS_Thing", " PS_Thing", "\tPS_Thing", "(PS_Thing)"] {
            assert!(ps.is_match(real), "{real} is a PS_ identifier");
        }

        // The in-repo lane wins even over TBD_PS_ORACLE — the override is unconditional.
        let fx = Fixture::new("lanes", &[]);
        std::fs::create_dir_all(fx.0.join(PS_REPO_REL)).unwrap();
        assert_eq!(Lanes::from_env(&fx.0).ps, fx.0.join(PS_REPO_REL));
        let bare = Fixture::new("lanes2", &[]);
        assert_ne!(Lanes::from_env(&bare.0).ps, bare.0.join(PS_REPO_REL));
        assert_eq!(Lanes::from_env(&bare.0).mod_dir, bare.0.join(MOD_REL));
    }
}
